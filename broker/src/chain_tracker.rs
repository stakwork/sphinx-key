use crate::conn::{ChannelReply, ChannelRequest};
use async_trait::async_trait;
use rocket::tokio::sync::{mpsc, oneshot};
use sphinx_signer::{parser, sphinx_glyph::topics};
use vls_protocol::{Error, Result};
use vls_protocol_client::{ClientResult, SignerPort};

pub struct MqttSignerPort {
    sender: mpsc::Sender<ChannelRequest>,
}

#[async_trait]
impl SignerPort for MqttSignerPort {
    async fn handle_message(&self, message: Vec<u8>) -> ClientResult<Vec<u8>> {
        let reply_rx = self.send_request(message).await?;
        self.get_reply(reply_rx).await
    }

    fn is_ready(&self) -> bool {
        true
    }
}

impl MqttSignerPort {
    pub fn new(sender: mpsc::Sender<ChannelRequest>) -> Self {
        Self { sender }
    }

    async fn send_request(&self, message: Vec<u8>) -> Result<oneshot::Receiver<ChannelReply>> {
        let m = parser::raw_request_from_bytes(message, 0, [0;33], 0)?;
        let (request, reply_rx) = ChannelRequest::new(topics::VLS, m);
        self.sender.send(request).await.map_err(|_| Error::Eof)?;
        Ok(reply_rx)
    }

    async fn get_reply(&self, reply_rx: oneshot::Receiver<ChannelReply>) -> ClientResult<Vec<u8>> {
        let reply = reply_rx.await.map_err(|_| Error::Eof)?;
        let r = parser::raw_response_from_bytes(reply.reply, 0)?;
        Ok(r)
    }
}
