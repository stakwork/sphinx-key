use log::*;
use secp256k1::PublicKey;
use sphinx_key_parser as parser;
use std::thread;
use tokio::sync::{mpsc, oneshot};
// use tokio::task::spawn_blocking;
use crate::{Channel, ChannelReply, ChannelRequest};
use async_trait::async_trait;
use vls_protocol::{msgs, msgs::Message, Error, Result};
use vls_protocol_client::SignerPort;
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
    pub fn start(&mut self) {
        info!("loop {}: start", self.log_prefix);
        match self.do_loop() {
            Ok(()) => info!("loop {}: done", self.log_prefix),
            Err(Error::Eof) => info!("loop {}: ending", self.log_prefix),
            Err(e) => error!("loop {}: error {:?}", self.log_prefix, e),
        }
    }

    fn do_loop(&mut self) -> Result<()> {
        loop {
            let raw_msg = self.client.read_raw()?;
            debug!("loop {}: got raw", self.log_prefix);
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
                    let mut new_loop =
                        SignerLoop::new_for_client(new_client, self.chan.sender.clone(), client_id);
                    thread::spawn(move || new_loop.start());
                }
                Message::Memleak(_) => {
                    let reply = msgs::MemleakReply { result: false };
                    self.client.write(reply)?;
                }
                _ => {
                    let reply = self.handle_message(raw_msg)?;
                    // Write the reply to the node
                    self.client.write_vec(reply)?;
                    info!("replied {}", self.log_prefix);
                }
            }
        }
    }

    fn handle_message(&mut self, message: Vec<u8>) -> Result<Vec<u8>> {
        let dbid = self.client_id.as_ref().map(|c| c.dbid).unwrap_or(0);
        let md = parser::raw_request_from_bytes(message, self.chan.sequence, dbid)?;
        let reply_rx = self.send_request(md)?;
        let res = self.get_reply(reply_rx)?;
        let reply = parser::raw_response_from_bytes(res, self.chan.sequence)?;
        self.chan.sequence = self.chan.sequence.wrapping_add(1);
        Ok(reply)
    }

    fn send_request(&mut self, message: Vec<u8>) -> Result<oneshot::Receiver<ChannelReply>> {
        // Create a one-shot channel to receive the reply
        let (reply_tx, reply_rx) = oneshot::channel();
        // Send a request to the MQTT handler to send to signer
        let request = ChannelRequest { message, reply_tx };
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

pub struct MqttSignerPort {
    sender: mpsc::Sender<ChannelRequest>,
}

#[async_trait]
impl SignerPort for MqttSignerPort {
    async fn handle_message(&self, message: Vec<u8>) -> Result<Vec<u8>> {
        let reply_rx = self.send_request(message).await?;
        self.get_reply(reply_rx).await
    }

    fn clone(&self) -> Box<dyn SignerPort> {
        Box::new(Self {
            sender: self.sender.clone(),
        })
    }
}

impl MqttSignerPort {
    pub fn new(sender: mpsc::Sender<ChannelRequest>) -> Self {
        Self { sender }
    }

    async fn send_request(&self, message: Vec<u8>) -> Result<oneshot::Receiver<ChannelReply>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let request = ChannelRequest { message, reply_tx };
        self.sender.send(request).await.map_err(|_| Error::Eof)?;
        Ok(reply_rx)
    }

    async fn get_reply(&self, reply_rx: oneshot::Receiver<ChannelReply>) -> Result<Vec<u8>> {
        let reply = reply_rx.blocking_recv().map_err(|_| Error::Eof)?;
        Ok(reply.reply)
    }
}
