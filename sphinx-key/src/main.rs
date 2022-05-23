#![allow(unused_imports)]

mod conn;

use sphinx_key_signer;
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use std::thread;
use log::*;

use std::sync::{Condvar, Mutex, Arc, atomic::*};
use std::time::*;

use esp_idf_svc::nvs::*;
use esp_idf_svc::nvs_storage::EspNvsStorage;
use esp_idf_svc::netif::*;
use esp_idf_svc::eventloop::*;
use esp_idf_svc::sysloop::*;
use esp_idf_svc::wifi::*;

use embedded_svc::httpd::*;
use embedded_svc::wifi::*;
use embedded_svc::storage::Storage;
// use log::*;
// use url;

fn main() -> Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    sphinx_key_signer::say_hi();

    thread::sleep(Duration::from_secs(1));

    let default_nvs = Arc::new(EspDefaultNvs::new()?);
    // let storage = Arc::new(Mutex::new(EspNvsStorage::new_default(default_nvs.clone(), "sphinx", true).expect("NVS FAIL")));
    let mut store = EspNvsStorage::new_default(default_nvs.clone(), "sphinx", true).expect("no storage");
    // uncomment to clear:
    // store.remove("config").expect("couldnt remove config 1");
    let existing: Option<conn::Config> = store.get("config").expect("failed");
    if let Some(exist) = existing {
        println!("=============> START CLIENT NOW <============== {:?}", exist);
        // store.remove("config").expect("couldnt remove config");
        if let Err(e) = start_client(default_nvs.clone(), &exist) {
            error!("CLIENT ERROR {:?}", e);
        }
    } else {
        println!("=============> START SERVER NOW AND WAIT <==============");
        if let Ok((mut wifi, config)) = start_server_and_wait(default_nvs.clone()) {
            store.put("config", &config).expect("could not store config");
            println!("CONFIG SAVED");
            drop(wifi);
            thread::sleep(Duration::from_secs(1));
        }
    }

    Ok(())
}

fn start_server_and_wait(default_nvs: Arc<EspDefaultNvs>) -> Result<(Box<EspWifi>, conn::Config)> {
    let netif_stack = Arc::new(EspNetifStack::new()?);
    let sys_loop_stack = Arc::new(EspSysLoopStack::new()?);

    let mutex = Arc::new((Mutex::new(None), Condvar::new()));

    #[allow(clippy::redundant_clone)]
    #[allow(unused_mut)]
    let mut wifi = conn::wifi::start_server(
        netif_stack.clone(),
        sys_loop_stack.clone(),
        default_nvs.clone(),
    )?;

    let httpd = conn::http::config_server(mutex.clone());
    
    let mut wait = mutex.0.lock().unwrap();

    let config: &conn::Config = loop {
        if let Some(conf) = &*wait {
            break conf;
        } else {
            wait = mutex
                .1
                .wait_timeout(wait, Duration::from_secs(1))
                .unwrap()
                .0;
        }
    };

    drop(httpd);
    // drop(wifi);
    // thread::sleep(Duration::from_secs(1));
    println!("===> config! {:?}", config);
    Ok((wifi, config.clone()))
}

fn start_client(default_nvs: Arc<EspDefaultNvs>, config: &conn::Config)  -> Result<()> {
    let netif_stack = Arc::new(EspNetifStack::new()?);
    let sys_loop_stack = Arc::new(EspSysLoopStack::new()?);

    let wifi = conn::wifi::start_client(
        netif_stack.clone(),
        sys_loop_stack.clone(),
        default_nvs.clone(),
        config
    )?;

    println!("CLIENT CONNECTED!!!!!! {:?}", wifi.get_status());

    let mut i = 0;
    loop {
        thread::sleep(Duration::from_secs(5));
        i = i + 1;
        println!("wait forever... {}", i);
    }
    Ok(())
}