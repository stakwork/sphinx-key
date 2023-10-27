use anyhow::Result;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::peripheral;
use esp_idf_svc::ipv4::Ipv4Addr;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::ping::EspPing;
use esp_idf_svc::wifi::{
    AccessPointConfiguration, AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi,
};
use log::*;
use sphinx_signer::sphinx_glyph::control::Config;

pub fn start_client(
    modem: impl peripheral::Peripheral<P = esp_idf_svc::hal::modem::Modem> + 'static,
    default_nvs: EspDefaultNvsPartition,
    config: &Config,
) -> Result<BlockingWifi<EspWifi<'static>>> {
    // let netif_stack = Arc::new(EspNetifStack::new()?);
    // let sys_loop_stack = Arc::new(EspSysLoopStack::new()?);
    let sysloop = EspSystemEventLoop::take()?;

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(modem, sysloop.clone(), Some(default_nvs))?,
        sysloop,
    )?;

    let ssid = config.ssid.as_str();
    let pass = config.pass.as_str();
    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ssid.into(),
        password: pass.into(),
        channel: None,
        ..Default::default()
    }))?;
    info!("Wifi configured");

    loop {
        match try_connection(&mut wifi) {
            Ok(_) => break,
            Err(e) => info!("error: {}, trying wifi connection again!", e),
        }
    }

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

fn try_connection(wifi: &mut BlockingWifi<EspWifi<'static>>) -> Result<()> {
    wifi.start()?;
    info!("Wifi started");
    wifi.connect()?;
    info!("Wifi connected");
    wifi.wait_netif_up()?;
    info!("Wifi netif up");
    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    info!("Wifi DHCP info: {:?}", ip_info);
    Ok(())
}

pub fn start_access_point(
    modem: impl peripheral::Peripheral<P = esp_idf_svc::hal::modem::Modem> + 'static,
    default_nvs: EspDefaultNvsPartition,
) -> Result<BlockingWifi<EspWifi<'static>>> {
    let sysloop = EspSystemEventLoop::take()?;
    // let netif_stack = Arc::new(EspNetifStack::new()?);
    // let sys_loop_stack = Arc::new(EspSysLoopStack::new()?);
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(modem, sysloop.clone(), Some(default_nvs))?,
        sysloop,
    )?;

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
    info!("Wifi configured");

    wifi.start()?;
    info!("Wifi started");
    wifi.wait_netif_up()?;
    info!("Wifi netif up");

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

fn _ping(ip_addr: Ipv4Addr) -> Result<()> {
    info!("About to do some pings for {}", ip_addr);

    let ping_summary = EspPing::default().ping(ip_addr, &Default::default())?;
    if ping_summary.transmitted != ping_summary.received {
        return Err(anyhow::anyhow!(
            "Pinging gateway {} resulted in timeouts",
            ip_addr
        ));
    }

    info!("Pinging done");

    Ok(())
}
