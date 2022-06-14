#![feature(once_cell)]

// use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
use sphinx_key_signer::lightning_signer::bitcoin::secp256k1::{
    Message, PublicKey, Secp256k1, SecretKey,
};

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();

    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&[0xcd; 32]).expect("32 bytes, within curve order");
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    // This is unsafe unless the supplied byte slice is the output of a cryptographic hash function.
    // See the above example for how to use this library together with `bitcoin_hashes`.
    let message = Message::from_slice(&[0xab; 32]).expect("32 bytes");

    let sig = secp.sign(&message, &secret_key);
    assert!(secp.verify(&message, &sig, &public_key).is_ok());

    println!("signature verified!");
    Ok(())
}
