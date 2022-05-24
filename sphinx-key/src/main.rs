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

use anyhow::bail;

const SSID: &str = env!("RUST_ESP32_STD_DEMO_WIFI_SSID");
const PASS: &str = env!("RUST_ESP32_STD_DEMO_WIFI_PASS");

use esp_idf_hal::prelude::*;
use esp_idf_hal::adc;
use embedded_hal::adc::OneShot;

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    sphinx_key_signer::say_hi();

    #[allow(unused)]
    let netif_stack = Arc::new(EspNetifStack::new()?);
    #[allow(unused)]
    let sys_loop_stack = Arc::new(EspSysLoopStack::new()?);
    #[allow(unused)]
    let default_nvs = Arc::new(EspDefaultNvs::new()?);

    let mut wifi = Box::new(EspWifi::new(netif_stack, sys_loop_stack, default_nvs)?);
    info!("About to scan for wifi access points");
    let ap_infos = wifi.scan()?;
    let ours = ap_infos.into_iter().find(|a| a.ssid == SSID);
    let channel = if let Some(ours) = ours {
        info!(
            "Found configured access point {} on channel {}",
            SSID, ours.channel
        );
        Some(ours.channel)
    } else {
        info!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            SSID
        );
        None
    };
    wifi.set_configuration(&Configuration::Client(
        ClientConfiguration {
            ssid: SSID.into(),
            password: PASS.into(),
            channel,
            ..Default::default()
        },
    ))?;

    info!("Client configuration set, about to get status");
    wifi.wait_status(|status| !status.is_transitional());
    let status = wifi.get_status();
    if let Status(
        ClientStatus::Started(ClientConnectionStatus::Connected(ClientIpStatus::Done(_ip_settings)))
    , ApStatus::Stopped) = status
    {
        info!("Client connected");
    }
    else
    {
        bail!("Unexpected Client status: {:?}", status);
    }

    wifi.set_configuration(&Configuration::AccessPoint(
        AccessPointConfiguration {
            ssid: "Sphinx".into(),
            channel: 1,
            ..Default::default()
        },
    ))?;

    info!("AP configuration set, about to get status");

    wifi.wait_status_with_timeout(Duration::from_secs(20), |status| !status.is_transitional())
        .map_err(|e| anyhow::anyhow!("Unexpected Wifi status: {:?}", e))?;

    let status = wifi.get_status();

    if let Status( ClientStatus::Stopped, ApStatus::Started(ApIpStatus::Done)) = status
    {
        info!("AP connected");

    } else {
        bail!("Unexpected AP status: {:?}", status);
    }

    Ok(())
}
