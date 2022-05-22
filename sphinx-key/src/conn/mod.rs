mod html;
pub mod wifi;

use url;
use embedded_svc::httpd::*;
use esp_idf_svc::httpd as idf;
use std::sync::{Condvar, Mutex, Arc};
use embedded_svc::httpd::registry::Registry;
use esp_idf_sys::{self};
use esp_idf_svc::nvs::*;
use esp_idf_svc::nvs_storage::EspNvsStorage;
use embedded_svc::storage::Storage;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Params {
    pub config: String
}
#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub broker: String,
    pub ssid: String,
    pub pass: String,
}

/*
curl -X POST 192.168.71.1/config?broker=52.91.253.115%3A1883&ssid=apples%26acorns&pass=42flutes

curl -X POST 192.168.71.1/config?ssid=apples%26acorns&pass=42flutes&broker=52.91.253.115%3A1883

curl -X POST 192.168.71.1/config?broker=52.91.253.115%3A1883

curl -X POST 192.168.71.1/config?config=%7B%22ssid%22%3A%22apples%26acorns%22%2C%22pass%22%3A%2242flutes%22%2C%22broker%22%3A%2252.91.253.115%3A1883%22%7D
*/

#[allow(unused_variables)]
pub fn config_server(mutex: Arc<(Mutex<Option<Config>>, Condvar)>, store: Arc<Mutex<EspNvsStorage>>) -> Result<idf::Server> {

    let server = idf::ServerRegistry::new()
        .at("/")
        .get(|_| Ok(html::HTML.into()))?
        .at("/config")
        .post(move |mut request| {
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