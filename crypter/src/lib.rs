pub mod chacha;
pub mod ecdh;

pub use secp256k1;

#[cfg(test)]
mod tests {
  use crate::chacha::{decrypt, encrypt, MSG_LEN, NONCE_END_LEN};
  use crate::ecdh::derive_shared_secret_from_slice;
  use secp256k1::rand::{rngs::OsRng, thread_rng, RngCore};
  use secp256k1::Secp256k1;

  #[test]
  fn test_crypter() -> anyhow::Result<()> {
    // two keypairs
    let s = Secp256k1::new();
    let (sk1, pk1) = s.generate_keypair(&mut thread_rng());
    let (sk2, pk2) = s.generate_keypair(&mut thread_rng());

    // derive shared secrets
    let sec1 = derive_shared_secret_from_slice(pk2.serialize(), sk1.secret_bytes())?;
    let sec2 = derive_shared_secret_from_slice(pk1.serialize(), sk2.secret_bytes())?;
    assert_eq!(sec1, sec2);

    // encrypt plaintext with sec1
    let plaintext = [1; MSG_LEN];
    let mut nonce_end = [0; NONCE_END_LEN];
    OsRng.fill_bytes(&mut nonce_end);
    let cipher = encrypt(plaintext, sec1, nonce_end)?;

    // decrypt with sec2
    let plain = decrypt(cipher, sec2)?;
    assert_eq!(plaintext, plain);

    println!("PLAINTEXT MATCHES!");
    Ok(())
  }
}
