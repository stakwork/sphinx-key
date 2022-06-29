use crate::conn::html;
use crate::core::config::Config;

use embedded_svc::httpd::*;
use esp_idf_svc::httpd as idf;
use std::sync::{Condvar, Mutex, Arc};
use embedded_svc::httpd::registry::Registry;
use serde::Deserialize;

use rsa::{PublicKey, RsaPrivateKey, RsaPublicKey, PaddingScheme};
use rsa::pkcs8::EncodePublicKey;

#[derive(Clone, Debug, Deserialize)]
pub struct Params {
    pub config: String
}

#[allow(unused_variables)]
pub fn config_server(mutex: Arc<(Mutex<Option<Config>>, Condvar)>) -> Result<idf::Server> {

    let mut rng = rand::thread_rng();
    let bits = 2048;
    let priv_key = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key");
    let pub_key = RsaPublicKey::from(&priv_key);
    let der = pub_key.to_public_key_der().expect("could not encode DER");
    let hexder = der.into_vec();

    let server = idf::ServerRegistry::new()
        .at("/")
        .get(|_| Ok(html::HTML.into()))?
        .at("/pubkey")
        .get(move |_| Ok(hex::encode(hexder.clone()).into()))?
        .at("/config")
        .post(move |request| {
            let bod = &request.query_string()
                .ok_or(anyhow::anyhow!("failed to parse query string"))?;
            println!("bod {:?}", bod);
            let params = serde_urlencoded::from_str::<Params>(bod)?;

            let conf = serde_json::from_str::<Config>(&params.config)?;
            
            let mut wait = mutex.0.lock().unwrap();
            *wait = Some(conf);
            mutex.1.notify_one();
            Ok("{\"success\":true}".to_owned().into())
        })?;

    server.start(&Default::default())
}