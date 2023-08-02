mod conn;
mod core;
mod ota;
mod periph;
mod status;

pub use crate::core::control::FlashPersister;
use esp_idf_hal::gpio::Gpio9;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::nvs::EspNvs;
use esp_idf_svc::nvs::*;
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use log;
use status::Status;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const ID_LEN: usize = 12;

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    thread::sleep(Duration::from_secs(1));

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;

    let (led_tx, led_rx) = mpsc::channel::<Status>();
    // LED control thread
    periph::led::led_control_loop(pins.gpio0, peripherals.rmt.channel0, led_rx);

    // BUTTON thread
    let default_nvs = EspDefaultNvsPartition::take()?;
    let flash_per = FlashPersister::new(default_nvs.clone());
    let flash_arc = Arc::new(Mutex::new(flash_per));
    while let Err(e) =
        periph::button::button_loop(unsafe { Gpio9::new() }, led_tx.clone(), flash_arc.clone())
    {
        log::error!("unable to spawn button thread: {:?}", e);
        thread::sleep(Duration::from_millis(1000));
    }

    loop {
        thread::sleep(Duration::from_millis(1000));
    }
}
