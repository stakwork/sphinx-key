use lightning::util::chacha20poly1305rfc::ChaCha20Poly1305RFC;

pub fn test_chacha20poly1305() {

    let key = [0; 32];
    // 32 bytes key
    // 12 byte nonce
    let n: u64 = 123456;
    let mut nonce = [0; 12];
    println!("chacha1");
	nonce[4..].copy_from_slice(&n.to_le_bytes()[..]);
    println!("chacha2");
    let mut chacha = ChaCha20Poly1305RFC::new(&key, &nonce, &[0; 0]);
    println!("chacha3");
    let mut tag = [0; 16];
    let plaintext = b"plaintext";
    let mut res = [0; 50];
    chacha.encrypt(plaintext, &mut res[0..plaintext.len()], &mut tag);
    println!("chacha4 {:?}", res);
    println!("tag {:?}", tag);

    let mut chacha2 = ChaCha20Poly1305RFC::new(&key, &nonce, &[0; 0]);
    let mut dec = [0; 9];
    let ok = chacha2.decrypt(&res[0..9], &mut dec, &tag);

    println!("ok {}", ok);
    println!("dec {:?}", dec);
    println!("decrypted: {}", String::from_utf8_lossy(&dec[..]));
}

#[cfg(test)]
mod tests {
  use crate::test_chacha20poly1305;

  #[test]
  fn find_solution_1_btc() {
    test_chacha20poly1305();
  }
}