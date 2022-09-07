#![feature(once_cell)]
mod chain_tracker;
mod mqtt;
mod routes;
mod run_test;
mod unix_fd;
mod util;

use crate::chain_tracker::MqttSignerPort;
use crate::mqtt::start_broker;
use crate::unix_fd::SignerLoop;
use crate::util::read_broker_config;
use clap::{App, AppSettings, Arg};
use std::env;
use std::sync::Arc;
use rocket::tokio::{self, sync::{mpsc, oneshot}};
use url::Url;
use vls_frontend::Frontend;
use vls_proxy::client::UnixClient;
use vls_proxy::connection::{open_parent_fd, UnixConnection};
use vls_proxy::portfront::SignerPortFront;

pub struct Channel {
    pub sequence: u16,
    pub sender: mpsc::Sender<ChannelRequest>,
}

/// Responses are received on the oneshot sender
#[derive(Debug)]
pub struct ChannelRequest {
    pub message: Vec<u8>,
    pub reply_tx: oneshot::Sender<ChannelReply>,
}

// mpsc reply
#[derive(Debug)]
pub struct ChannelReply {
    pub reply: Vec<u8>,
}

const BROKER_CONFIG_PATH: &str = "../broker.conf";

#[rocket::launch]
async fn rocket() -> _ {
    let parent_fd = open_parent_fd();

    util::setup_logging("hsmd  ", "info");
    let app = App::new("signer")
        .setting(AppSettings::NoAutoVersion)
        .about("CLN:mqtt - connects to an embedded VLS over a MQTT connection")
        .arg(
            Arg::new("--dev-disconnect")
                .about("ignored dev flag")
                .long("dev-disconnect")
                .takes_value(true),
        )
        .arg(Arg::from("--log-io ignored dev flag"))
        .arg(Arg::from("--version show a dummy version"))
        .arg(Arg::from("--test run a test against the embedded device"));

    let matches = app.get_matches();

    if matches.is_present("version") {
        // Pretend to be the right version, given to us by an env var
        let version =
            env::var("GREENLIGHT_VERSION").expect("set GREENLIGHT_VERSION to match c-lightning");
        println!("{}", version);
        panic!("end")
    } else {
        if matches.is_present("test") {
            run_test::run_test().await
        } else {
            run_main(parent_fd).await
        }
    }
}

async fn run_main(parent_fd: i32) -> rocket::Rocket<rocket::Build> {
    let settings = read_broker_config(BROKER_CONFIG_PATH);

    let (tx, rx) = mpsc::channel(1000);
    let (status_tx, mut status_rx) = mpsc::channel(1000);
    log::info!("=> start broker on network: {}", settings.network);
    start_broker(rx, status_tx, "sphinx-1", &settings).await;
    log::info!("=> wait for connected status");
    // wait for connection = true
    let status = status_rx.recv().await.expect("couldnt receive");
    log::info!("=> connection status: {}", status);
    assert_eq!(status, true, "expected connected = true");

    if let Ok(btc_url) = env::var("BITCOIND_RPC_URL") {
        let signer_port = MqttSignerPort::new(tx.clone());
        let frontend = Frontend::new(
            Arc::new(SignerPortFront {
                signer_port: Box::new(signer_port),
                network: settings.network,
            }),
            Url::parse(&btc_url).expect("malformed btc rpc url"),
        );
        tokio::spawn(async move {
            frontend.start();
        });
    }
    // listen to reqs from CLN
    let conn = UnixConnection::new(parent_fd);
    let client = UnixClient::new(conn);
    // TODO pass status_rx into SignerLoop
    let mut signer_loop = SignerLoop::new(client, tx.clone());
    std::thread::spawn(move || {
        signer_loop.start(Some(&settings));
    });

    routes::launch_rocket(tx)
}
