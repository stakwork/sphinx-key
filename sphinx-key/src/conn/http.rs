use crate::core::config::{decrypt_seed, ecdh_keypair, ConfigDTO};
use anyhow::Result;
use esp_idf_svc::http::server::HandlerError;
use esp_idf_svc::http::server::{Configuration, EspHttpServer};
use esp_idf_svc::http::Method;
use serde::Deserialize;
use sphinx_signer::sphinx_glyph::control::Config;
use std::sync::{Arc, Condvar, Mutex};

#[derive(Clone, Debug, Deserialize)]
pub struct Ecdh {
    pub pubkey: String,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Params {
    pub config: String,
}

pub fn config_server(
    mutex: Arc<(Mutex<Option<(Config, Option<[u8; 32]>)>>, Condvar)>,
    has_stored_seed: bool,
) -> Result<EspHttpServer<'static>> {
    let (sk1, pk1) = ecdh_keypair();

    let mut server = EspHttpServer::new(&Configuration::default()).unwrap();
    server
        .fn_handler("/ecdh", Method::Get, move |request| {
            let mut response = request.into_ok_response()?;
            response.write(
                &format!("{{\"pubkey\":\"{}\"}}", hex::encode(pk1.serialize())).into_bytes(),
            )?;
            response.flush()?;
            Ok(())
        })?
        .fn_handler("/config", Method::Post, move |request| {
            let params =
                serde_urlencoded::from_str::<Params>(request.uri().split_once("?").unwrap().1)?;
            let dto = serde_json::from_str::<ConfigDTO>(&params.config)?;
            let conf_seed_tuple = decrypt_seed(dto, sk1)?;
            if !has_stored_seed && conf_seed_tuple.1.is_none() {
                return Err(HandlerError::new("seed required"));
            }
            let mut wait = mutex.0.lock().unwrap();
            *wait = Some(conf_seed_tuple);
            mutex.1.notify_one();
            let mut response = request.into_ok_response()?;
            response.write("{\"success\":true}".as_bytes())?;
            response.flush()?;
            Ok(())
        })?;
    Ok(server)
}
