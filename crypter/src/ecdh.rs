use secp256k1::ecdh::SharedSecret;
use secp256k1::{SecretKey, PublicKey};
use anyhow::Result;

pub const PUBLIC_KEY_LEN: usize = 33;
pub const PRIVATE_KEY_LEN: usize = 32;
pub const SECRET_LEN: usize = 32;

pub fn derive_shared_secret_from_slice(their_public_key: [u8; PUBLIC_KEY_LEN], my_private_key: [u8; PRIVATE_KEY_LEN]) -> Result<[u8; SECRET_LEN]> {
  let public_key = PublicKey::from_slice(&their_public_key[..])?;
  let private_key = SecretKey::from_slice(&my_private_key[..])?;
  Ok(derive_shared_secret(&public_key, &private_key).secret_bytes())
}

pub fn derive_shared_secret(their_public_key: &PublicKey, my_private_key: &SecretKey) -> SharedSecret {
  SharedSecret::new(their_public_key, my_private_key)
}

#[cfg(test)]
mod tests {
  use crate::ecdh::{derive_shared_secret, derive_shared_secret_from_slice};
  use rand::thread_rng;
  use secp256k1::Secp256k1;

  #[test]
  fn test_ecdh() -> anyhow::Result<()> {
    let s = Secp256k1::new();
    let (sk1, pk1) = s.generate_keypair(&mut thread_rng());
    let (sk2, pk2) = s.generate_keypair(&mut thread_rng());
    let sec1 = derive_shared_secret(&pk2, &sk1);
    let sec2 = derive_shared_secret(&pk1, &sk2);
    assert_eq!(sec1, sec2);
    Ok(())
  }

  #[test]
  fn test_ecdh_from_slice() -> anyhow::Result<()> {
    let s = Secp256k1::new();
    let (sk1, pk1) = s.generate_keypair(&mut thread_rng());
    let (sk2, pk2) = s.generate_keypair(&mut thread_rng());
    let sec1 = derive_shared_secret_from_slice(pk2.serialize(), sk1.secret_bytes())?;
    let sec2 = derive_shared_secret_from_slice(pk1.serialize(), sk2.secret_bytes())?;
    assert_eq!(sec1, sec2);
    Ok(())
  }

}