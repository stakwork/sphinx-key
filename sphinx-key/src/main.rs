
mod conn;
mod core;

use crate::core::{events::*, config::*};

use sphinx_key_signer;
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use std::thread;
use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;

use esp_idf_svc::nvs::*;
use esp_idf_svc::nvs_storage::EspNvsStorage;
use embedded_svc::storage::Storage;
use embedded_svc::wifi::Wifi;

fn main() -> Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    sphinx_key_signer::say_hi();

    thread::sleep(Duration::from_secs(1));

    let default_nvs = Arc::new(EspDefaultNvs::new()?);
    let mut store = EspNvsStorage::new_default(default_nvs.clone(), "sphinx", true).expect("no storage");
    let existing: Option<Config> = store.get("config").expect("failed");
    if let Some(exist) = existing {
        println!("=============> START CLIENT NOW <============== {:?}", exist);
        // store.remove("config").expect("couldnt remove config");
        let wifi = start_client(default_nvs.clone(), &exist)?;

        let mqtt = conn::mqtt::make_client(&exist.broker)?;
        // if the subscription goes out of scope its dropped
        // the sub needs to publish back to mqtt???
        let (eventloop, _sub) = make_eventloop(&mqtt)?;
        let mqtt_client = conn::mqtt::mqtt_client(&exist.broker, eventloop)?;
       
        println!("{:?}", wifi.get_status());
        for s in 0..60 {
            log::info!("Shutting down in {} secs", 60 - s);
            thread::sleep(Duration::from_secs(1));
        }
        drop(wifi);
    } else {
        println!("=============> START SERVER NOW AND WAIT <==============");
        if let Ok((wifi, config)) = start_server_and_wait(default_nvs.clone()) {
            store.put("config", &config).expect("could not store config");
            println!("CONFIG SAVED");
            drop(wifi);
            thread::sleep(Duration::from_secs(1));
        }
    }

    Ok(())
}
