use anyhow::{anyhow, Result};
use esp_idf_svc::http::client::Configuration;
use esp_idf_svc::http::client::EspHttpConnection;
use esp_idf_svc::http::client::FollowRedirectsPolicy::FollowNone;
use esp_idf_svc::http::Method;
use esp_idf_svc::ota::EspOta;
use log::{error, info};
use sphinx_signer::lightning_signer::bitcoin::hashes::{sha256, Hash};
use sphinx_signer::lightning_signer::bitcoin::secp256k1::{
    schnorr::Signature, Message, PublicKey, Secp256k1,
};
use sphinx_signer::sphinx_glyph::control::OtaParams;
use std::fs::{remove_file, File};
use std::io::Write;
use std::io::{BufReader, BufWriter};
use std::str::FromStr;

const BUFFER_LEN: usize = 1024;
const UPDATE_BIN_PATH: &str = "/sdcard/update.bin";
const PUBLIC: &str = "039707459d92b1809a9f6f78feebf6f518e7319b851fe474a31d64307b86aaf38a";

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

fn get_update(params: &OtaParams) -> Result<()> {
    let configuration = Configuration {
        buffer_size: Some(BUFFER_LEN),
        buffer_size_tx: Some(BUFFER_LEN / 3),
        follow_redirects_policy: FollowNone,
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        ..Default::default()
    };
    let mut reader = EspHttpConnection::new(&configuration)?;
    let full_url = params_to_url(params);
    reader.initiate_request(Method::Get, &full_url, &[])?;
    reader.initiate_response()?;
    // let mut reader = response.reader();

    let _ = remove_file(UPDATE_BIN_PATH);
    let file = File::create(UPDATE_BIN_PATH)?;
    let mut writer = BufWriter::new(file);

    let mut buf = [0_u8; BUFFER_LEN];
    let mut read_tot: usize = 0;
    let mut write_tot: usize = 0;
    loop {
        let r = reader.read(&mut buf)?;
        if r == 0 {
            break;
        }
        let w = writer.write(&buf[..r])?;
        read_tot += r;
        write_tot += w;
    }
    info!("TOTAL read: {}", read_tot);
    info!("TOTAL written: {}", write_tot);
    Ok(())
}

fn check_signature(params: &OtaParams) -> Result<()> {
    let msg = Message::from_hashed_data::<sha256::Hash>(params.sha256_hash.as_bytes());
    let sig = Signature::from_str(&params.schnorr_sig).unwrap();
    let pbk = PublicKey::from_str(PUBLIC).unwrap().x_only_public_key().0;
    let secp = Secp256k1::verification_only();
    secp.verify_schnorr(&sig, &msg, &pbk).unwrap();
    Ok(())
}
fn check_integrity(params: &OtaParams) -> Result<()> {
    let f = File::open(UPDATE_BIN_PATH)?;
    let mut reader = BufReader::new(f);
    let mut engine = sha256::HashEngine::default();
    std::io::copy(&mut reader, &mut engine)?;
    let hash = sha256::Hash::from_engine(engine);
    if hash.to_string() == params.sha256_hash {
        Ok(())
    } else {
        Err(anyhow!(
            "Integrity check failed! params: {} vs sdcard: {}",
            params.sha256_hash,
            hash.to_string()
        ))
    }
}

pub fn update_sphinx_key(params: &OtaParams) -> Result<()> {
    info!("Getting the update...");
    get_update(params)?;
    info!("Update written to sd card, checking integrity...");
    check_integrity(params)?;
    info!("Integrity check passed, performing factory reset...");
    factory_reset()?;
    info!("Factory reset completed!");
    Ok(())
}

pub fn validate_ota_message(params: &OtaParams) -> Result<()> {
    info!("Checking signature...");
    check_signature(params)?;
    info!("Good signature, checking url...");
    let configuration = Configuration {
        buffer_size: Some(BUFFER_LEN / 3),
        buffer_size_tx: Some(BUFFER_LEN / 3),
        follow_redirects_policy: FollowNone,
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        ..Default::default()
    };
    let mut reader = EspHttpConnection::new(&configuration)?;
    let full_url = params_to_url(params);
    info!("Pinging this url for an update: {}", full_url);
    reader.initiate_request(Method::Get, &full_url, &[])?;
    reader.initiate_response()?;
    let status = reader.status();
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

fn params_to_url(params: &OtaParams) -> String {
    let mut url = params.url.clone();
    url.push_str(&params.version.to_string());
    url
}
