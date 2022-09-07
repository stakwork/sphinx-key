use vls_core::{
    bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey},
    bitcoin::Network,
    signer::derive::{key_derive, KeyDerivationStyle},
};
use vls_protocol_signer::lightning_signer as vls_core;

pub fn node_keys(network: &Network, seed: &[u8]) -> (PublicKey, SecretKey) {
    let style = KeyDerivationStyle::Native;
    let deriver = key_derive(style, network.clone());
    let ctx = Secp256k1::new();
    deriver.node_keys(seed, &ctx)
}
