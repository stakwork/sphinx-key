use sphinx_auther as auther;
use sphinx_auther::secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
pub struct Validator(PublicKey);

const SIG_LEN: usize = 65;

impl Validator {
    fn new(pk: PublicKey) -> Self {
        Self(pk)
    }
    fn parse_control_message(&self, mut input: Vec<u8>) -> anyhow::Result<()> {
        let arr = input.split_at(input.len() - SIG_LEN);
        let sig: [u8; SIG_LEN] = arr.1.try_into().unwrap();
        auther::verify_message(arr.0, &sig, &self.0)?;
        Ok(())
    }
}
