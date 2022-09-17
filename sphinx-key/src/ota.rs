use anyhow::{anyhow, Result};
use embedded_svc::http::client::Client;
use embedded_svc::http::client::Request;
use embedded_svc::http::client::Response;
use embedded_svc::io::Read;
use embedded_svc::ota::Ota;
use esp_idf_svc::http::client::EspHttpClient;
use esp_idf_svc::http::client::EspHttpClientConfiguration;
use esp_idf_svc::http::client::FollowRedirectsPolicy::FollowNone;
use esp_idf_svc::ota::EspOta;
use log::{error, info};
use std::fs::{remove_file, File};
use std::io::BufWriter;
use std::io::Write;

const BUFFER_LEN: usize = 3072;
const UPDATE_BIN_PATH: &str = "/sdcard/update.bin";

fn factory_reset() -> Result<()> {
    let mut ota = EspOta::new()?;
    if ota.is_factory_reset_supported()? {
        info!("Factory reset supported, attempting reset...");
        ota.factory_reset()?;
        Ok(())
    } else {
        error!("FACTORY RESET CURRENTLY NOT SUPPORTED!");
        error!("Only wrote the update binary to the sdcard");
        Err(anyhow!("Factory reset not supported"))
    }
}

fn get_update(version: u64, mut url: String) -> Result<()> {
    let configuration = EspHttpClientConfiguration {
        buffer_size: Some(BUFFER_LEN),
        buffer_size_tx: Some(BUFFER_LEN / 3),
        follow_redirects_policy: FollowNone,
        use_global_ca_store: true,
        crt_bundle_attach: None,
    };
    let mut client = EspHttpClient::new(&configuration)?;
    url.push_str(&version.to_string());
    let mut response = client.get(&url)?.submit()?;
    let mut reader = response.reader();

    let _ = remove_file(UPDATE_BIN_PATH);
    let file = File::create(UPDATE_BIN_PATH)?;
    let mut writer = BufWriter::new(file);

    let mut buf = [0_u8; BUFFER_LEN];
    let mut read_tot: usize = 0;
    let mut write_tot: usize = 0;
    let mut i = 0;
    loop {
        let r = reader.read(&mut buf)?;
        if r == 0 {
            break;
        }
        let w = writer.write(&buf[..r])?;
        read_tot += r;
        write_tot += w;
        i += 1;
        if i % 20 == 0 {
            info!("Cumulative bytes read: {}", read_tot);
            info!("Cumulative bytes written: {}", write_tot);
        }
    }
    info!("TOTAL read: {}", read_tot);
    info!("TOTAL written: {}", write_tot);
    Ok(())
}

pub fn update_sphinx_key(version: u64, url: String) -> Result<()> {
    info!("Getting the update...");
    info!("Version: {}", version.to_string());
    info!("URL: {}", url);
    get_update(version, url)?;
    info!("Update written to sd card, performing factory reset");
    factory_reset()?;
    info!("Factory reset completed!");
    Ok(())
}
