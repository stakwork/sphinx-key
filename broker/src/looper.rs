use crate::conn::{Channel, ChannelRequest, LssReq};
use crate::util::Settings;
use bitcoin::blockdata::constants::ChainHash;
use log::*;
use rocket::tokio::sync::mpsc;
use secp256k1::PublicKey;
use sphinx_signer::{parser, sphinx_glyph::topics};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;
use vls_protocol::{msgs, msgs::Message, Error, Result};
use vls_proxy::client::Client;

#[derive(Clone, Debug)]
pub struct ClientId {
    pub peer_id: PublicKey,
    pub dbid: u64,
}

impl Channel {
    pub fn new(sender: mpsc::Sender<ChannelRequest>) -> Self {
        Self {
            sender,
            sequence: 0,
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
    busy: Arc<AtomicBool>,
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
            busy: Arc::new(AtomicBool::new(false)),
        }
    }

    // Create a loop for a non-root connection
    fn new_for_client(
        client: C,
        lss_tx: mpsc::Sender<LssReq>,
        sender: mpsc::Sender<ChannelRequest>,
        client_id: ClientId,
        busy: Arc<AtomicBool>,
    ) -> Self {
        let log_prefix = format!("{}/{}", std::process::id(), client.id());
        Self {
            client,
            log_prefix,
            chan: Channel::new(sender),
            client_id: Some(client_id),
            lss_tx,
            busy,
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
        loop {
            let raw_msg = self.client.read_raw()?;
            // debug!("loop {}: got raw", self.log_prefix);
            let msg = msgs::from_vec(raw_msg.clone())?;
            // info!("loop {}: got {:x?}", self.log_prefix, msg);
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
                        self.busy.clone(),
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
                                != &m.chain_params.0
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
                }
            }
        }
    }

    fn handle_message(&mut self, message: Vec<u8>, catch_init: bool) -> Result<Vec<u8>> {
        // let l = self.lock.lock().unwrap();
        loop {
            let is_busy = self.busy.load(Ordering::Relaxed);
            if !is_busy {
                // busy now!
                self.busy.store(true, Ordering::Relaxed);
                break;
            } else {
                // wait 5 ms
                thread::sleep(Duration::from_millis(5));
            }
        }

        let dbid = self.client_id.as_ref().map(|c| c.dbid).unwrap_or(0);
        let peer_id = self
            .client_id
            .as_ref()
            .map(|c| c.peer_id.serialize())
            .unwrap_or([0u8; 33]);
        let md = parser::raw_request_from_bytes(message, self.chan.sequence, peer_id, dbid)?;
        // send to signer
        log::info!("SEND ON {}", topics::VLS);
        let (_res_topic, res) = self.send_request_and_get_reply(topics::VLS, md)?;
        // send reply to LSS to store muts
        log::info!("GOT ON {}", _res_topic);
        let lss_reply = self.send_lss_and_get_reply(res)?;
        log::info!("LSS REPLY LEN {}", &lss_reply.len());
        // send to signer for HMAC validation, and get final reply
        log::info!("SEND ON {}", topics::LSS_MSG);
        let (_res_topic, res2) = self.send_request_and_get_reply(topics::LSS_MSG, lss_reply)?;
        // create reply for CLN
        log::info!("GOT ON {}, send to CLN", _res_topic);
        let reply = parser::raw_response_from_bytes(res2, self.chan.sequence)?;
        // add to the sequence
        self.chan.sequence = self.chan.sequence.wrapping_add(1);
        // catch the pubkey if its the first one connection
        if catch_init {
            let _ = self.set_channel_pubkey(reply.clone());
        }
        // unlock
        self.busy.store(false, Ordering::Relaxed);
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
    fn send_request_and_get_reply(
        &mut self,
        topic: &str,
        message: Vec<u8>,
    ) -> Result<(String, Vec<u8>)> {
        // Send a request to the MQTT handler to send to signer
        let (request, reply_rx) = ChannelRequest::new(topic, message);
        // This can fail if MQTT shuts down
        self.chan
            .sender
            .blocking_send(request)
            .map_err(|_| Error::Eof)?;
        let reply = reply_rx.blocking_recv().map_err(|_| Error::Eof)?;

        Ok((reply.topic, reply.reply))
    }

    fn send_lss_and_get_reply(&mut self, message: Vec<u8>) -> Result<Vec<u8>> {
        // Send a request to the MQTT handler to send to signer
        let (request, reply_rx) = LssReq::new(message);
        // This can fail if MQTT shuts down
        self.lss_tx.blocking_send(request).map_err(|_| Error::Eof)?;
        let res = reply_rx.blocking_recv().map_err(|_| Error::Eof)?;
        Ok(res)
    }
}
