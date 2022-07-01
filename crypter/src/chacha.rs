use anyhow::anyhow;
use lightning::util::chacha20poly1305rfc::ChaCha20Poly1305RFC;

pub const MSG_LEN: usize = 32;
pub const KEY_LEN: usize = 32;
pub const NONCE_END_LEN: usize = 8;
pub const TAG_LEN: usize = 16;
pub const CIPHER_LEN: usize = MSG_LEN + NONCE_END_LEN + TAG_LEN;

pub fn encrypt(
  plaintext: [u8; MSG_LEN],
  key: [u8; KEY_LEN],
  nonce_end: [u8; NONCE_END_LEN],
) -> anyhow::Result<[u8; CIPHER_LEN]> {
  let mut nonce = [0; 4 + NONCE_END_LEN];
  nonce[4..].copy_from_slice(&nonce_end);
  let mut chacha = ChaCha20Poly1305RFC::new(&key, &nonce, &[0; 0]);
  let mut res = [0; MSG_LEN];
  let mut tag = [0; TAG_LEN];
  chacha.encrypt(&plaintext[..], &mut res[0..plaintext.len()], &mut tag);
  let mut ret = [0; CIPHER_LEN];
  ret[..MSG_LEN].copy_from_slice(&res);
  ret[MSG_LEN..MSG_LEN + NONCE_END_LEN].copy_from_slice(&nonce_end);
  ret[MSG_LEN + NONCE_END_LEN..].copy_from_slice(&tag);
  Ok(ret)
}

pub fn decrypt(ciphertext: [u8; CIPHER_LEN], key: [u8; KEY_LEN]) -> anyhow::Result<[u8; MSG_LEN]> {
  let mut nonce = [0; 4 + NONCE_END_LEN];
  nonce[4..].copy_from_slice(&ciphertext[MSG_LEN..MSG_LEN + NONCE_END_LEN]);
  let mut tag = [0; TAG_LEN];
  tag.copy_from_slice(&ciphertext[MSG_LEN + NONCE_END_LEN..]);
  let mut chacha2 = ChaCha20Poly1305RFC::new(&key, &nonce, &[0; 0]);
  let mut dec = [0; MSG_LEN];
  let ok = chacha2.decrypt(&ciphertext[..MSG_LEN], &mut dec, &tag);
  if ok {
    Ok(dec)
  } else {
    Err(anyhow!("failed chacha authentication"))
  }
}

#[cfg(test)]
mod tests {
  use crate::chacha::{decrypt, encrypt, KEY_LEN, MSG_LEN, NONCE_END_LEN};
  use rand::{rngs::OsRng, RngCore};

  #[test]
  fn test_chacha() -> anyhow::Result<()> {
    let key = [9; KEY_LEN];
    let plaintext = [1; MSG_LEN];
    let mut nonce_end = [0; NONCE_END_LEN];
    OsRng.fill_bytes(&mut nonce_end);
    let cipher = encrypt(plaintext, key, nonce_end)?;
    let plain = decrypt(cipher, key)?;
    assert_eq!(plaintext, plain);
    Ok(())
  }
}
