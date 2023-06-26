use crate::conn::{ChannelRequest, LssReq};
use anyhow::Result;
use async_trait::async_trait;
use rocket::tokio::sync::mpsc;
use sphinx_signer::{parser, sphinx_glyph::topics};
use vls_protocol::Error;
use vls_protocol_client::{ClientResult, SignerPort};

pub struct MqttSignerPort {
    sender: mpsc::Sender<ChannelRequest>,
    lss_tx: mpsc::Sender<LssReq>,
}

#[async_trait]
impl SignerPort for MqttSignerPort {
    async fn handle_message(&self, message: Vec<u8>) -> ClientResult<Vec<u8>> {
        Ok(self.send_and_wait(message).await.map_err(|_| Error::Eof)?)
    }

    fn is_ready(&self) -> bool {
        true
    }
}

impl MqttSignerPort {
    pub fn new(sender: mpsc::Sender<ChannelRequest>, lss_tx: mpsc::Sender<LssReq>) -> Self {
        Self { sender, lss_tx }
    }

    async fn send_and_wait(&self, message: Vec<u8>) -> Result<Vec<u8>> {
        let m = parser::raw_request_from_bytes(message, 0, [0; 33], 0)?;
        let (res_topic, res) = self.send_request_wait(topics::VLS, m).await?;
        let mut the_res = res.clone();
        if res_topic == topics::LSS_RES {
            // send LSS instead
            let lss_reply = self.send_lss(res).await?;
            let (_res_topic, res2) = self.send_request_wait(topics::LSS_MSG, lss_reply).await?;
            the_res = res2;
        }
        let r = parser::raw_response_from_bytes(the_res, 0)?;
        Ok(r)
    }

    async fn send_request_wait(&self, topic: &str, message: Vec<u8>) -> Result<(String, Vec<u8>)> {
        let (request, reply_rx) = ChannelRequest::new(topic, message);
        self.sender.send(request).await?;
        let reply = reply_rx.await?;
        Ok((reply.topic_end, reply.reply))
    }

    async fn send_lss(&self, message: Vec<u8>) -> Result<Vec<u8>> {
        // Send a request to the MQTT handler to send to signer
        let (request, reply_rx) = LssReq::new(message);
        // This can fail if MQTT shuts down
        self.lss_tx.send(request).await?;
        let res = reply_rx.await?;
        Ok(res)
    }
}
