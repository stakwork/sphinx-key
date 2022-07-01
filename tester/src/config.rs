use rand::{rngs::OsRng, thread_rng, RngCore};
use serde::{Deserialize, Serialize};
use sphinx_key_crypter::chacha::{encrypt, MSG_LEN, NONCE_END_LEN};
use sphinx_key_crypter::ecdh::{derive_shared_secret_from_slice, PUBLIC_KEY_LEN};
use sphinx_key_crypter::secp256k1::Secp256k1;
use std::convert::TryInto;
use std::time::Duration;
use dotenv::dotenv;
use std::env;

const URL: &str = "http://192.168.71.1/";

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
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigResponse {
  pub success: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  dotenv().ok();

  let ssid: String = env::var("SSID").expect("no ssid");
  let pass: String = env::var("PASS").expect("no pass");
  let broker: String = env::var("BROKER").expect("no broker");

  let s = Secp256k1::new();
  let (sk1, pk1) = s.generate_keypair(&mut thread_rng());

  let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(10))
    .build()
    .expect("couldnt build reqwest client");

  let body = EcdhBody {
    pubkey: hex::encode(pk1.serialize()),
  };
  let res = client
    .post(format!("{}{}", URL, "ecdh"))
    .json(&body)
    .header("Content-Type", "application/json")
    .send()
    .await?;
  let their_ecdh: EcdhBody = res.json().await?;
  let their_pk = hex::decode(their_ecdh.pubkey)?;

  let their_pk_bytes: [u8; PUBLIC_KEY_LEN] = their_pk[..PUBLIC_KEY_LEN].try_into()?;
  let shared_secret = derive_shared_secret_from_slice(their_pk_bytes, sk1.secret_bytes())?;

  let plaintext = [1; MSG_LEN];
  let mut nonce_end = [0; NONCE_END_LEN];
  OsRng.fill_bytes(&mut nonce_end);
  let cipher = encrypt(plaintext, shared_secret, nonce_end)?;

  let cipher_seed = hex::encode(cipher);
  let config = ConfigBody {
    seed: cipher_seed,
    ssid, pass, broker,
  };

  let conf_string = serde_json::to_string(&config)?;
  let conf_encoded = urlencoding::encode(&conf_string).to_owned();

  let res2 = client
    .post(format!("{}{}{}", URL, "/config?config=", conf_encoded))
    .send()
    .await?;
  let conf_res: ConfigResponse = res2.json().await?;

  if conf_res.success {
    println!("SUCCESS!")
  }

  Ok(())
}
