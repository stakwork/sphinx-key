use crate::conn::{current_client_and_synced, ChannelRequest, LssReq};
use async_trait::async_trait;
use rocket::tokio;
use tokio::sync::mpsc;
use vls_protocol_client::{ClientResult, SignerPort};

pub struct MqttSignerPort {
    vls_tx: mpsc::Sender<ChannelRequest>,
    lss_tx: mpsc::Sender<LssReq>,
}

#[async_trait]
impl SignerPort for MqttSignerPort {
    async fn handle_message(&self, message: Vec<u8>) -> ClientResult<Vec<u8>> {
        let vls_tx = self.vls_tx.clone();
        let lss_tx = self.lss_tx.clone();
        let reply = tokio::task::spawn_blocking(move || {
            crate::handle::handle_message(&None, message, &vls_tx, &lss_tx)
        })
        .await
        .unwrap();
        Ok(reply)
    }

    fn is_ready(&self) -> bool {
        let (cid, is_synced) = current_client_and_synced();
        cid.is_some() && is_synced
    }
}

impl MqttSignerPort {
    pub fn new(vls_tx: mpsc::Sender<ChannelRequest>, lss_tx: mpsc::Sender<LssReq>) -> Self {
        Self { vls_tx, lss_tx }
    }
}
