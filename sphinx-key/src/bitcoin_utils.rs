use crate::bitcoin::{
    hashes::{sha256, Hash},
    secp256k1::Secp256k1,
    util::misc::{signed_msg_hash, MessageSignature},
    Address,
};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::fs::File;
use std::io::BufReader;

const ADDRESS: &str = "1K51sSTyoVxHhKFtwWpzMZsoHvLshtw3Dp";

pub(crate) fn check_signature(msg: &str, sig: &str) -> Result<()> {
    let add = ADDRESS.parse::<Address>()?;
    let sig = STANDARD.decode(sig)?;
    let sig = MessageSignature::from_slice(&sig)?;
    let secp = Secp256k1::verification_only();
    let signed = sig.is_signed_by_address(&secp, &add, signed_msg_hash(msg))?;
    match signed {
        true => Ok(()),
        false => Err(anyhow!("Failed signature check")),
    }
}

pub(crate) fn check_integrity(file_path: &str, check: &str) -> Result<()> {
    let f = File::open(file_path)?;
    let mut reader = BufReader::new(f);
    let mut engine = sha256::HashEngine::default();
    std::io::copy(&mut reader, &mut engine)?;
    let hash = sha256::Hash::from_engine(engine).to_string();
    if hash == check {
        Ok(())
    } else {
        Err(anyhow!(
            "Integrity check failed! check: {} vs calculated: {}",
            check,
            hash
        ))
    }
}
