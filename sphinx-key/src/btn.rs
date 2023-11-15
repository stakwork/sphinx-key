mod conn;
mod core;
mod ota;
mod periph;
mod status;

pub use crate::core::control::FlashPersister;
use esp_idf_svc::hal::gpio::Gpio0;
use esp_idf_svc::hal::gpio::Gpio9;
use esp_idf_svc::hal::peripheral::Peripheral;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use status::Status;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const ID_LEN: usize = 16;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    thread::sleep(Duration::from_secs(1));
    let mut peripherals = Peripherals::take().unwrap();

    // LED control thread
    let (mut led_tx, mut led_rx) = mpsc::channel::<Status>();
    while let Err(e) = periph::led::led_control_loop(
        unsafe { Gpio0::new() },
        unsafe { peripherals.rmt.channel0.clone_unchecked() },
        led_rx,
    ) {
        log::error!("unable to spawn led thread: {:?}", e);
        thread::sleep(Duration::from_millis(1000));
        (led_tx, led_rx) = mpsc::channel::<Status>();
    }

    // BUTTON thread
    let default_nvs = EspDefaultNvsPartition::take()?;
    let flash_per = FlashPersister::new(default_nvs);
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
