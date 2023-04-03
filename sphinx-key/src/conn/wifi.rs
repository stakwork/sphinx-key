use sphinx_signer::sphinx_glyph::control::Config;

use esp_idf_svc::netif::*;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::ping;
// use esp_idf_svc::sysloop::*;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::wifi::*;

use embedded_svc::httpd::Result;
use embedded_svc::ipv4;
// use embedded_svc::ping::Ping;
// use embedded_svc::wifi::Wifi;
use embedded_svc::wifi::*;

use esp_idf_hal::peripheral;

use log::*;
// use std::thread;
use std::time::Duration;

pub fn start_client(
    modem: impl peripheral::Peripheral<P = esp_idf_hal::modem::Modem> + 'static,
    default_nvs: EspDefaultNvsPartition,
    config: &Config,
) -> Result<Box<EspWifi<'static>>> {
    // let netif_stack = Arc::new(EspNetifStack::new()?);
    // let sys_loop_stack = Arc::new(EspSysLoopStack::new()?);

    let sysloop = EspSystemEventLoop::take()?;

    let mut wifi = Box::new(EspWifi::new(modem, sysloop.clone(), Some(default_nvs))?);
    let ap_infos = wifi.scan()?;
    let ssid = config.ssid.as_str();
    let pass = config.pass.as_str();

    let ours = ap_infos.into_iter().find(|a| a.ssid == ssid);
    let channel = if let Some(ours) = ours {
        info!(
            "Found configured access point {} on channel {}",
            ssid, ours.channel
        );
        Some(ours.channel)
    } else {
        info!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            ssid
        );
        None
    };

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ssid.into(),
        password: pass.into(),
        channel,
        ..Default::default()
    }))?;

    wifi.start()?;

    info!("...Wifi client configuration set, get status");
    // match wifi.wait_status_with_timeout(Duration::from_secs(20), |status| !status.is_transitional())
    // {
    //     Ok(_) => (),
    //     Err(e) => warn!("Unexpected Wifi status: {:?}", e),
    // };

    if !WifiWait::new(&sysloop)?
        .wait_with_timeout(Duration::from_secs(20), || wifi.is_started().unwrap())
    {
        warn!("Wifi did not start");
    }

    info!("Connecting wifi...");

    wifi.connect()?;

    if !EspNetifWait::new::<EspNetif>(wifi.sta_netif(), &sysloop)?.wait_with_timeout(
        Duration::from_secs(20),
        || {
            wifi.is_connected().unwrap()
                && wifi.sta_netif().get_ip_info().unwrap().ip != std::net::Ipv4Addr::new(0, 0, 0, 0)
        },
    ) {
        warn!("Wifi did not connect or did not receive a DHCP lease");
    }

    let ip_info = wifi.sta_netif().get_ip_info()?;

    info!("Wifi DHCP info: {:?}", ip_info);
    // let status = wifi.get_status();
    // println!("=> wifi STATUS {:?}", status);
    // println!("=> is transitional? {:?}", status.is_transitional());
    // if let Status(
    //     ClientStatus::Started(ClientConnectionStatus::Connected(ClientIpStatus::Done(ip_settings))),
    //     ApStatus::Stopped,
    // ) = status
    // {
    //     info!("Wifi started!");
    //     ping(&ip_settings)?;
    // } else {
    //     thread::sleep(Duration::from_secs(13));
    //     // bail!("Unexpected Client Wifi status: {:?}", status);
    //     return Err(anyhow::anyhow!(
    //         "Unexpected Client Wifi status: {:?}",
    //         status
    //     ));
    // }

    info!("wifi::start_client Ok(())");

    Ok(wifi)
}

pub fn start_access_point(
    modem: impl peripheral::Peripheral<P = esp_idf_hal::modem::Modem> + 'static,
    default_nvs: EspDefaultNvsPartition,
) -> Result<Box<EspWifi<'static>>> {
    let sysloop = EspSystemEventLoop::take()?;
    // let netif_stack = Arc::new(EspNetifStack::new()?);
    // let sys_loop_stack = Arc::new(EspSysLoopStack::new()?);
    let mut wifi = Box::new(EspWifi::new(modem, sysloop.clone(), Some(default_nvs))?);

    let ssid: &'static str = env!("SSID");
    let password: &'static str = env!("PASS");
    if password.len() < 8 {
        return Err(anyhow::anyhow!("Password error!\nCurrent password: {}\nYour wifi password must be >= 8 characters. Compile this software again with a longer password.", password));
    }

    wifi.set_configuration(&Configuration::AccessPoint(AccessPointConfiguration {
        ssid: ssid.into(),
        password: password.into(),
        channel: 6,
        auth_method: AuthMethod::WPA2Personal,
        ..Default::default()
    }))?;

    wifi.start()?;

    info!("Wifi configuration set, about to get status");
    if !WifiWait::new(&sysloop)?
        .wait_with_timeout(Duration::from_secs(20), || wifi.is_started().unwrap())
    {
        return Err(anyhow::anyhow!("Wifi did not start"));
    }

    info!(
        "Wifi started!\n \nWIFI NAME: {}\nWIFI PASSWORD: {}\n",
        ssid, password
    );

    // let status = wifi.get_status();
    // if let Status(ClientStatus::Stopped, ApStatus::Started(ApIpStatus::Done)) = status {
    //     info!(
    //         "Wifi started!\n \nWIFI NAME: {}\nWIFI PASSWORD: {}\n",
    //         ssid, password
    //     );
    // } else {
    //     return Err(anyhow::anyhow!("Unexpected AP Wifi status: {:?}", status));
    // }

    Ok(wifi)
}

fn _ping(ip_settings: &ipv4::ClientSettings) -> Result<()> {
    info!("About to do some pings for {:?}", ip_settings);

    let ping_summary =
        ping::EspPing::default().ping(ip_settings.subnet.gateway, &Default::default())?;
    if ping_summary.transmitted != ping_summary.received {
        return Err(anyhow::anyhow!(
            "Pinging gateway {} resulted in timeouts",
            ip_settings.subnet.gateway
        ));
    }

    info!("Pinging done");

    Ok(())
}
