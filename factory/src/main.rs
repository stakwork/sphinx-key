#![no_std]
#![no_main]

mod colors;
mod led;
mod ota;
mod sdcard;

use embedded_sdmmc::{Error, SdCardError};
use esp_idf_svc::{
    hal::{delay::FreeRtos, prelude::Peripherals},
    sys::EspError,
};
use esp_println::println;

#[derive(Debug)]
pub(crate) enum FactoryError {
    SdCardError(Error<SdCardError>),
    OtaError(EspError),
    EspError(EspError),
}

#[no_mangle]
fn main() -> Result<(), FactoryError> {
    esp_idf_svc::sys::link_patches();
    println!("Launcher started");
    let (sd_card_peripherals, led_peripherals) = assign_peripherals()?;
    println!("Assigned peripherals");
    let mut manager = sdcard::setup(sd_card_peripherals)?;
    println!("Setup sdcard");
    let mut led_tx = led::setup(led_peripherals)?;
    println!("Setup led");
    led::setup_complete(&mut led_tx)?; // BLUE
    println!("Setup complete");
    FreeRtos::delay_ms(5000u32);
    if ota::update_present(&mut manager)? {
        led::update_launch(&mut led_tx)?; // ORANGE
        println!("Update present, proceeding with update");
        ota::write_update(&mut manager)?;
        led::update_complete(&mut led_tx)?; // GREEN
        println!("Update finished, restarting the chip");
    } else {
        println!("No update present, setting boot to main app");
        ota::set_boot_main_app()?;
        led::main_app_launch(&mut led_tx)?; // WHITE
        println!("Boot set to main app");
    }
    println!("Restarting esp");
    FreeRtos::delay_ms(5000u32);
    unsafe { esp_idf_svc::sys::esp_restart() };
}

fn assign_peripherals() -> Result<(sdcard::Peripherals, led::Peripherals), FactoryError> {
    // this function here must be called only once
    let peripherals = Peripherals::take().map_err(|e| FactoryError::EspError(e))?;
    let sd_card_peripherals = sdcard::Peripherals {
        spi: peripherals.spi2,
        sck: peripherals.pins.gpio6,
        mosi: peripherals.pins.gpio7,
        miso: peripherals.pins.gpio2,
        cs: peripherals.pins.gpio10,
    };
    let led_peripherals = led::Peripherals {
        led: peripherals.pins.gpio0,
        channel: peripherals.rmt.channel0,
    };
    Ok((sd_card_peripherals, led_peripherals))
}
