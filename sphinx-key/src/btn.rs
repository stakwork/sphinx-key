mod periph;
mod status;

use status::Status;

use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

// use embedded_svc::storage::StorageBase;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspNvs;
use esp_idf_svc::nvs::*;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

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
    periph::button::button_loop(pins.gpio9, led_tx.clone());

    loop {
        thread::sleep(Duration::from_millis(1000));
    }
}
