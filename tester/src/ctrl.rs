use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use sphinx_key_parser::control::{ControlMessage, Controller};
use sphinx_key_signer::lightning_signer::bitcoin::Network;
use std::env;
use std::time::Duration;

const URL: &str = "http://localhost:8000/api";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EcdhBody {
    pub pubkey: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let seed_string: String = env::var("SEED").expect("no seed");
    let seed = hex::decode(seed_string).expect("yo");
    let mut ctrl = controller_from_seed(&Network::Regtest, &seed);

    let msg = ctrl.build_msg(ControlMessage::Nonce)?;
    let msg_hex = hex::encode(&msg);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("couldnt build reqwest client");

    let res = client
        .post(format!("{}/control?msg={}", URL, msg_hex))
        .header("Content-Type", "application/json")
        .send()
        .await?;

    let response: String = res.text().await?;
    let res_bytes = hex::decode(response).expect("couldnt decode response");

    let resp = ctrl.parse_response(&res_bytes).expect("nope");
    println!("RESponse from the ESP!!! {:?}", resp);

    Ok(())
}

pub fn controller_from_seed(network: &Network, seed: &[u8]) -> Controller {
    let (pk, sk) = sphinx_key_signer::derive_node_keys(network, seed);
    Controller::new(sk, pk, 0)
}
