#![feature(once_cell)]

use lightning_signer::node::{NodeConfig};
use lightning_signer::signer::derive::KeyDerivationStyle;
use lightning_signer::signer::my_keys_manager::MyKeysManager;
use std::time::Duration;
use sphinx_key_signer::lightning_signer;
use sphinx_key_signer::lightning_signer::bitcoin::Network;

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    
    let network = Network::Regtest;
    let config = NodeConfig { network, key_derivation_style: KeyDerivationStyle::Native };

    let seed = [0; 32];

    let now = Duration::from_secs(1);
    let keys_manager = MyKeysManager::new(
        config.key_derivation_style,
        &seed[..],
        config.network,
        now.as_secs(),
        now.subsec_nanos(),
    );

    Ok(())
}

