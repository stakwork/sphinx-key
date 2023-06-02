use rocket::tokio::sync::{mpsc, oneshot};
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize)]
pub struct Connections {
    pub pubkey: Option<String>,
    pub clients: Vec<String>,
}

impl Connections {
    pub fn new() -> Self {
        Self {
            pubkey: None,
            clients: Vec::new(),
        }
    }
    pub fn set_pubkey(&mut self, pk: &str) {
        self.pubkey = Some(pk.to_string())
    }
    pub fn add_client(&mut self, cid: &str) {
        let cids = cid.to_string();
        if !self.clients.contains(&cids) {
            self.clients.push(cids)
        }
    }
    pub fn remove_client(&mut self, cid: &str) {
        let cids = cid.to_string();
        if self.clients.contains(&cids) {
            self.clients.retain(|x| x != cid)
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
    pub sequence: u16,
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
    pub async fn send(topic: &str, message: Vec<u8>, sender: &mpsc::Sender<ChannelRequest>) -> Result<Vec<u8>> {
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
    pub async fn send_for(cid: &str, topic: &str, message: Vec<u8>, sender: &mpsc::Sender<ChannelRequest>) -> Result<Vec<u8>> {
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
    // the return topic
    pub topic: String,
    pub reply: Vec<u8>,
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
