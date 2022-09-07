use crate::{ChannelReply, ChannelRequest};
use async_trait::async_trait;
use rocket::tokio::sync::{mpsc, oneshot};
use vls_protocol::{Error, Result};
use vls_protocol_client::SignerPort;

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
        let reply = reply_rx.await.map_err(|_| Error::Eof)?;
        Ok(reply.reply)
    }
}
