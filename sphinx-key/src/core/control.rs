use anyhow::{anyhow, Result};
use embedded_svc::storage::RawStorage;
use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition};
use sphinx_signer::lightning_signer::bitcoin::Network;
use sphinx_signer::sphinx_glyph::control::{
    Config, ControlPersist, Controller, FlashKey, Policy, Velocity,
};
use std::convert::TryInto;
use std::sync::{Arc, Mutex};

// the controller validates Control messages
pub fn controller_from_seed(
    network: &Network,
    seed: &[u8],
    flash: Arc<Mutex<FlashPersister>>,
) -> Controller {
    let (pk, sk) = sphinx_signer::derive_node_keys(network, seed);
    Controller::new_with_persister(sk, pk, flash)
}

// EspDefaultNvsPartition
pub struct FlashPersister(pub EspDefaultNvs);

impl FlashPersister {
    pub fn new(nvs: EspDefaultNvsPartition) -> Self {
        let store = EspDefaultNvs::new(nvs, "sphinx", true).expect("no storage");
        Self(store)
    }
}

impl ControlPersist for FlashPersister {
    fn read_nonce(&self) -> Result<u64> {
        let mut buf = [0u8; 8];
        let existing = self
            .0
            .get_raw(FlashKey::Nonce.as_str(), &mut buf)?
            .ok_or(anyhow!("no existing nonce"))?;
        let r: [u8; 8] = existing.try_into()?;
        Ok(u64::from_be_bytes(r))
    }
    fn set_nonce(&mut self, nonce: u64) -> Result<()> {
        let n = nonce.to_be_bytes();
        self.0.set_raw(FlashKey::Nonce.as_str(), &n[..])?;
        Ok(())
    }
    fn read_config(&self) -> Result<Config> {
        let mut buf = [0u8; 250];
        let existing = self
            .0
            .get_raw(FlashKey::Config.as_str(), &mut buf)?
            .ok_or(anyhow!("no existing config"))?;
        Ok(rmp_serde::from_slice(existing)?)
    }
    fn write_config(&mut self, conf: Config) -> Result<()> {
        let conf1 = rmp_serde::to_vec(&conf)?;
        self.0.set_raw(FlashKey::Config.as_str(), &conf1[..])?;
        Ok(())
    }
    fn remove_config(&mut self) -> Result<()> {
        self.0.remove(FlashKey::Config.as_str())?;
        Ok(())
    }
    fn read_seed(&self) -> Result<[u8; 32]> {
        let mut buf = [0u8; 32];
        let s = self
            .0
            .get_raw(FlashKey::Seed.as_str(), &mut buf)?
            .ok_or(anyhow!("no existing seed"))?;
        let r: [u8; 32] = s.try_into()?;
        Ok(r)
    }
    fn write_seed(&mut self, s: [u8; 32]) -> Result<()> {
        self.0.set_raw(FlashKey::Seed.as_str(), &s[..])?;
        Ok(())
    }
    fn remove_seed(&mut self) -> Result<()> {
        self.0.remove(FlashKey::Seed.as_str())?;
        Ok(())
    }
    fn read_policy(&self) -> Result<Policy> {
        let mut buf = [0u8; 250];
        let existing = self
            .0
            .get_raw(FlashKey::Policy.as_str(), &mut buf)?
            .ok_or(anyhow!("no existing policy"))?;
        Ok(rmp_serde::from_slice(existing)?)
    }
    fn write_policy(&mut self, pol: Policy) -> Result<()> {
        let pol1 = rmp_serde::to_vec(&pol)?;
        self.0.set_raw(FlashKey::Policy.as_str(), &pol1[..])?;
        Ok(())
    }
    fn remove_policy(&mut self) -> Result<()> {
        self.0.remove(FlashKey::Policy.as_str())?;
        Ok(())
    }
    fn read_velocity(&self) -> Result<Velocity> {
        let mut buf = [0u8; 250];
        let existing = self
            .0
            .get_raw(FlashKey::Velocity.as_str(), &mut buf)?
            .ok_or(anyhow!("no existing velocity"))?;
        Ok(rmp_serde::from_slice(existing)?)
    }
    fn write_velocity(&mut self, vel: Velocity) -> Result<()> {
        let vel1 = rmp_serde::to_vec(&vel)?;
        self.0.set_raw(FlashKey::Velocity.as_str(), &vel1[..])?;
        Ok(())
    }
}
