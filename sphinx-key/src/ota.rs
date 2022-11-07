//use crate::core::events::Status;
use anyhow::{anyhow, Result};
use embedded_svc::http::client::Client;
use embedded_svc::http::client::Request;
use embedded_svc::http::client::Response;
use embedded_svc::http::Status as HttpStatus;
use embedded_svc::io::Read;
use embedded_svc::ota::Ota;
use esp_idf_svc::http::client::EspHttpClient;
use esp_idf_svc::http::client::EspHttpClientConfiguration;
use esp_idf_svc::http::client::FollowRedirectsPolicy::FollowNone;
use esp_idf_svc::ota::EspOta;
use log::{error, info};
use sphinx_signer::sphinx_glyph::control::OtaParams;
use std::fs::{remove_file, File};
use std::io::BufWriter;
use std::io::Write;
//use std::sync::mpsc;

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

fn get_update(params: OtaParams) -> Result<()> {
    let configuration = EspHttpClientConfiguration {
        buffer_size: Some(BUFFER_LEN),
        buffer_size_tx: Some(BUFFER_LEN / 3),
        follow_redirects_policy: FollowNone,
        use_global_ca_store: true,
        crt_bundle_attach: None,
    };
    let mut client = EspHttpClient::new(&configuration)?;
    let full_url = params_to_url(params);
    let mut response = client.get(&full_url)?.submit()?;
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
            //led_tx.send(Status::Ota).unwrap();
            info!("Cumulative bytes read: {}", read_tot);
            info!("Cumulative bytes written: {}", write_tot);
        }
    }
    info!("TOTAL read: {}", read_tot);
    info!("TOTAL written: {}", write_tot);
    Ok(())
}

pub fn update_sphinx_key(params: OtaParams) -> Result<()> {
    info!("Getting the update...");
    get_update(params)?;
    info!("Update written to sd card, performing factory reset");
    factory_reset()?;
    info!("Factory reset completed!");
    Ok(())
}

pub fn validate_ota_message(params: OtaParams) -> Result<()> {
    let configuration = EspHttpClientConfiguration {
        buffer_size: Some(BUFFER_LEN / 3),
        buffer_size_tx: Some(BUFFER_LEN / 3),
        follow_redirects_policy: FollowNone,
        use_global_ca_store: true,
        crt_bundle_attach: None,
    };
    let mut client = EspHttpClient::new(&configuration)?;
    let full_url = params_to_url(params);
    info!("Pinging this url for an update: {}", full_url);
    let response = client.get(&full_url)?.submit()?;
    let status = response.status();
    if status == 200 {
        info!("Got valid OTA url! Proceeding with OTA update...");
        Ok(())
    } else if status == 404 {
        error!("got 404, update not found on server, make sure the url and version are correct");
        Err(anyhow!(
            "got 404, update not found on server, make sure the url and version are correct"
        ))
    } else {
        error!(
            "got {} code when fetching update, something is wrong",
            &status.to_string()
        );
        Err(anyhow!(
            "got {} code when fetching update, something is wrong",
            &status.to_string()
        ))
    }
}

fn params_to_url(params: OtaParams) -> String {
    let mut url = params.url.clone();
    url.push_str(&params.version.to_string());
    url
}
