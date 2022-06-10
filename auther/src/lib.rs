use anyhow::anyhow;
use anyhow::Result;
use base64::{decode_config, encode_config, URL_SAFE};
use secp256k1::ecdsa::Signature;
use secp256k1::hashes::sha256d::Hash as Sha256dHash;
use secp256k1::hashes::Hash;
use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
use std::convert::TryInto;

pub struct Token(u64);

fn now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    since_the_epoch.as_nanos() as u64
}

pub fn u64_to_bytes(input: u64) -> [u8; 8] {
    input.to_le_bytes()
}
pub fn bytes_to_u64(bytes: [u8; 8]) -> u64 {
    u64::from_le_bytes(bytes)
}

pub fn base64_encode(input: &Vec<u8>) -> String {
    encode_config(input, URL_SAFE)
}
pub fn base64_decode(input: &str) -> Result<Vec<u8>> {
    let r = decode_config(input, URL_SAFE)?;
    Ok(r)
}

impl Token {
    pub fn new() -> Self {
        Self(now())
    }
    pub fn from_bytes(bytes: [u8; 8]) -> Self {
        Self(bytes_to_u64(bytes))
    }
    pub fn from_base64(s: &str) -> Result<Self> {
        if s.len() < 8 {
            return Err(anyhow!("too short slice".to_string()));
        }
        let bytes = base64_decode(s)?;
        let ts: [u8; 8] = bytes[..8].try_into()?;
        Ok(Self(bytes_to_u64(ts)))
    }
    pub fn expected_len(&self) -> usize {
        73
    }
    pub fn sign(&self, secret_key: &SecretKey) -> Result<Vec<u8>> {
        let mut ts = u64_to_bytes(self.0).to_vec();
        let sig = self.sign_message(&ts, secret_key)?;
        println!("tts {:?}", ts);
        ts.extend(sig);
        assert_eq!(ts.len(), self.expected_len());
        Ok(ts)
    }
    pub fn verify(&self, sig: Vec<u8>, public_key: &PublicKey) -> Result<()> {
        // remove ts
        // let (msg, sig) = input.split_at(8);
        let msg = u64_to_bytes(self.0);
        self.verify_message(&msg.to_vec(), &sig.to_vec(), public_key)
    }
    /// Sign a Lightning message
    pub fn sign_message(&self, message: &Vec<u8>, secret_key: &SecretKey) -> Result<Vec<u8>> {
        let encmsg = self.lightning_hash(message)?;
        let secp_ctx = Secp256k1::signing_only();
        let sig = secp_ctx.sign_ecdsa_recoverable(&encmsg, &secret_key);
        let (rid, sig) = sig.serialize_compact();
        let mut res = sig.to_vec();
        res.push(rid.to_i32() as u8);
        Ok(res)
    }
    /// Verify a Lightning message
    pub fn verify_message(
        &self,
        message: &Vec<u8>,
        sig: &Vec<u8>,
        public_key: &PublicKey,
    ) -> Result<()> {
        let secp_ctx = Secp256k1::verification_only();
        let encmsg = self.lightning_hash(message)?;
        // remove the rid
        let s = Signature::from_compact(&sig[..64])?;
        secp_ctx.verify_ecdsa(&encmsg, &s, public_key)?;
        Ok(())
    }
    /// hash lightning message
    pub fn lightning_hash(&self, message: &Vec<u8>) -> Result<Message> {
        let mut buffer = String::from("Lightning Signed Message:").into_bytes();
        buffer.extend(message);
        let hash = Sha256dHash::hash(&buffer);
        let encmsg = secp256k1::Message::from_slice(&hash[..])?;
        Ok(encmsg)
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

    #[test]
    fn test_token() {
        let sk = secret_key();
        let t = Token::new();
        let res = t.sign(&sk).expect("couldnt make token");
        let secp = Secp256k1::new();
        let public_key = PublicKey::from_secret_key(&secp, &sk);
        let (_, sig) = res.split_at(8);
        t.verify(sig.to_vec(), &public_key).expect("couldnt verify");
        println!("token verified!");
    }

    #[test]
    fn test_decode() {
        let sk = secret_key();
        let secp = Secp256k1::new();
        let public_key = PublicKey::from_secret_key(&secp, &sk);
        let s = "aHt45kxY9xZCMvT5du5mTw-jx3X2g0Eg7QhHTIi6rBRAFqY_syx1SzcSoriXyPIVCWdG6T0I8xKXSEnoeajFdwmtQbHqC_qfAQ==";
        let v = base64_decode(s).expect("couldnt decode");
        let (_, sig) = v.split_at(8);
        let t = Token::from_base64(s).expect("couldnt parse base64");
        t.verify(sig.to_vec(), &public_key)
            .expect("failed to verify");
        println!("decoded token verified!");
    }
}
