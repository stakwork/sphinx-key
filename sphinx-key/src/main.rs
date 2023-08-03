mod conn;
mod core;
mod ota;
mod periph;
mod status;

use crate::core::control::controller_from_seed;
pub use crate::core::control::FlashPersister;
use crate::core::{config::*, events::*};
use crate::periph::button::button_loop;
use crate::periph::led::led_control_loop;
#[allow(unused_imports)]
use crate::periph::sd::{mount_sd_card, simple_fs_test};
use crate::status::Status;

use anyhow::Result;
use esp_idf_hal::gpio::{Gpio0, Gpio9};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use sphinx_signer::lightning_signer::bitcoin::Network;
use sphinx_signer::sphinx_glyph::control::{Config, ControlPersist, Policy, Velocity};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::SystemTime;

const ID_LEN: usize = 12;

fn main() -> Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    thread::sleep(Duration::from_secs(1));
    let mut peripherals = Peripherals::take().unwrap();

    // LED control thread
    let (mut led_tx, mut led_rx) = mpsc::channel::<Status>();
    while let Err(e) = led_control_loop(
        unsafe { Gpio0::new() },
        unsafe { peripherals.rmt.channel0.clone_unchecked() },
        led_rx,
    ) {
        log::error!("unable to spawn led thread: {:?}", e);
        thread::sleep(Duration::from_millis(1000));
        (led_tx, led_rx) = mpsc::channel::<Status>();
    }

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
    let flash_per = FlashPersister::new(default_nvs.clone());
    let flash_arc = Arc::new(Mutex::new(flash_per));
    // BUTTON thread
    while let Err(e) = button_loop(unsafe { Gpio9::new() }, led_tx.clone(), flash_arc.clone()) {
        log::error!("unable to spawn button thread: {:?}", e);
        thread::sleep(Duration::from_millis(1000));
    }
    let flash = flash_arc.lock().unwrap();
    if let Ok(exist) = flash.read_config() {
        let seed = flash.read_seed().expect("no seed...");
        let id = flash.read_id().expect("no id...");
        let policy = flash.read_policy().unwrap_or_default();
        let velocity = flash.read_velocity().ok();
        drop(flash);
        println!(
            "=============> START CLIENT NOW <============== {:?}",
            exist
        );
        led_tx.send(Status::ConnectingToWifi).unwrap();
        let _wifi = match start_wifi_client(peripherals.modem, default_nvs.clone(), &exist) {
            Ok(wifi) => wifi,
            Err(e) => {
                log::error!("Could not setup wifi: {}", e);
                log::info!("Restarting esp!");
                unsafe { esp_idf_sys::esp_restart() };
            }
        };

        led_tx.send(Status::SyncingTime).unwrap();
        if let Err(e) = conn::sntp::sync_time_timeout() {
            log::error!("Could not setup sntp: {}", e);
            log::info!("Restarting esp!");
            unsafe { esp_idf_sys::esp_restart() };
        }
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        println!(
            "Completed the time sync, here is the UNIX time: {}",
            now.as_secs(),
        );

        led_tx.send(Status::ConnectingToMqtt).unwrap();

        loop {
            if let Ok(()) = make_and_launch_client(
                exist.clone(),
                seed,
                id.clone(),
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
        let stored_seed = flash.read_seed().ok();
        drop(flash);
        match start_config_server_and_wait(
            peripherals.modem,
            default_nvs.clone(),
            stored_seed.is_some(),
        ) {
            Ok((_wifi, config, seed_opt)) => {
                let mut flash = flash_arc.lock().unwrap();
                flash.write_config(config).expect("could not store config");
                if stored_seed.is_none() {
                    match seed_opt {
                        Some(s) => flash.write_seed(s).expect("could not store seed"),
                        None => panic!("SEED REQUIRED!!!"),
                    }
                    flash
                        .write_id(random_word(ID_LEN))
                        .expect("could not store id");
                }
                drop(flash);
                println!("CONFIG SAVED");
                unsafe { esp_idf_sys::esp_restart() };
            }
            Err(msg) => {
                log::error!("{}", msg);
            }
        }
    }

    Ok(())
}

fn make_and_launch_client(
    config: Config,
    seed: [u8; 32],
    client_id: String,
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
