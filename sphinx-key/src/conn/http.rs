use crate::conn::{Params, Config};
use crate::conn::html;

use url;
use embedded_svc::httpd::*;
use esp_idf_svc::httpd as idf;
use std::sync::{Condvar, Mutex, Arc};
use embedded_svc::httpd::registry::Registry;
use esp_idf_sys::{self};

#[allow(unused_variables)]
pub fn config_server(mutex: Arc<(Mutex<Option<Config>>, Condvar)>) -> Result<idf::Server> {

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