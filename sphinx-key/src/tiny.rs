#![feature(once_cell)]

use sphinx_key_signer::{self, DummyPersister, Persist, RootHandler, SignerArc};

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();

    let persister: SignerArc<dyn Persist> = SignerArc::new(DummyPersister);
    let rh = RootHandler::new(0, Some([0; 32]), persister, Vec::new());

    Ok(())
}
