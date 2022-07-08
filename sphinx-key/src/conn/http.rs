use crate::conn::html;
use crate::core::config::{Config, ConfigDTO, ecdh_keypair, decrypt_seed};

use serde::Deserialize;
use std::sync::{Arc, Condvar, Mutex};

use embedded_svc::httpd::registry::Registry;
use embedded_svc::httpd::*;
use esp_idf_svc::httpd as idf;

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
    
    let (sk1, pk1) = ecdh_keypair();

    let server = idf::ServerRegistry::new()
        .at("/")
        .get(|_| Ok(html::HTML.into()))?
        .at("/ecdh")
        .get(move |_| Ok(format!("{{\"pubkey\":\"{}\"}}",  hex::encode(pk1.serialize())).to_owned().into()))?
        .at("/config")
        .post(move |request| {
            let bod = &request
                .query_string()
                .ok_or(anyhow::anyhow!("failed to parse query string"))?;
            println!("bod {:?}", bod);
            let params = serde_urlencoded::from_str::<Params>(bod)?;

            let dto = serde_json::from_str::<ConfigDTO>(&params.config)?;

            let conf = decrypt_seed(dto, sk1)?;
            
            let mut wait = mutex.0.lock().unwrap();
            *wait = Some(conf);
            mutex.1.notify_one();
            Ok("{\"success\":true}".to_owned().into())
        })?;

    server.start(&Default::default())
}
