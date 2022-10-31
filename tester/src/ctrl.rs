use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use sphinx_signer::lightning_signer::bitcoin::Network;
use sphinx_signer::sphinx_glyph::control::{ControlMessage, Controller};
use std::env;
use std::time::Duration;

const DEFAULT_URL: &str = "http://localhost:8000/api";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EcdhBody {
    pub pubkey: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let nonce_string: String = env::var("NONCE").unwrap_or("0".to_string());
    let nonce: u64 = nonce_string.parse::<u64>().expect("failed to parse nonce");

    let broker_url: String = env::var("BROKER_URL").unwrap_or(DEFAULT_URL.to_string());

    let seed_string: String = env::var("SEED").expect("no seed");
    let seed = hex::decode(seed_string).expect("yo");
    let mut ctrl = controller_from_seed(&Network::Regtest, &seed, nonce);

    let mut command = ControlMessage::Nonce;
    if let Ok(cmd_content) = std::fs::read_to_string("./tester/cmd.json") {
        if let Ok(cmd) = serde_json::from_str::<ControlMessage>(&cmd_content) {
            command = cmd;
        }
    }

    println!("COMMAND! {:?}", command);

    let msg = ctrl.build_msg(command)?;
    let msg_hex = hex::encode(&msg);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("couldnt build reqwest client");

    let res = client
        .post(format!("{}/control?msg={}", broker_url, msg_hex))
        .header("Content-Type", "application/json")
        .send()
        .await?;

    let response: String = res.text().await?;
    let res_bytes = hex::decode(response).expect("couldnt decode response");

    let resp = ctrl.parse_response(&res_bytes).expect("nope");
    println!("RESponse from the ESP!!! {:?}", resp);

    Ok(())
}

pub fn controller_from_seed(network: &Network, seed: &[u8], nonce: u64) -> Controller {
    let (pk, sk) = sphinx_signer::derive_node_keys(network, seed);
    Controller::new(sk, pk, nonce)
}
