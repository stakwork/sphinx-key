use embedded_svc::storage::StorageBase;
use esp_idf_svc::nvs_storage::EspNvsStorage;
use sphinx_key_signer::control::{ControlPersist, Controller};
use sphinx_key_signer::lightning_signer::bitcoin::Network;
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

impl ControlPersist for FlashPersister {
    fn reset(&mut self) {
        // let mut store = self.0.lock();
        self.0.remove("config").expect("couldnt remove config 1");
    }
}
