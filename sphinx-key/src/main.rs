#![allow(unused_imports)]

mod srv;

use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use sphinx_key_signer;
use std::sync::{Condvar, Mutex, Arc};
use embedded_svc::httpd::*;
// use log::*;
// use url;

fn main() -> Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    println!("Hello, world!");

    sphinx_key_signer::say_hi();

    let mutex = Arc::new((Mutex::new(None), Condvar::new()));

    let _httpd = srv::httpd(mutex.clone());

    /* shutdown */
    // drop(httpd);
    // info!("Httpd stopped");

    Ok(())
}
