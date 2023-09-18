mod conn;
mod core;
mod ota;
mod periph;
mod status;

#[allow(unused_imports)]
use crate::periph::sd::mount_sd_card;

use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

// use embedded_svc::storage::StorageBase;
// use esp_idf_svc::nvs::EspNvs;
// use esp_idf_svc::nvs::*;

use std::fs;
use std::path::Path;

pub const ROOT_STORE: &str = "/sdcard/store";
pub const ID_LEN: usize = 16usize;

fn main() -> anyhow::Result<()> {
    // NvsDefault::new();
    // let default_nvs = EspDefaultNvsPartition::take()?;
    // let mut store = EspNvs::new(default_nvs.clone(), "sphinx", true).expect("no storage");
    // store.remove("config").expect("couldnt remove config 1");
    // store.remove("seed").expect("couldnt remove seed 1");
    // store.remove("nonce").expect("couldnt remove nonce 1");
    // store.remove("policy").expect("couldnt remove policy 1");
    // println!("NVS cleared!");

    println!("About to mount the sdcard...");
    while let Err(_e) = mount_sd_card() {
        println!("Failed to mount sd card. Make sure it is connected, trying again...");
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
    println!("SD card mounted!");

    let dir = Path::new(ROOT_STORE);
    println!("root store is dir {}", dir.is_dir());
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                println!("PATH {}", path.display());
                if let Err(e) = fs::remove_dir_all(path.clone()) {
                    println!("err removing dir {:?}", e);
                    // remove inner dirs too
                    for entry in fs::read_dir(path)? {
                        let entry = entry?;
                        let path = entry.path();
                        if path.is_dir() {
                            println!("INNER PATH {:?}", path.display());
                            if let Err(e) = fs::remove_dir_all(path) {
                                println!("err removing inner dir {:?}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    let dir = Path::new(ROOT_STORE);
    println!("root store is dir {}", dir.is_dir());
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                println!("PATH {}", path.display());
                if let Err(e) = fs::remove_dir_all(path.clone()) {
                    println!("err removing dir {:?}", e);
                }
            }
        }
    }

    Ok(())
}
