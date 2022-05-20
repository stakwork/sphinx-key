mod html;

use embedded_svc::httpd::*;
use esp_idf_svc::httpd as idf;
use std::sync::{Condvar, Mutex, Arc};
use embedded_svc::httpd::registry::Registry;
use esp_idf_sys::{self};

#[allow(unused_variables)]
pub fn httpd(mutex: Arc<(Mutex<Option<u32>>, Condvar)>) -> Result<idf::Server> {

    let server = idf::ServerRegistry::new()
        .at("/")
        .get(|_| {
            Ok(html::HTML.into())
        })?;

    server.start(&Default::default())
}