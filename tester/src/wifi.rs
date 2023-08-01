use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

use sphinx_signer::sphinx_glyph::control::Config;

const URL: &str = "http://192.168.71.1";

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

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("couldnt build reqwest client");

    let config = Config {
        ssid,
        pass,
        broker,
        network,
    };

    let conf_string = serde_json::to_string(&config)?;
    let conf_encoded = urlencoding::encode(&conf_string).to_owned();

    let res = client
        .post(format!("{}/{}={}", url, "config?config", conf_encoded))
        .send()
        .await?;
    let conf_res: ConfigResponse = res.json().await?;

    if conf_res.success {
        println!("SUCCESS!")
    }

    Ok(())
}
