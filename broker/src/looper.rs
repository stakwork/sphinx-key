use crate::conn::{ChannelRequest, LssReq};
use crate::handle::handle_message;
use bitcoin::blockdata::constants::ChainHash;
use bitcoin::Network;
use log::*;
use rocket::tokio::sync::mpsc;
use secp256k1::PublicKey;
use std::sync::atomic::{AtomicU16, Ordering};
use std::thread;
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

/// Implement the hsmd UNIX fd protocol.
pub struct SignerLoop<C: 'static + Client> {
    client: C,
    log_prefix: String,
    vls_tx: mpsc::Sender<ChannelRequest>,
    lss_tx: mpsc::Sender<LssReq>,
    client_id: Option<ClientId>,
}

impl<C: 'static + Client> SignerLoop<C> {
    /// Create a loop for the root (lightningd) connection, but doesn't start it yet
    pub fn new(
        client: C,
        vls_tx: mpsc::Sender<ChannelRequest>,
        lss_tx: mpsc::Sender<LssReq>,
    ) -> Self {
        let log_prefix = format!("{}/{}", std::process::id(), client.id());
        Self {
            client,
            log_prefix,
            vls_tx,
            lss_tx,
            client_id: None,
        }
    }

    // Create a loop for a non-root connection
    fn new_for_client(
        client: C,
        lss_tx: mpsc::Sender<LssReq>,
        vls_tx: mpsc::Sender<ChannelRequest>,
        client_id: ClientId,
    ) -> Self {
        let log_prefix = format!("{}/{}", std::process::id(), client.id());
        Self {
            client,
            log_prefix,
            vls_tx,
            lss_tx,
            client_id: Some(client_id),
        }
    }

    /// Start the read loop
    pub fn start(&mut self, network: Option<Network>) {
        info!("loop {}: start", self.log_prefix);
        match self.do_loop(network) {
            Ok(()) => info!("loop {}: done", self.log_prefix),
            Err(Error::Eof) => info!("loop {}: ending", self.log_prefix),
            Err(e) => error!("loop {}: error {:?}", self.log_prefix, e),
        }
    }

    fn do_loop(&mut self, network: Option<Network>) -> Result<()> {
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
                        self.vls_tx.clone(),
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
                    if let Message::HsmdInit(m) = msg {
                        if let Some(net) = network {
                            if ChainHash::using_genesis_block(net).as_bytes()
                                != m.chain_params.as_ref()
                            {
                                panic!("The network settings of CLN and broker don't match!");
                            }
                        } else {
                            log::error!("No Network provided");
                        }
                    }
                    let reply =
                        handle_message(&self.client_id, raw_msg, &self.vls_tx, &self.lss_tx);
                    // Write the reply to CLN
                    self.client.write_vec(reply)?;
                    // Only send heartbeat messages from the root loop, as roothandler alone can process them, not channelhandler
                    // Send it every ten messages to prune extraneous data on hardware signer
                    if self.client_id.is_none() && send_heartbeat % 10u8 == 0u8 {
                        let beat = msgs::GetHeartbeat {};
                        let _ = handle_message(
                            &self.client_id,
                            beat.as_vec(),
                            &self.vls_tx,
                            &self.lss_tx,
                        );
                    }
                }
            }
            send_heartbeat = send_heartbeat.wrapping_add(1u8);
        }
    }
}
