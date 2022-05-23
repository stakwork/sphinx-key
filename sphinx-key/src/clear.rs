#![allow(unused_imports)]

use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

use esp_idf_svc::nvs::*;
use esp_idf_svc::nvs_storage::EspNvsStorage;

use embedded_svc::httpd::*;
use embedded_svc::wifi::*;
use embedded_svc::storage::Storage;

use std::sync::Arc;

fn main() -> Result<()> {
    let default_nvs = Arc::new(EspDefaultNvs::new()?);
    let mut store = EspNvsStorage::new_default(default_nvs.clone(), "sphinx", true).expect("no storage");
    store.remove("config").expect("couldnt remove config 1");
    println!("NVS cleared!");
    Ok(())
}