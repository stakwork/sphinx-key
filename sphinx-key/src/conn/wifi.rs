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

const SSID: &str = "apples&acorns";
const PASS: &str = "42flutes";

#[cfg(not(feature = "qemu"))]
#[allow(dead_code)]
pub fn connect(
    netif_stack: Arc<EspNetifStack>,
    sys_loop_stack: Arc<EspSysLoopStack>,
    default_nvs: Arc<EspDefaultNvs>,
) -> Result<Box<EspWifi>> {
    let mut wifi = Box::new(EspWifi::new(netif_stack, sys_loop_stack, default_nvs)?);

    info!("Wifi created, about to scan");

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

    // let conf = Configuration::Client(
    //     ClientConfiguration {
    //         ssid: SSID.into(),
    //         password: PASS.into(),
    //         channel,
    //         ..Default::default()
    //     };
    // );
    // let conf = Configuration::AccessPoint(
    //     AccessPointConfiguration {
    //         ssid: "aptest111".into(),
    //         channel: channel.unwrap_or(1),
    //         ..Default::default()
    //     },
    // );
    let conf = Configuration::Mixed(
        ClientConfiguration {
            ssid: SSID.into(),
            password: PASS.into(),
            channel,
            ..Default::default()
        },
        AccessPointConfiguration {
            ssid: "aptest123".into(),
            channel: channel.unwrap_or(1),
            ..Default::default()
        },
    );
    wifi.set_configuration(&conf)?;

    info!("Wifi configuration set, about to get status");

    wifi.wait_status_with_timeout(Duration::from_secs(20), |status| !status.is_transitional())
        .map_err(|e| anyhow::anyhow!("Unexpected Wifi status: {:?}", e))?;

    let status = wifi.get_status();

    if let Status(
        ClientStatus::Started(ClientConnectionStatus::Connected(ClientIpStatus::Done(ip_settings))),
        ApStatus::Started(ApIpStatus::Done),
    ) = status
    {
        info!("Wifi connected");

        ping(&ip_settings)?;
    } else {
        bail!("Unexpected Wifi status: {:?}", status);
    }

    Ok(wifi)
}


fn ping(ip_settings: &ipv4::ClientSettings) -> Result<()> {
    info!("About to do some pings for {:?}", ip_settings);

    let ping_summary =
        ping::EspPing::default().ping(ip_settings.subnet.gateway, &Default::default())?;
    if ping_summary.transmitted != ping_summary.received {
        bail!(
            "Pinging gateway {} resulted in timeouts",
            ip_settings.subnet.gateway
        );
    }

    info!("Pinging done");

    Ok(())
}