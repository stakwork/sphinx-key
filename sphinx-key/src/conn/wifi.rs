use crate::conn::Config;

use esp_idf_svc::wifi::*;
use esp_idf_svc::sysloop::*;
use esp_idf_svc::netif::*;
use esp_idf_svc::nvs::EspDefaultNvs;
use esp_idf_svc::ping;

use embedded_svc::wifi::*;
use embedded_svc::httpd::Result;
use embedded_svc::ping::Ping;
use embedded_svc::ipv4;

use log::*;
use anyhow::bail;
use std::time::Duration;
use std::sync::Arc;
use std::thread;

#[allow(dead_code)]
pub fn start_client(
    netif_stack: Arc<EspNetifStack>,
    sys_loop_stack: Arc<EspSysLoopStack>,
    default_nvs: Arc<EspDefaultNvs>,
    config: &Config,
) -> Result<Box<EspWifi>> {
    let mut wifi = Box::new(EspWifi::new(netif_stack, sys_loop_stack, default_nvs)?);
    let ap_infos = wifi.scan()?;
    let ssid = config.ssid.as_str();
    let pass = config.pass.as_str();

    let ours = ap_infos.into_iter().find(|a| a.ssid == ssid);
    let channel = if let Some(ours) = ours {
        info!("Found configured access point {} on channel {}", ssid, ours.channel);
        Some(ours.channel)
    } else {
        info!("Configured access point {} not found during scanning, will go with unknown channel", ssid);
        None
    };

    wifi.set_configuration(&Configuration::Client(
        ClientConfiguration {
            ssid: ssid.into(),
            password: pass.into(),
            channel,
            ..Default::default()
        }
    ))?;

    // not working
    info!("Wifi client configuration set, about to get status");
    match wifi.wait_status_with_timeout(Duration::from_secs(20), |status| !status.is_transitional()) {
        Ok(_) => (),
        Err(e) => warn!("Unexpected Wifi status: {:?}", e),
    };
    let status = wifi.get_status();
    println!("=> wifi STATUS 1 {:?}", status);

    info!("...Wifi client configuration set, AGAIN get status");
    match wifi.wait_status_with_timeout(Duration::from_secs(20), |status| !status.is_transitional()) {
        Ok(_) => (),
        Err(e) => warn!("Unexpected Wifi status: {:?}", e),
    };

    let status = wifi.get_status();
    println!("=> wifi STATUS {:?}", status);
    println!("=> is transitional? {:?}", status.is_transitional());
    if let Status(
        ClientStatus::Started(ClientConnectionStatus::Connected(ClientIpStatus::Done(ip_settings))),
        ApStatus::Stopped,
    ) = status {
        info!("Wifi started!");
        ping(&ip_settings)?;
    } else {
        thread::sleep(Duration::from_secs(13));
        // bail!("Unexpected Client Wifi status: {:?}", status);
        return Err(anyhow::anyhow!("Unexpected Client Wifi status: {:?}", status));
    }

    info!("wifi::start_client Ok(())");

    Ok(wifi)
}

#[allow(dead_code)]
pub fn start_server(
    netif_stack: Arc<EspNetifStack>,
    sys_loop_stack: Arc<EspSysLoopStack>,
    default_nvs: Arc<EspDefaultNvs>,
) -> Result<Box<EspWifi>> {
    let mut wifi = Box::new(EspWifi::new(netif_stack, sys_loop_stack, default_nvs)?);
    wifi.set_configuration(&Configuration::AccessPoint(
        AccessPointConfiguration {
            ssid: "sphinxkey".into(),
            channel: 6,
            ..Default::default()
        },
    ))?;

    info!("Wifi configuration set, about to get status");
    wifi.wait_status_with_timeout(Duration::from_secs(20), |status| !status.is_transitional())
        .map_err(|e| anyhow::anyhow!("Unexpected Wifi status: {:?}", e))?;

    let status = wifi.get_status();
    if let Status(
        ClientStatus::Stopped,
        ApStatus::Started(ApIpStatus::Done),
    ) = status {
        info!("Wifi started!");
    } else {
        return Err(anyhow::anyhow!("Unexpected AP Wifi status: {:?}", status));
    }

    Ok(wifi)
}

fn ping(ip_settings: &ipv4::ClientSettings) -> Result<()> {
    info!("About to do some pings for {:?}", ip_settings);

    let ping_summary =
        ping::EspPing::default().ping(ip_settings.subnet.gateway, &Default::default())?;
    if ping_summary.transmitted != ping_summary.received {
        return Err(anyhow::anyhow!("Pinging gateway {} resulted in timeouts", ip_settings.subnet.gateway));
    }

    info!("Pinging done");

    Ok(())
}