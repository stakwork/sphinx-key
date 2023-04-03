use crate::core::config::{decrypt_seed, ecdh_keypair, ConfigDTO};
use sphinx_signer::sphinx_glyph::control::Config;

use serde::Deserialize;
use std::sync::{Arc, Condvar, Mutex};

// use embedded_svc::http::server::registry::Registry;
// use embedded_svc::http::server::*;
#[allow(deprecated)]
use embedded_svc::httpd::registry::Registry;
use embedded_svc::httpd::Result;

use esp_idf_svc::httpd as idf;

#[derive(Clone, Debug, Deserialize)]
pub struct Ecdh {
    pub pubkey: String,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Params {
    pub config: String,
}

#[allow(unused_variables, deprecated)]
pub fn config_server(
    mutex: Arc<(Mutex<Option<(Config, [u8; 32])>>, Condvar)>,
) -> Result<idf::Server> {
    let (sk1, pk1) = ecdh_keypair();

    let server = idf::ServerRegistry::new()
        .at("/ecdh")
        .get(move |_| {
            Ok(
                format!("{{\"pubkey\":\"{}\"}}", hex::encode(pk1.serialize()))
                    .to_owned()
                    .into(),
            )
        })?
        .at("/config")
        .post(move |request| {
            let bod = &request
                .query_string()
                .ok_or(anyhow::anyhow!("failed to parse query string"))?;
            println!("bod {:?}", bod);
            let params = serde_urlencoded::from_str::<Params>(bod)?;

            let dto = serde_json::from_str::<ConfigDTO>(&params.config)?;

            let conf_seed_tuple = decrypt_seed(dto, sk1)?;

            let mut wait = mutex.0.lock().unwrap();
            *wait = Some(conf_seed_tuple);
            mutex.1.notify_one();
            Ok("{\"success\":true}".to_owned().into())
        })?;

    server.start(&Default::default())
}
