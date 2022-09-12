use anyhow::Result;
use serde::{Deserialize, Serialize};
use sphinx_auther::nonce;
use sphinx_auther::secp256k1::{PublicKey, SecretKey};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct Config {
    pub broker: String,
    pub ssid: String,
    pub pass: String,
    pub network: String,
    // pub seed: [u8; 32],
}

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
        per: Arc<Mutex<dyn ControlPersist>>,
    ) -> Self {
        let store1 = per.clone();
        let store = store1.lock().unwrap();
        let nonce = store.read_nonce().unwrap_or(0);
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
    pub fn parse_msg_no_nonce(&mut self, input: &[u8]) -> anyhow::Result<ControlMessage> {
        let (msg, _nonce) = nonce::parse_msg_no_nonce(input, &self.1)?;
        let ret = rmp_serde::from_slice(&msg)?;
        Ok(ret)
    }
    pub fn parse_response(&self, input: &[u8]) -> anyhow::Result<ControlResponse> {
        Ok(rmp_serde::from_slice(input)?)
    }
    // return the OG message for further processing
    pub fn handle(&mut self, input: &[u8]) -> anyhow::Result<(Vec<u8>, ControlMessage)> {
        let msg = self.parse_msg_no_nonce(input)?;
        // increment the nonce EXCEPT for Nonce requests
        let mut store = self.3.lock().unwrap();
        match msg {
            ControlMessage::Nonce => (),
            _ => {
                self.2 = self.2 + 1;
                store.set_nonce(self.2)?;
            }
        }
        let res = match msg.clone() {
            ControlMessage::Nonce => ControlResponse::Nonce(self.2),
            ControlMessage::ResetWifi => {
                store.reset();
                ControlResponse::ResetWifi
            }
            ControlMessage::UpdatePolicy(np) => ControlResponse::PolicyUpdated(np),
            _ => ControlResponse::Nonce(self.2),
        };
        let response = self.build_response(res)?;
        Ok((response, msg))
    }
}

#[derive(Debug)]
pub enum FlashKey {
    Config,
    Seed,
    Nonce,
}
impl FlashKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            FlashKey::Config => "config",
            FlashKey::Seed => "seed",
            FlashKey::Nonce => "nonce",
        }
    }
}

pub trait ControlPersist: Sync + Send {
    fn reset(&mut self);
    fn read_nonce(&self) -> Result<u64>;
    fn set_nonce(&mut self, nonce: u64) -> Result<()>;
    fn read_config(&self) -> Result<Config>;
    fn write_config(&mut self, c: Config) -> Result<()>;
    fn read_seed(&self) -> Result<[u8; 32]>;
    fn write_seed(&mut self, s: [u8; 32]) -> Result<()>;
}

pub struct DummyPersister;

impl ControlPersist for DummyPersister {
    fn reset(&mut self) {}
    fn read_nonce(&self) -> Result<u64> {
        Ok(0u64)
    }
    fn set_nonce(&mut self, _nonce: u64) -> Result<()> {
        Ok(())
    }
    fn read_config(&self) -> Result<Config> {
        Ok(Default::default())
    }
    fn write_config(&mut self, _conf: Config) -> Result<()> {
        Ok(())
    }
    fn read_seed(&self) -> Result<[u8; 32]> {
        Ok([0; 32])
    }
    fn write_seed(&mut self, _s: [u8; 32]) -> Result<()> {
        Ok(())
    }
}
