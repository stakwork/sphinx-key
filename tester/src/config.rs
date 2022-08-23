use dotenv::dotenv;
use rand::{rngs::OsRng, thread_rng, RngCore};
use serde::{Deserialize, Serialize};
use sphinx_crypter::chacha::{encrypt, MSG_LEN, NONCE_LEN};
use sphinx_crypter::ecdh::{derive_shared_secret_from_slice, PUBLIC_KEY_LEN};
use sphinx_crypter::secp256k1::Secp256k1;
use std::convert::TryInto;
use std::env;
use std::time::Duration;

const URL: &str = "http://192.168.71.1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EcdhBody {
    pub pubkey: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigBody {
    pub seed: String,
    pub ssid: String,
    pub pass: String,
    pub broker: String,
    pub pubkey: String, // for ecdh
    pub network: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub success: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let url: String = env::var("URL").unwrap_or(URL.to_string());

    let ssid: String = env::var("SSID").expect("no ssid");
    let pass: String = env::var("PASS").expect("no pass");
    let broker: String = env::var("BROKER").expect("no broker");
    let seed_string: String = env::var("SEED").expect("no seed");
    let seed: [u8; MSG_LEN] = hex::decode(seed_string)?[..MSG_LEN].try_into()?;
    let network: String = env::var("NETWORK").unwrap_or("regtest".to_string());
    if !(network == "bitcoin"
        || network == "mainnet"
        || network == "testnet"
        || network == "signet"
        || network == "regtest")
    {
        panic!("invalid network string");
    }
    println!("network {:?}", network);

    let s = Secp256k1::new();
    let (sk1, pk1) = s.generate_keypair(&mut thread_rng());

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("couldnt build reqwest client");

    let res = client
        .get(format!("{}/{}", url, "ecdh"))
        .header("Content-Type", "application/json")
        .send()
        .await?;
    let their_ecdh: EcdhBody = res.json().await?;
    let their_pk = hex::decode(their_ecdh.pubkey)?;

    let their_pk_bytes: [u8; PUBLIC_KEY_LEN] = their_pk[..PUBLIC_KEY_LEN].try_into()?;
    let shared_secret = derive_shared_secret_from_slice(their_pk_bytes, sk1.secret_bytes())?;

    let mut nonce_end = [0; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_end);
    let cipher = encrypt(seed, shared_secret, nonce_end)?;

    let cipher_seed = hex::encode(cipher);
    let config = ConfigBody {
        seed: cipher_seed,
        ssid,
        pass,
        broker,
        network,
        pubkey: hex::encode(pk1.serialize()),
    };

    let conf_string = serde_json::to_string(&config)?;
    let conf_encoded = urlencoding::encode(&conf_string).to_owned();

    let res2 = client
        .post(format!("{}/{}={}", url, "config?config", conf_encoded))
        .send()
        .await?;
    let conf_res: ConfigResponse = res2.json().await?;

    if conf_res.success {
        println!("SUCCESS!")
    }

    Ok(())
}
