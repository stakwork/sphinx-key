use anyhow::Result;
use once_cell::sync::Lazy;
use rocket::tokio::sync::{mpsc, oneshot};
use serde::{Deserialize, Serialize};
use std::collections::HashMap; // 1.3.1
use std::sync::Mutex;

pub static CONNS: Lazy<Mutex<Connections>> = Lazy::new(|| Mutex::new(Connections::new()));

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Connections {
    pub pubkey: Option<String>,
    pub clients: HashMap<String, bool>, // bool is "synced" state (done with dance)
    pub current: Option<String>,
}

impl Connections {
    pub fn new() -> Self {
        Self {
            pubkey: None,
            clients: HashMap::new(),
            current: None,
        }
    }
    fn connect_client(&mut self, cid: &str, synced: bool) {
        self.clients.insert(cid.to_string(), synced);
        self.current = Some(cid.to_string());
    }
    /*
    fn remove_client(&mut self, cid: &str) {
        self.clients.remove(cid);
        if self.current == Some(cid.to_string()) {
            self.current = None;
        }
    }
    */
}

pub fn current_client() -> Option<String> {
    CONNS.lock().unwrap().current.clone()
}

pub fn current_client_and_synced() -> (Option<String>, bool) {
    let cs = CONNS.lock().unwrap();
    let c = cs.current.clone();
    let mut b = false;
    if let Some(ref client) = c {
        b = *cs.clients.get(client).unwrap_or(&false);
    }
    (c, b)
}

pub fn current_pubkey() -> Option<String> {
    CONNS.lock().unwrap().pubkey.clone()
}

pub fn current_conns() -> Connections {
    CONNS.lock().unwrap().clone()
}

pub fn conns_set_pubkey(pubkey: String) {
    let mut cs = CONNS.lock().unwrap();
    cs.pubkey = Some(pubkey);
}

pub fn new_connection(cid: &str, connected: bool) {
    let mut cs = CONNS.lock().unwrap();
    cs.connect_client(cid, connected);
}

pub fn cycle_clients(cid: &str) {
    let mut cs = CONNS.lock().unwrap();
    let clients = cs.clients.clone();
    let other_clients: Vec<String> = clients.into_keys().filter(|k| k != cid).collect();
    if let Some(nc) = other_clients.get(0) {
        log::info!("=> client switched to {}", nc);
        cs.current = Some(nc.to_string());
    }
}

/// Responses are received on the oneshot sender
#[derive(Debug)]
pub struct ChannelRequest {
    pub topic: String,
    pub message: Vec<u8>,
    pub reply_tx: oneshot::Sender<ChannelReply>,
    pub cid: String, // if it exists, only try the one client
}
impl ChannelRequest {
    pub fn new(
        cid: &str,
        topic: &str,
        message: Vec<u8>,
    ) -> (Self, oneshot::Receiver<ChannelReply>) {
        let (reply_tx, reply_rx) = oneshot::channel();
        let cr = ChannelRequest {
            topic: topic.to_string(),
            message,
            reply_tx,
            cid: cid.to_string(),
        };
        (cr, reply_rx)
    }
    pub async fn send(
        cid: &str,
        topic: &str,
        message: Vec<u8>,
        sender: &mpsc::Sender<ChannelRequest>,
    ) -> Result<Vec<u8>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let req = ChannelRequest {
            topic: topic.to_string(),
            message,
            reply_tx,
            cid: cid.to_string(),
        };
        let _ = sender.send(req).await;
        let reply = reply_rx.await?;
        Ok(reply.reply)
    }
}

// mpsc reply
#[derive(Debug)]
pub struct ChannelReply {
    // the return topic end part (after last "/")
    pub topic_end: String,
    pub reply: Vec<u8>,
}
impl ChannelReply {
    pub fn new(topic_end: String, reply: Vec<u8>) -> Self {
        Self { topic_end, reply }
    }
    pub fn empty() -> Self {
        Self {
            topic_end: "".to_string(),
            reply: Vec::new(),
        }
    }
    // failed channel request
    pub fn is_empty(&self) -> bool {
        self.topic_end.len() == 0 && self.reply.len() == 0
    }
}

/// Responses are received on the oneshot sender
#[derive(Debug)]
pub struct LssReq {
    pub topic: String,
    pub message: Vec<u8>,
    pub reply_tx: oneshot::Sender<(String, Vec<u8>)>,
}
impl LssReq {
    pub fn new(topic: String, message: Vec<u8>) -> (Self, oneshot::Receiver<(String, Vec<u8>)>) {
        let (reply_tx, reply_rx) = oneshot::channel();
        let cr = Self {
            topic,
            message,
            reply_tx,
        };
        (cr, reply_rx)
    }
}
