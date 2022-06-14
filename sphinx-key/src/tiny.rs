#![feature(once_cell)]

use sphinx_key_signer::lightning_signer::bitcoin::secp256k1::Secp256k1;

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();

    let ctx = Secp256k1::new();

    Ok(())
}

