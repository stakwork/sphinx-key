use crate::conn::html;
use crate::core::config::{Config, ConfigDTO};

use serde::Deserialize;
use std::convert::TryInto;
use std::sync::{Arc, Condvar, Mutex};

use embedded_svc::httpd::registry::Registry;
use embedded_svc::httpd::*;
use esp_idf_svc::httpd as idf;

use sphinx_key_crypter::chacha::{decrypt, CIPHER_LEN};
use sphinx_key_crypter::ecdh::{derive_shared_secret_from_slice, PUBLIC_KEY_LEN};
use sphinx_key_crypter::secp256k1::rand::thread_rng;
use sphinx_key_crypter::secp256k1::Secp256k1;

#[derive(Clone, Debug, Deserialize)]
pub struct Ecdh {
    pub pubkey: String,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Params {
    pub config: String,
}

#[allow(unused_variables)]
pub fn config_server(mutex: Arc<(Mutex<Option<Config>>, Condvar)>) -> Result<idf::Server> {
    let s = Secp256k1::new();
    let (sk1, pk1) = s.generate_keypair(&mut thread_rng());
    let pk_hex = hex::encode(pk1.serialize());

    let server = idf::ServerRegistry::new()
        .at("/")
        .get(|_| Ok(html::HTML.into()))?
        .at("/ecdh")
        .get(move |_| Ok(format!("{{\"pubkey\":\"{}\"}}", pk_hex).to_owned().into()))?
        .at("/config")
        .post(move |request| {
            let bod = &request
                .query_string()
                .ok_or(anyhow::anyhow!("failed to parse query string"))?;
            println!("bod {:?}", bod);
            let params = serde_urlencoded::from_str::<Params>(bod)?;

            let dto = serde_json::from_str::<ConfigDTO>(&params.config)?;

            let their_pk = hex::decode(dto.pubkey)?;
            let their_pk_bytes: [u8; PUBLIC_KEY_LEN] = their_pk[..PUBLIC_KEY_LEN].try_into()?;
            let shared_secret =
                derive_shared_secret_from_slice(their_pk_bytes, sk1.secret_bytes())?;
            // decrypt seed
            let cipher_seed = hex::decode(dto.seed)?;
            let cipher: [u8; CIPHER_LEN] = cipher_seed[..CIPHER_LEN].try_into()?;
            let seed = decrypt(cipher, shared_secret)?;

            let conf = Config {
                broker: dto.broker,
                ssid: dto.ssid,
                pass: dto.pass,
                network: dto.network,
                seed: seed,
            };
            let mut wait = mutex.0.lock().unwrap();
            *wait = Some(conf);
            mutex.1.notify_one();
            Ok("{\"success\":true}".to_owned().into())
        })?;

    server.start(&Default::default())
}
