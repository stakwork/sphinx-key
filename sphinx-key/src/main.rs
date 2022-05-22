#![allow(unused_imports)]

mod conn;

use sphinx_key_signer;
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use std::thread;
use log::*;

use std::sync::{Condvar, Mutex, Arc};
use std::time::*;

use esp_idf_svc::nvs::*;
use esp_idf_svc::nvs_storage::EspNvsStorage;
use esp_idf_svc::netif::*;
use esp_idf_svc::eventloop::*;
use esp_idf_svc::sysloop::*;
use esp_idf_svc::wifi::*;

use embedded_svc::httpd::*;
use embedded_svc::wifi::*;

// use log::*;
// use url;

fn main() -> Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    sphinx_key_signer::say_hi();

    // let init_conf = Some(conn::Config{
    //     broker: "52.91.253.115:1883".to_string(),
    // });
    let mutex = Arc::new((Mutex::new(None), Condvar::new()));

    let netif_stack = Arc::new(EspNetifStack::new()?);
    let sys_loop_stack = Arc::new(EspSysLoopStack::new()?);
    let default_nvs = Arc::new(EspDefaultNvs::new()?);
    let storage = Arc::new(Mutex::new(EspNvsStorage::new_default(default_nvs.clone(), "sphinx", true).expect("NVS FAIL")));

    #[allow(clippy::redundant_clone)]
    #[allow(unused_mut)]
    let mut wifi = conn::wifi::connect(
        netif_stack.clone(),
        sys_loop_stack.clone(),
        default_nvs.clone(),
    )?;

    // conn::tcp::tcp_bind().expect("failed TCP bind");

    let httpd = conn::config_server(mutex.clone(), storage);
    
    info!("=====> yo yo");

    let mut wait = mutex.0.lock().unwrap();

    let config = loop {
        if let Some(conf) = &*wait {
            break conf;
        } else {
            wait = mutex
                .1
                .wait_timeout(wait, Duration::from_secs(1))
                .unwrap()
                .0;
            println!("tick...");
        }
    };

    println!("===> config! {:?}", config);

    let mut i = 0;
    loop {
        thread::sleep(Duration::from_secs(5));
        i = i + 1;
        println!("wait forever... {}", i);
    } 

    // drop(httpd);
    // println!("Httpd stopped");

    /* shutdown */
    // drop(httpd);
    // info!("Httpd stopped");

    Ok(())
}
