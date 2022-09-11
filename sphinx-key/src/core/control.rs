use anyhow::{anyhow, Result};
use embedded_svc::storage::RawStorage;
use embedded_svc::storage::StorageBase;
use esp_idf_svc::nvs::EspDefaultNvs;
use esp_idf_svc::nvs_storage::EspNvsStorage;
use sphinx_key_signer::control::{Config, ControlPersist, Controller, FlashKey};
use sphinx_key_signer::lightning_signer::bitcoin::Network;
use std::convert::TryInto;
use std::sync::{Arc, Mutex};
// the controller validates Control messages
pub fn controller_from_seed(
    network: &Network,
    seed: &[u8],
    flash: Arc<Mutex<FlashPersister>>,
) -> Controller {
    let (pk, sk) = sphinx_key_signer::derive_node_keys(network, seed);
    Controller::new_with_persister(sk, pk, 0, flash)
}

pub struct FlashPersister(pub EspNvsStorage);

impl FlashPersister {
    pub fn new(nvs: Arc<EspDefaultNvs>) -> Self {
        let store = EspNvsStorage::new_default(nvs, "sphinx", true).expect("no storage");
        Self(store)
    }
}

impl ControlPersist for FlashPersister {
    fn reset(&mut self) {
        self.0
            .remove(FlashKey::Config.as_str())
            .expect("couldnt remove config 1");
    }
    fn set_nonce(&mut self, nonce: u64) {
        // self.0.set
        //
    }
    fn read_config(&self) -> Result<Config> {
        let mut buf = [0u8; 250];
        let existing = self.0.get_raw(FlashKey::Config.as_str(), &mut buf)?;
        if let None = existing {
            return Err(anyhow!("no existing config"));
        }
        Ok(rmp_serde::from_slice(existing.unwrap().0)?)
    }
    fn write_config(&mut self, conf: Config) -> Result<()> {
        let conf1 = rmp_serde::to_vec(&conf)?;
        self.0.put_raw(FlashKey::Config.as_str(), &conf1[..])?;
        Ok(())
    }
    fn read_seed(&self) -> Result<[u8; 32]> {
        let mut buf = [0u8; 32];
        let s = self.0.get_raw(FlashKey::Seed.as_str(), &mut buf)?;
        if let None = s {
            return Err(anyhow!("no existing seed"));
        }
        let r: [u8; 32] = s.unwrap().0.try_into()?;
        Ok(r)
    }
    fn write_seed(&mut self, s: [u8; 32]) -> Result<()> {
        self.0.put_raw(FlashKey::Seed.as_str(), &s[..])?;
        Ok(())
    }
}
