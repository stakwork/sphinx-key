use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

use embedded_svc::storage::StorageBase;
use esp_idf_svc::nvs::*;
use esp_idf_svc::nvs_storage::EspNvsStorage;

use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    let default_nvs = Arc::new(EspDefaultNvs::new()?);
    let mut store =
        EspNvsStorage::new_default(default_nvs.clone(), "sphinx", true).expect("no storage");
    store.remove("config").expect("couldnt remove config 1");
    store.remove("seed").expect("couldnt remove seed 1");
    store.remove("nonce").expect("couldnt remove nonce 1");
    store.remove("policy").expect("couldnt remove policy 1");
    println!("NVS cleared!");
    Ok(())
}
