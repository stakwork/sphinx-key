#![feature(once_cell)]
mod conn;
mod core;
mod periph;

use crate::core::{events::*, config::*};
use crate::periph::led::Led;

use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use std::thread;
use std::sync::{Arc, mpsc};
use std::time::Duration;
use anyhow::Result;

use esp_idf_svc::nvs::*;
use esp_idf_svc::nvs_storage::EspNvsStorage;
use embedded_svc::storage::Storage;
use embedded_svc::wifi::Wifi;

#[cfg(not(feature = "pingpong"))]
const CLIENT_ID: &str = "sphinx-1";

#[cfg(feature = "pingpong")]
const CLIENT_ID: &str = "test-1";

fn main() -> Result<()> {

    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    thread::sleep(Duration::from_secs(1));

    let default_nvs = Arc::new(EspDefaultNvs::new()?);
    let mut store = EspNvsStorage::new_default(default_nvs.clone(), "sphinx", true).expect("no storage");
    let existing: Option<Config> = store.get("config").expect("failed");
    if let Some(exist) = existing {
        println!("=============> START CLIENT NOW <============== {:?}", exist);
        // store.remove("config").expect("couldnt remove config");
        let wifi = start_wifi_client(default_nvs.clone(), &exist)?;

        let (tx, rx) = mpsc::channel();

        // _conn needs to stay in scope or its dropped
        let (mqtt, connection) = conn::mqtt::make_client(&exist.broker, CLIENT_ID)?;
        let mqtt_client = conn::mqtt::start_listening(mqtt, connection, tx)?;
        
        // this blocks forever... the "main thread"
        log::info!(">>>>>>>>>>> blocking forever...");
        make_event_loop(mqtt_client, rx)?;
        
        let mut blue = Led::new(0x000001, 100);
        println!("{:?}", wifi.get_status());
        loop {
            log::info!("Listening...");
            blue.blink();
            thread::sleep(Duration::from_secs(1));
        }
        // drop(wifi);
    } else {
        println!("=============> START SERVER NOW AND WAIT <==============");
        if let Ok((wifi, config)) = start_config_server_and_wait(default_nvs.clone()) {
            store.put("config", &config).expect("could not store config");
            println!("CONFIG SAVED");
            drop(wifi);
            thread::sleep(Duration::from_secs(1));
        }
    }

    Ok(())
}
