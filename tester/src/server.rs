#[macro_use]
extern crate rocket;

use rocket::State;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

use sphinx_key_crypter::chacha::{decrypt, CIPHER_LEN};
use sphinx_key_crypter::ecdh::{derive_shared_secret_from_slice, PUBLIC_KEY_LEN};
use sphinx_key_crypter::secp256k1::rand::thread_rng;
use sphinx_key_crypter::secp256k1::Secp256k1;
use sphinx_key_crypter::secp256k1::{PublicKey, SecretKey};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigBody {
    pub seed: String,
    pub ssid: String,
    pub pass: String,
    pub broker: String,
    pub pubkey: String, // for ecdh
    pub network: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub broker: String,
    pub ssid: String,
    pub pass: String,
    pub seed: [u8; 32],
    pub network: String,
}

struct Keys {
    sk: SecretKey,
    pk: PublicKey,
}

#[get("/ecdh")]
fn ecdh(keys: &State<Keys>) -> String {
    format!("{{\"pubkey\":\"{}\"}}", hex::encode(keys.pk.serialize()))
}

#[post("/config?<config>")]
fn config(keys: &State<Keys>, config: &str) -> String {
    // println!("============> {:?}", config);
    let dto = serde_json::from_str::<ConfigBody>(&config).expect("failed to parse");
    let conf = decrypt_seed(dto, keys.sk).expect("couldnt decrypt seed");
    println!("SEED: ===========> {:?}", hex::encode(conf.seed));
    "{\"success\":true}".to_string()
}

pub fn decrypt_seed(dto: ConfigBody, sk1: SecretKey) -> anyhow::Result<Config> {
    let their_pk = hex::decode(dto.pubkey)?;
    let their_pk_bytes: [u8; PUBLIC_KEY_LEN] = their_pk[..PUBLIC_KEY_LEN].try_into()?;
    let shared_secret = derive_shared_secret_from_slice(their_pk_bytes, sk1.secret_bytes())?;
    // decrypt seed
    let cipher_seed = hex::decode(dto.seed)?;
    let cipher: [u8; CIPHER_LEN] = cipher_seed[..CIPHER_LEN].try_into()?;
    let seed = decrypt(cipher, shared_secret)?;

    Ok(Config {
        broker: dto.broker,
        ssid: dto.ssid,
        pass: dto.pass,
        network: dto.network,
        seed: seed,
    })
}

#[launch]
fn rocket() -> _ {
    let s = Secp256k1::new();
    let (sk, pk) = s.generate_keypair(&mut thread_rng());
    rocket::build()
        .mount("/", routes![ecdh, config])
        .manage(Keys { sk, pk })
}
