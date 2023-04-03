use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

// use embedded_svc::storage::StorageBase;
use esp_idf_svc::nvs::EspNvs;
use esp_idf_svc::nvs::*;

fn main() -> anyhow::Result<()> {
    // NvsDefault::new();
    let default_nvs = EspDefaultNvsPartition::take()?;
    let mut store = EspNvs::new(default_nvs.clone(), "sphinx", true).expect("no storage");
    store.remove("config").expect("couldnt remove config 1");
    store.remove("seed").expect("couldnt remove seed 1");
    store.remove("nonce").expect("couldnt remove nonce 1");
    store.remove("policy").expect("couldnt remove policy 1");
    println!("NVS cleared!");
    Ok(())
}
