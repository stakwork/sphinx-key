use crate::conn;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use esp_idf_svc::nvs::*;
use esp_idf_svc::wifi::*;

use esp_idf_svc::hal::peripheral;

use sphinx_crypter::chacha::{decrypt, PAYLOAD_LEN};
use sphinx_crypter::ecdh::{derive_shared_secret_from_slice, PUBLIC_KEY_LEN};
use sphinx_crypter::secp256k1::rand::thread_rng;
use sphinx_crypter::secp256k1::{PublicKey, Secp256k1, SecretKey};

use sphinx_signer::sphinx_glyph::control::Config;
// #[derive(Clone, Debug, Deserialize, Serialize)]
// pub struct Config {
//     pub broker: String,
//     pub ssid: String,
//     pub pass: String,
//     pub seed: [u8; 32],
//     pub network: String,
// }
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConfigDTO {
    pub broker: String,
    pub ssid: String,
    pub pass: String,
    pub network: String,
    pub pubkey: Option<String>,
    pub seed: Option<String>, // encrypted (56 bytes)
}

/*
52.91.253.115:1883
arp -a
*/

pub fn start_wifi_client(
    modem: impl peripheral::Peripheral<P = esp_idf_svc::hal::modem::Modem> + 'static,
    default_nvs: EspDefaultNvsPartition,
    config: &Config,
) -> Result<BlockingWifi<EspWifi>> {
    let wifi = conn::wifi::start_client(modem, default_nvs, config)?;
    println!("CLIENT CONNECTED!!!!!! {:?}", wifi.is_connected());
    Ok(wifi)
}

pub fn ecdh_keypair() -> (SecretKey, PublicKey) {
    let s = Secp256k1::new();
    s.generate_keypair(&mut thread_rng())
}

pub fn decrypt_seed(dto: ConfigDTO, sk1: SecretKey) -> Result<(Config, Option<[u8; 32]>)> {
    let mut seed = None;
    if let Some(pubkey) = dto.pubkey {
        if let Some(seed_in) = dto.seed {
            let their_pk = hex::decode(pubkey)?;
            let their_pk_bytes: [u8; PUBLIC_KEY_LEN] = their_pk[..PUBLIC_KEY_LEN].try_into()?;
            let shared_secret =
                derive_shared_secret_from_slice(their_pk_bytes, sk1.secret_bytes())?;
            // decrypt seed
            let cipher_seed = hex::decode(seed_in)?;
            let cipher: [u8; PAYLOAD_LEN] = cipher_seed[..PAYLOAD_LEN].try_into()?;
            seed = Some(decrypt(cipher, shared_secret)?);
        }
    }
    Ok((
        Config {
            broker: dto.broker,
            ssid: dto.ssid,
            pass: dto.pass,
            network: dto.network,
        },
        seed,
    ))
}

pub fn start_config_server_and_wait(
    modem: impl peripheral::Peripheral<P = esp_idf_svc::hal::modem::Modem> + 'static,
    default_nvs: EspDefaultNvsPartition,
    has_stored_seed: bool,
) -> Result<(BlockingWifi<EspWifi<'static>>, Config, Option<[u8; 32]>)> {
    let mutex = Arc::new((Mutex::new(None), Condvar::new()));

    #[allow(clippy::redundant_clone)]
    #[allow(unused_mut)]
    let mut wifi = conn::wifi::start_access_point(modem, default_nvs.clone())?;

    let httpd = conn::http::config_server(mutex.clone(), has_stored_seed)?;
    let mut wait = mutex.0.lock().unwrap();
    if has_stored_seed {
        log::info!("Waiting for wifi details from the phone!");
    } else {
        log::info!("Waiting for seed and data from the phone!");
    }

    let config_seed_tuple: &(Config, Option<[u8; 32]>) = loop {
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
    println!("===> config! {:?}", config_seed_tuple.0);
    Ok((wifi, config_seed_tuple.0.clone(), config_seed_tuple.1))
}
