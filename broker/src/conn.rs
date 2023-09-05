use anyhow::Result;
use rocket::tokio::sync::{mpsc, oneshot};
use serde::{Deserialize, Serialize};
use sphinx_signer::sphinx_glyph::types::SignerType;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Connections {
    pub pubkey: Option<String>,
    pub clients: HashMap<String, SignerType>,
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
    pub fn add_client(&mut self, cid: &str, signer_type: SignerType) {
        self.clients.insert(cid.to_string(), signer_type);
        self.current = Some(cid.to_string());
    }
    pub fn remove_client(&mut self, cid: &str) {
        self.clients.remove(cid);
        if let Some(id) = &self.current {
            if id == cid {
                self.current = None;
            }
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
    pub signer_type: Option<SignerType>, // if it exists, only try clients of these types
}
impl ChannelRequest {
    pub fn new(topic: &str, message: Vec<u8>) -> (Self, oneshot::Receiver<ChannelReply>) {
        let (reply_tx, reply_rx) = oneshot::channel();
        let cr = ChannelRequest {
            topic: topic.to_string(),
            message,
            reply_tx,
            cid: None,
            signer_type: None,
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
            signer_type: None,
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
            signer_type: None,
        };
        let _ = sender.send(req).await;
        let reply = reply_rx.await?;
        Ok(reply.reply)
    }
    pub fn for_cid(&mut self, cid: &str) {
        self.cid = Some(cid.to_string());
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
    pub fn for_type(&mut self, signer_type: SignerType) {
        self.signer_type = Some(signer_type);
    }
    pub fn new_for_type(
        signer_type: SignerType,
        topic: &str,
        message: Vec<u8>,
    ) -> (Self, oneshot::Receiver<ChannelReply>) {
        let (mut cr, reply_rx) = ChannelRequest::new(topic, message);
        cr.for_type(signer_type);
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
