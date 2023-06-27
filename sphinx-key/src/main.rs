#![feature(once_cell)]
mod conn;
mod core;
mod ota;
mod periph;
mod status;

use crate::core::control::{controller_from_seed, FlashPersister};
use crate::core::{config::*, events::*};
use crate::status::Status;
// use crate::periph::led::led_control_loop;
use crate::periph::button::button_loop;
#[allow(unused_imports)]
use crate::periph::sd::{mount_sd_card, simple_fs_test};

use anyhow::Result;
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::SystemTime;

use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::nvs::*;

use sphinx_signer::lightning_signer::bitcoin::Network;
use sphinx_signer::sphinx_glyph::control::{Config, ControlPersist, Policy, Velocity};

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

    let (led_tx, _led_rx) = mpsc::channel();
    // LED control thread
    // led_control_loop(pins.gpio0, peripherals.rmt.channel0, led_rx);

    // BUTTON thread
    // button_loop(pins.gpio8, led_tx.clone());

    // thread::sleep(Duration::from_secs(100));

    led_tx.send(Status::MountingSDCard).unwrap();
    println!("About to mount the sdcard...");
    while let Err(_e) = mount_sd_card() {
        println!("Failed to mount sd card. Make sure it is connected, trying again...");
        thread::sleep(Duration::from_secs(5));
    }
    println!("SD card mounted!");

    // let default_nav_partition = EspDefaultNvs.take().unwrap();
    let default_nvs = EspDefaultNvsPartition::take()?;
    // let default_nvs = Arc::new();
    let mut flash = FlashPersister::new(default_nvs.clone());
    if let Ok(exist) = flash.read_config() {
        let seed = flash.read_seed().expect("no seed...");
        let policy = flash.read_policy().unwrap_or_default();
        let velocity = flash.read_velocity().ok();
        println!(
            "=============> START CLIENT NOW <============== {:?}",
            exist
        );
        led_tx.send(Status::ConnectingToWifi).unwrap();
        let wifi_ = start_wifi_client(peripherals.modem, default_nvs.clone(), &exist)?;
        // let _wifi = loop {
        //     if let Ok(wifi) = start_wifi_client(peripherals.modem, default_nvs.clone(), &exist) {
        //         println!("Wifi connected!");
        //         break wifi;
        //     } else {
        //         println!("Failed to connect to wifi. Make sure the details are correct, trying again in 5 seconds...");
        //         thread::sleep(Duration::from_secs(5));
        //     }
        // };

        led_tx.send(Status::SyncingTime).unwrap();
        conn::sntp::sync_time();
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        println!(
            "Completed the time sync, here is the UNIX time: {}",
            now.as_secs(),
        );

        led_tx.send(Status::ConnectingToMqtt).unwrap();

        let flash_arc = Arc::new(Mutex::new(flash));
        loop {
            if let Ok(()) = make_and_launch_client(
                exist.clone(),
                seed,
                &policy,
                &velocity,
                led_tx.clone(),
                flash_arc.clone(),
            ) {
                println!("Exited out of the event loop, trying again in 5 seconds...");
                thread::sleep(Duration::from_secs(5));
            } else {
                println!("Failed to setup MQTT. Make sure the details are correct, trying again in 5 seconds...");
                thread::sleep(Duration::from_secs(5));
            }
        }
    } else {
        led_tx.send(Status::WifiAccessPoint).unwrap();
        println!("=============> START SERVER NOW AND WAIT <==============");
        match start_config_server_and_wait(peripherals.modem, default_nvs.clone()) {
            Ok((_wifi, config, seed)) => {
                flash.write_config(config).expect("could not store config");
                flash.write_seed(seed).expect("could not store seed");
                println!("CONFIG SAVED");
                unsafe { esp_idf_sys::esp_restart() };
            }
            Err(msg) => log::error!("{}", msg),
        }
    }

    Ok(())
}

fn make_and_launch_client(
    config: Config,
    seed: [u8; 32],
    policy: &Policy,
    velocity: &Option<Velocity>,
    led_tx: mpsc::Sender<Status>,
    flash: Arc<Mutex<FlashPersister>>,
) -> anyhow::Result<()> {
    let (tx, rx) = mpsc::channel();

    let network = match config.network.as_str() {
        "bitcoin" => Network::Bitcoin,
        "mainnet" => Network::Bitcoin,
        "testnet" => Network::Testnet,
        "signet" => Network::Signet,
        "regtest" => Network::Regtest,
        _ => Network::Regtest,
    };

    // make the controller to validate Control messages
    let ctrlr = controller_from_seed(&network, &seed[..], flash);

    let pubkey = ctrlr.pubkey();
    let pubkey_str = hex::encode(&pubkey.serialize());
    let token = ctrlr.make_auth_token().expect("couldnt make auth token");
    log::info!("PUBKEY {} TOKEN {}", &pubkey_str, &token);

    let client_id = random_word(8);
    let mqtt_client =
        conn::mqtt::make_client(&config.broker, &client_id, &pubkey_str, &token, tx.clone())?;
    // let mqtt_client = conn::mqtt::start_listening(mqtt, connection, tx)?;

    // this blocks forever... the "main thread"
    let do_log = false;
    log::info!("Network set to {:?}", network);
    log::info!(">>>>>>>>>>> blocking forever...");
    log::info!("{:?}", config);

    // heartbeat loop
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(60));
        let _ = tx.send(Event::HeartBeat);
    });

    make_event_loop(
        mqtt_client,
        rx,
        network,
        do_log,
        led_tx,
        config,
        seed,
        policy,
        velocity,
        ctrlr,
        &client_id,
        &pubkey,
    )?;
    Ok(())
}

pub fn random_word(n: usize) -> String {
    use sphinx_crypter::secp256k1::rand::{self, distributions::Alphanumeric, Rng};
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(n)
        .map(char::from)
        .collect()
}
