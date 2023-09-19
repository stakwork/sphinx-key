use crate::conn::{Channel, ChannelRequest, LssReq};
use crate::util::Settings;
use bitcoin::blockdata::constants::ChainHash;
use log::*;
use rocket::tokio::sync::mpsc;
use secp256k1::PublicKey;
use sphinx_signer::{parser, sphinx_glyph::topics};
use std::sync::atomic::{AtomicU16, Ordering};
use std::thread;
use std::time::Duration;
use vls_protocol::msgs::SerBolt;
use vls_protocol::{msgs, msgs::Message, Error, Result};
use vls_proxy::client::Client;

static COUNTER: AtomicU16 = AtomicU16::new(0u16);
static CURRENT: AtomicU16 = AtomicU16::new(0u16);

pub fn take_a_ticket() -> u16 {
    COUNTER.fetch_add(1u16, Ordering::SeqCst)
}

pub fn is_my_turn(ticket: u16) -> bool {
    let curr = CURRENT.load(Ordering::SeqCst);
    curr == ticket
}

pub fn my_turn_is_done() {
    CURRENT.fetch_add(1u16, Ordering::SeqCst);
}

#[derive(Clone, Debug)]
pub struct ClientId {
    pub peer_id: PublicKey,
    pub dbid: u64,
}

impl Channel {
    pub fn new(sender: mpsc::Sender<ChannelRequest>) -> Self {
        Self {
            sender,
            pubkey: [0; 33], // init with empty pubkey
        }
    }
}

/// Implement the hsmd UNIX fd protocol.
pub struct SignerLoop<C: 'static + Client> {
    client: C,
    log_prefix: String,
    chan: Channel,
    client_id: Option<ClientId>,
    lss_tx: mpsc::Sender<LssReq>,
}

impl<C: 'static + Client> SignerLoop<C> {
    /// Create a loop for the root (lightningd) connection, but doesn't start it yet
    pub fn new(
        client: C,
        lss_tx: mpsc::Sender<LssReq>,
        sender: mpsc::Sender<ChannelRequest>,
    ) -> Self {
        let log_prefix = format!("{}/{}", std::process::id(), client.id());
        Self {
            client,
            log_prefix,
            chan: Channel::new(sender),
            client_id: None,
            lss_tx,
        }
    }

    // Create a loop for a non-root connection
    fn new_for_client(
        client: C,
        lss_tx: mpsc::Sender<LssReq>,
        sender: mpsc::Sender<ChannelRequest>,
        client_id: ClientId,
    ) -> Self {
        let log_prefix = format!("{}/{}", std::process::id(), client.id());
        Self {
            client,
            log_prefix,
            chan: Channel::new(sender),
            client_id: Some(client_id),
            lss_tx,
        }
    }

    /// Start the read loop
    pub fn start(&mut self, settings: Option<Settings>) {
        info!("loop {}: start", self.log_prefix);
        match self.do_loop(settings) {
            Ok(()) => info!("loop {}: done", self.log_prefix),
            Err(Error::Eof) => info!("loop {}: ending", self.log_prefix),
            Err(e) => error!("loop {}: error {:?}", self.log_prefix, e),
        }
    }

    fn do_loop(&mut self, settings: Option<Settings>) -> Result<()> {
        // This counter is only used in the root loop to periodically send heartbeats to the hardware signer
        let mut send_heartbeat = 0u8;
        loop {
            let raw_msg = self.client.read_raw()?;
            // debug!("loop {}: got raw", self.log_prefix);
            let msg = msgs::from_vec(raw_msg.clone())?;
            info!("loop {}: got {:x?}", self.log_prefix, msg);
            match msg {
                Message::ClientHsmFd(m) => {
                    self.client.write(msgs::ClientHsmFdReply {}).unwrap();
                    let new_client = self.client.new_client();
                    info!("new client {} -> {}", self.log_prefix, new_client.id());
                    let peer_id = PublicKey::from_slice(&m.peer_id.0).expect("client pubkey"); // we don't expect a bad key from lightningd parent
                    let client_id = ClientId {
                        peer_id,
                        dbid: m.dbid,
                    };
                    let mut new_loop = SignerLoop::new_for_client(
                        new_client,
                        self.lss_tx.clone(),
                        self.chan.sender.clone(),
                        client_id,
                    );
                    thread::spawn(move || new_loop.start(None));
                }
                Message::Memleak(_) => {
                    // info!("Memleak");
                    let reply = msgs::MemleakReply { result: false };
                    self.client.write(reply)?;
                }
                msg => {
                    let mut catch_init = false;
                    if let Message::HsmdInit(m) = msg {
                        catch_init = true;
                        if let Some(set) = settings {
                            if ChainHash::using_genesis_block(set.network).as_bytes()
                                != m.chain_params.as_ref()
                            {
                                panic!("The network settings of CLN and broker don't match!");
                            }
                        } else {
                            panic!("Got HsmdInit without settings - likely because HsmdInit was sent after startup");
                        }
                    }
                    let reply = self.handle_message(raw_msg, catch_init)?;
                    // Write the reply to CLN
                    self.client.write_vec(reply)?;
                    // Only send heartbeat messages from the root loop, as roothandler alone can process them, not channelhandler
                    // Send it every ten messages to prune extraneous data on hardware signer
                    if self.client_id.is_none() && send_heartbeat % 10u8 == 0u8 {
                        let beat = msgs::GetHeartbeat {};
                        let _ = self.handle_message(beat.as_vec(), false)?;
                    }
                }
            }
            send_heartbeat = send_heartbeat.wrapping_add(1u8);
        }
    }

    fn handle_message(&mut self, message: Vec<u8>, catch_init: bool) -> Result<Vec<u8>> {
        // wait until not busy
        let ticket = take_a_ticket();
        loop {
            if is_my_turn(ticket) {
                break;
            } else {
                thread::sleep(Duration::from_millis(5));
            }
        }

        let dbid = self.client_id.as_ref().map(|c| c.dbid).unwrap_or(0);
        let peer_id = self
            .client_id
            .as_ref()
            .map(|c| c.peer_id.serialize())
            .unwrap_or([0u8; 33]);
        let md = parser::raw_request_from_bytes(message, ticket, peer_id, dbid)?;
        // send to signer
        log::info!("SEND ON {}", topics::VLS);
        let (res_topic, res) = self.send_request_wait(topics::VLS, md)?;
        log::info!("GOT ON {}", res_topic);
        let the_res = if res_topic == topics::LSS_RES {
            // send reply to LSS to store muts
            let lss_reply = self.send_lss(topics::LSS_MSG.to_string(), res)?;
            log::info!("LSS REPLY LEN {}", &lss_reply.1.len());
            // send to signer for HMAC validation, and get final reply
            log::info!("SEND ON {}", lss_reply.0);
            let (res_topic2, res2) = self.send_request_wait(&lss_reply.0, lss_reply.1)?;
            log::info!("GOT ON {}, send to CLN?", res_topic2);
            if res_topic2 != topics::VLS_RES {
                log::warn!("got a topic NOT on {}", topics::VLS_RES);
            }
            res2
        } else {
            res
        };
        // create reply bytes for CLN
        let reply = parser::raw_response_from_bytes(the_res, ticket)?;

        // catch the pubkey if its the first one connection
        if catch_init {
            let _ = self.set_channel_pubkey(reply.clone());
        }

        // next turn
        my_turn_is_done();

        Ok(reply)
    }

    fn set_channel_pubkey(&mut self, raw_msg: Vec<u8>) -> Result<()> {
        let msg = msgs::from_vec(raw_msg.clone())?;
        let pk = match msg {
            Message::HsmdInitReplyV2(r) => Some(r.node_id.0),
            Message::HsmdInit2Reply(r) => Some(r.node_id.0),
            _ => None,
        };
        if let Some(pubkey) = pk {
            let pks = hex::encode(pubkey);
            log::info!("PUBKEY received from CLN: {}", pks);
            self.chan.pubkey = pubkey;
        }
        Ok(())
    }

    // returns (topic, payload)
    // might halt if signer is offline
    fn send_request_wait(&mut self, topic: &str, message: Vec<u8>) -> Result<(String, Vec<u8>)> {
        // Send a request to the MQTT handler to send to signer
        let (request, reply_rx) = ChannelRequest::new(topic, message);
        // This can fail if MQTT shuts down
        self.chan
            .sender
            .blocking_send(request)
            .map_err(|_| Error::Eof)?;
        let reply = reply_rx.blocking_recv().map_err(|_| Error::Eof)?;

        Ok((reply.topic_end, reply.reply))
    }

    fn send_lss(&mut self, topic: String, message: Vec<u8>) -> Result<(String, Vec<u8>)> {
        // Send a request to the LSS server
        let (request, reply_rx) = LssReq::new(topic, message);
        self.lss_tx.blocking_send(request).map_err(|_| Error::Eof)?;
        let res = reply_rx.blocking_recv().map_err(|_| Error::Eof)?;
        Ok(res)
    }
}
