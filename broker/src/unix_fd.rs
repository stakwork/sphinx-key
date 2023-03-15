use crate::util::Settings;
use crate::{Channel, ChannelReply, ChannelRequest};
use bitcoin::blockdata::constants::ChainHash;
use log::*;
use rocket::tokio::sync::{mpsc, oneshot};
use secp256k1::PublicKey;
use sphinx_signer::{parser, sphinx_glyph::topics};
use std::thread;
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
        }
    }
}

/// Implement the hsmd UNIX fd protocol.
pub struct SignerLoop<C: 'static + Client> {
    client: C,
    log_prefix: String,
    chan: Channel,
    client_id: Option<ClientId>,
}

impl<C: 'static + Client> SignerLoop<C> {
    /// Create a loop for the root (lightningd) connection, but doesn't start it yet
    pub fn new(client: C, sender: mpsc::Sender<ChannelRequest>) -> Self {
        let log_prefix = format!("{}/{}", std::process::id(), client.id());
        Self {
            client,
            log_prefix,
            chan: Channel::new(sender),
            client_id: None,
        }
    }

    // Create a loop for a non-root connection
    fn new_for_client(
        client: C,
        sender: mpsc::Sender<ChannelRequest>,
        client_id: ClientId,
    ) -> Self {
        let log_prefix = format!("{}/{}", std::process::id(), client.id());
        Self {
            client,
            log_prefix,
            chan: Channel::new(sender),
            client_id: Some(client_id),
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
                    let mut new_loop =
                        SignerLoop::new_for_client(new_client, self.chan.sender.clone(), client_id);
                    thread::spawn(move || new_loop.start(None));
                }
                Message::Memleak(_) => {
                    // info!("Memleak");
                    let reply = msgs::MemleakReply { result: false };
                    self.client.write(reply)?;
                }
                msg => {
                    if let Message::HsmdInit(m) = msg {
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
                    let reply = self.handle_message(raw_msg)?;
                    // Write the reply to the node
                    self.client.write_vec(reply)?;
                    // info!("replied {}", self.log_prefix);
                }
            }
        }
    }

    fn handle_message(&mut self, message: Vec<u8>) -> Result<Vec<u8>> {
        let dbid = self.client_id.as_ref().map(|c| c.dbid).unwrap_or(0);
        let peer_id = self
            .client_id
            .as_ref()
            .map(|c| c.peer_id.serialize())
            .unwrap_or([0u8; 33]);
        let md = parser::raw_request_from_bytes(message, self.chan.sequence, peer_id, dbid)?;
        let reply_rx = self.send_request(md)?;
        let res = self.get_reply(reply_rx)?;
        let reply = parser::raw_response_from_bytes(res, self.chan.sequence)?;
        self.chan.sequence = self.chan.sequence.wrapping_add(1);
        Ok(reply)
    }

    fn send_request(&mut self, message: Vec<u8>) -> Result<oneshot::Receiver<ChannelReply>> {
        // Send a request to the MQTT handler to send to signer
        let (request, reply_rx) = ChannelRequest::new(topics::VLS, message);
        // This can fail if MQTT shuts down
        self.chan
            .sender
            .blocking_send(request)
            .map_err(|_| Error::Eof)?;
        Ok(reply_rx)
    }

    fn get_reply(&mut self, reply_rx: oneshot::Receiver<ChannelReply>) -> Result<Vec<u8>> {
        // Wait for the signer reply
        // Can fail if MQTT shuts down
        let reply = reply_rx.blocking_recv().map_err(|_| Error::Eof)?;
        Ok(reply.reply)
    }
}
