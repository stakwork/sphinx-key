use crate::bitcoin::Network;
use crate::ID_LEN;
use anyhow::{anyhow, Context, Result};
use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition};
use glyph::control::{Config, ControlPersist, Controller, FlashKey, Policy, Velocity};
use glyph::ser::*;
use sphinx_signer::sphinx_glyph as glyph;
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
        let mut bytes = Bytes::new(existing);
        deserialize_config(&mut bytes)
    }
    fn write_config(&mut self, conf: Config) -> Result<()> {
        let mut bb = ByteBuf::new();
        serialize_config(&mut bb, &conf)?;
        self.0.set_raw(FlashKey::Config.as_str(), bb.as_slice())?;
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
    fn write_id(&mut self, id: [u8; ID_LEN]) -> Result<()> {
        self.0.set_raw(FlashKey::Id.as_str(), &id[..])?;
        Ok(())
    }
    fn read_id(&self) -> Result<[u8; ID_LEN]> {
        let mut buf = [0u8; ID_LEN];
        let existing = self
            .0
            .get_raw(FlashKey::Id.as_str(), &mut buf)?
            .ok_or(anyhow!("no existing id"))?;
        Ok(existing.try_into()?)
    }
    fn read_policy(&self) -> Result<Policy> {
        let mut buf = [0u8; 250];
        let existing = self
            .0
            .get_raw(FlashKey::Policy.as_str(), &mut buf)?
            .ok_or(anyhow!("no existing policy"))?;
        let mut bytes = Bytes::new(existing);
        deserialize_policy(&mut bytes, None)
    }
    fn write_policy(&mut self, pol: Policy) -> Result<()> {
        let mut bb = ByteBuf::new();
        serialize_policy(&mut bb, None, &pol)?;
        self.0.set_raw(FlashKey::Policy.as_str(), bb.as_slice())?;
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
        let mut bytes = Bytes::new(existing);
        let desvel = deserialize_velocity(&mut bytes, None)?;
        desvel.context(anyhow::anyhow!("no velocity"))
    }
    fn write_velocity(&mut self, vel: Velocity) -> Result<()> {
        let mut bb = ByteBuf::new();
        serialize_velocity(&mut bb, None, Some(&vel))?;
        self.0.set_raw(FlashKey::Velocity.as_str(), bb.as_slice())?;
        Ok(())
    }
}
