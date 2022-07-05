#![feature(once_cell)]
mod conn;
mod core;
mod periph;

use crate::core::{config::*, events::*};
use crate::periph::led::led_control_loop;
use crate::periph::sd::sd_card;

use anyhow::Result;
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use embedded_svc::storage::Storage;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::nvs::*;
use esp_idf_svc::nvs_storage::EspNvsStorage;

use sphinx_key_signer::lightning_signer::bitcoin::Network;

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

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;

    let (led_tx, led_rx) = mpsc::channel();
    // LED control thread
    led_control_loop(pins.gpio8, peripherals.rmt.channel0, led_rx);

    // sd card
    sd_card(peripherals.spi2);

    let default_nvs = Arc::new(EspDefaultNvs::new()?);
    let mut store =
        EspNvsStorage::new_default(default_nvs.clone(), "sphinx", true).expect("no storage");
    let existing: Option<Config> = store.get("config").expect("failed");
    if let Some(exist) = existing {
        println!(
            "=============> START CLIENT NOW <============== {:?}",
            exist
        );
        // store.remove("config").expect("couldnt remove config");
        led_tx.send(Status::ConnectingToWifi).unwrap();
        let _wifi = start_wifi_client(default_nvs.clone(), &exist)?;

        let (tx, rx) = mpsc::channel();

        led_tx.send(Status::ConnectingToMqtt).unwrap();
        // _conn needs to stay in scope or its dropped
        let (mqtt, connection) = conn::mqtt::make_client(&exist.broker, CLIENT_ID)?;
        let mqtt_client = conn::mqtt::start_listening(mqtt, connection, tx)?;
        // this blocks forever... the "main thread"
        let do_log = true;
        let network = match exist.network.as_str() {
            "bitcoin" => Network::Bitcoin,
            "mainnet" => Network::Bitcoin,
            "testnet" => Network::Testnet,
            "signet" => Network::Signet,
            "regtest" => Network::Regtest,
            _ => Network::Regtest,
        };
        log::info!("Network set to {:?}", network);
        log::info!(">>>>>>>>>>> blocking forever...");
        make_event_loop(mqtt_client, rx, network, do_log, led_tx, exist.seed)?;
    } else {
        led_tx.send(Status::WifiAccessPoint).unwrap();
        println!("=============> START SERVER NOW AND WAIT <==============");
        if let Ok((wifi, config)) = start_config_server_and_wait(default_nvs.clone()) {
            store
                .put("config", &config)
                .expect("could not store config");
            println!("CONFIG SAVED");
            drop(wifi);
            thread::sleep(Duration::from_secs(1));
        }
    }

    Ok(())
}
