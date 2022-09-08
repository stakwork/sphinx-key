use serde::{Deserialize, Serialize};
use sphinx_auther::nonce;
use sphinx_auther::secp256k1::{PublicKey, SecretKey};
use std::sync::{Arc, Mutex};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ControlMessage {
    Nonce,
    ResetWifi,
    QueryPolicy,
    UpdatePolicy(Policy),
    Ota(OtaParams),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ControlResponse {
    Nonce(u64),
    ResetWifi,
    PolicyCurrent(Policy),
    PolicyUpdated(Policy),
    OtaConfirm(OtaParams),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Policy {
    pub sats_per_day: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OtaParams {
    pub version: u64,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WifiParams {
    pub ssid: String,
    pub password: String,
}

// u64 is the nonce. Each signature must have a higher nonce
pub struct Controller(SecretKey, PublicKey, u64, Arc<Mutex<dyn ControlPersist>>);

impl Controller {
    pub fn new(sk: SecretKey, pk: PublicKey, nonce: u64) -> Self {
        Self(sk, pk, nonce, Arc::new(Mutex::new(DummyPersister)))
    }
    pub fn new_with_persister(
        sk: SecretKey,
        pk: PublicKey,
        nonce: u64,
        per: Arc<Mutex<dyn ControlPersist>>,
    ) -> Self {
        Self(sk, pk, nonce, per)
    }
    pub fn build_msg(&mut self, msg: ControlMessage) -> anyhow::Result<Vec<u8>> {
        let data = rmp_serde::to_vec(&msg)?;
        self.2 = self.2 + 1;
        let ret = nonce::build_msg(&data, &self.0, self.2)?;
        Ok(ret)
    }
    pub fn build_response(&self, msg: ControlResponse) -> anyhow::Result<Vec<u8>> {
        Ok(rmp_serde::to_vec(&msg)?)
    }
    pub fn parse_msg(&mut self, input: &[u8]) -> anyhow::Result<ControlMessage> {
        let msg = nonce::parse_msg(input, &self.1, self.2)?;
        let ret = rmp_serde::from_slice(&msg)?;
        self.2 = self.2 + 1;
        Ok(ret)
    }
    pub fn parse_response(&self, input: &[u8]) -> anyhow::Result<ControlResponse> {
        Ok(rmp_serde::from_slice(input)?)
    }
    pub fn handle(&mut self, input: &[u8]) -> anyhow::Result<Vec<u8>> {
        let msg = self.parse_msg(input)?;
        let mut store = self.3.lock().unwrap();
        let res = match msg {
            ControlMessage::Nonce => ControlResponse::Nonce(self.2),
            ControlMessage::ResetWifi => {
                store.reset();
                ControlResponse::ResetWifi
            }
            _ => ControlResponse::Nonce(self.2),
        };
        Ok(self.build_response(res)?)
    }
}

pub trait ControlPersist: Sync + Send {
    fn reset(&mut self);
}

pub struct DummyPersister;

impl ControlPersist for DummyPersister {
    fn reset(&mut self) {
        // nothing
    }
}
