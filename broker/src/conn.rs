use anyhow::Result;
use rocket::tokio::sync::{mpsc, oneshot};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Connections {
    pub pubkey: Option<String>,
    pub clients: HashMap<String, bool>,
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
    pub fn len(&self) -> usize {
        self.clients.len()
    }
    pub fn set_pubkey(&mut self, pk: &str) {
        self.pubkey = Some(pk.to_string())
    }
    pub fn set_current(&mut self, cid: String) {
        self.current = Some(cid);
    }
    fn add_client(&mut self, cid: &str) {
        self.clients.insert(cid.to_string(), true);
        self.current = Some(cid.to_string());
    }
    fn remove_client(&mut self, cid: &str) {
        self.clients.remove(cid);
        if self.current == Some(cid.to_string()) {
            self.current = None;
        }
    }
    pub fn client_action(&mut self, cid: &str, connected: bool) {
        if connected {
            self.add_client(cid);
        } else {
            self.remove_client(cid);
        }
    }
}

pub struct Channel {
    pub sender: mpsc::Sender<ChannelRequest>,
    pub pubkey: [u8; 33],
}

/// Responses are received on the oneshot sender
#[derive(Debug)]
pub struct ChannelRequest {
    pub topic: String,
    pub message: Vec<u8>,
    pub reply_tx: oneshot::Sender<ChannelReply>,
    pub cid: Option<String>, // if it exists, only try the one client
}
impl ChannelRequest {
    pub fn new(topic: &str, message: Vec<u8>) -> (Self, oneshot::Receiver<ChannelReply>) {
        let (reply_tx, reply_rx) = oneshot::channel();
        let cr = ChannelRequest {
            topic: topic.to_string(),
            message,
            reply_tx,
            cid: None,
        };
        (cr, reply_rx)
    }
    pub async fn send(
        topic: &str,
        message: Vec<u8>,
        sender: &mpsc::Sender<ChannelRequest>,
    ) -> Result<Vec<u8>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let req = ChannelRequest {
            topic: topic.to_string(),
            message,
            reply_tx,
            cid: None,
        };
        let _ = sender.send(req).await;
        let reply = reply_rx.await?;
        Ok(reply.reply)
    }
    pub async fn send_for(
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
            cid: Some(cid.to_string()),
        };
        let _ = sender.send(req).await;
        let reply = reply_rx.await?;
        Ok(reply.reply)
    }
    pub fn for_cid(&mut self, cid: &str) {
        self.cid = Some(cid.to_string())
    }
    pub fn new_for(
        cid: &str,
        topic: &str,
        message: Vec<u8>,
    ) -> (Self, oneshot::Receiver<ChannelReply>) {
        let (mut cr, reply_rx) = ChannelRequest::new(topic, message);
        cr.for_cid(cid);
        (cr, reply_rx)
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
    pub message: Vec<u8>,
    pub reply_tx: oneshot::Sender<Vec<u8>>,
}
impl LssReq {
    pub fn new(message: Vec<u8>) -> (Self, oneshot::Receiver<Vec<u8>>) {
        let (reply_tx, reply_rx) = oneshot::channel();
        let cr = Self { message, reply_tx };
        (cr, reply_rx)
    }
}
