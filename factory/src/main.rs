mod ota;
mod sdcard;
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use log::{error, info, warn};
use ota::{run_sdcard_ota_update, set_boot_main_app, UPDATE_BIN_PATH};
use std::path::Path;
use std::thread;
use std::time::Duration;

fn main() {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_svc::log::EspLogger::initialize_default();
    esp_idf_sys::link_patches();

    thread::sleep(Duration::from_secs(10));
    info!("Hello, world! Mounting sd card...");
    sdcard::mount_sd_card();
    info!("SD card mounted! Checking for update...");
    if let Ok(true) = Path::new(UPDATE_BIN_PATH).try_exists() {
        info!("Found update.bin file! Launching the update process...");
        while let Err(e) = run_sdcard_ota_update() {
            error!("OTA update failed: {}", e.to_string());
            error!("Trying again...");
            thread::sleep(Duration::from_secs(5));
        }
        info!("OTA update complete!");
    } else {
        warn!("Update file not found! Setting up main app boot...");
        set_boot_main_app();
    }
    info!("Restarting ESP, booting the main app...");
    unsafe { esp_idf_sys::esp_restart() };
}
