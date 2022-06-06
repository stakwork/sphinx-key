use crate::conn;

use anyhow::Result;
use std::sync::{Condvar, Mutex, Arc};
use std::time::Duration;
use serde::{Serialize, Deserialize};

use esp_idf_svc::nvs::*;
use esp_idf_svc::wifi::*;
use embedded_svc::wifi::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub broker: String,
    pub ssid: String,
    pub pass: String,
}

/*
52.91.253.115:1883

arp -a

http://192.168.71.1/?broker=52.91.253.115%3A1883

http://192.168.71.1/?broker=192.168.86.222%3A1883

*/

pub fn start_wifi_client(default_nvs: Arc<EspDefaultNvs>, config: &Config)  -> Result<Box<EspWifi>> {
    let wifi = conn::wifi::start_client(
        default_nvs,
        config
    )?;
    println!("CLIENT CONNECTED!!!!!! {:?}", wifi.get_status());
    Ok(wifi)
}

pub fn start_config_server_and_wait(default_nvs: Arc<EspDefaultNvs>) -> Result<(Box<EspWifi>, Config)> {

    let mutex = Arc::new((Mutex::new(None), Condvar::new()));

    #[allow(clippy::redundant_clone)]
    #[allow(unused_mut)]
    let mut wifi = conn::wifi::start_access_point(
        default_nvs.clone(),
    )?;

    let httpd = conn::http::config_server(mutex.clone());
    
    let mut wait = mutex.0.lock().unwrap();

    let config: &Config = loop {
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