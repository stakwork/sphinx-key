use secp256k1::ecdsa::Signature;
use secp256k1::hashes::sha256d::Hash as Sha256dHash;
use secp256k1::hashes::Hash;
use secp256k1::{Message, Secp256k1, SecretKey};

pub struct Token(u64);

impl Token {
    pub fn new() -> Self {
        Self(0)
    }
    /// Sign a Lightning message
    pub fn sign_message(
        &self,
        message: &Vec<u8>,
        secret_key: &SecretKey,
    ) -> anyhow::Result<Vec<u8>> {
        let mut buffer = String::from("Lightning Signed Message:").into_bytes();
        buffer.extend(message);
        let secp_ctx = Secp256k1::signing_only();
        let hash = Sha256dHash::hash(&buffer);
        let encmsg = secp256k1::Message::from_slice(&hash[..])?;
        let sig = secp_ctx.sign_ecdsa_recoverable(&encmsg, &secret_key);
        let (rid, sig) = sig.serialize_compact();
        let mut res = sig.to_vec();
        res.push(rid.to_i32() as u8);
        Ok(res)
    }
}

pub fn sign<T: secp256k1::Signing>(
    secp: &Secp256k1<T>,
    input: Vec<u8>,
    secret_key: &SecretKey,
) -> Signature {
    let message = hash_message(input);
    secp.sign_ecdsa(&message, &secret_key)
}

pub fn hash_message(input: Vec<u8>) -> Message {
    let hash = Sha256dHash::hash(&input);
    Message::from_slice(&hash[..]).expect("encmsg failed")
}

#[cfg(test)]
mod tests {
    use crate::*;
    use secp256k1::{PublicKey, Secp256k1, SecretKey};

    fn secret_key() -> SecretKey {
        SecretKey::from_slice(&[0xcd; 32]).expect("32 bytes, within curve order")
    }

    #[test]
    fn test_sign() {
        let secp = Secp256k1::new();
        let sk = secret_key();
        let public_key = PublicKey::from_secret_key(&secp, &sk);
        let input = vec![1, 2, 3];
        let message = hash_message(input);
        let sig = sign(&secp, vec![1, 2, 3], &sk);
        assert!(secp.verify_ecdsa(&message, &sig, &public_key).is_ok());
    }
}
