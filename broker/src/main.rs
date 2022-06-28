#![feature(once_cell)]
mod chain_tracker;
mod init;
mod mqtt;
mod run_test;
mod unix_fd;
mod util;

use crate::chain_tracker::MqttSignerPort;
use crate::mqtt::start_broker;
use crate::unix_fd::SignerLoop;
use bitcoin::Network;
use clap::{arg, App, AppSettings, Arg};
use std::env;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
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

fn main() -> anyhow::Result<()> {
    let parent_fd = open_parent_fd();

    util::setup_logging("hsmd  ", "info");
    let app = App::new("signer")
        .setting(AppSettings::NoAutoVersion)
        .about("CLN:mqtt - connects to an embedded VLS over a MQTT connection")
        .arg(
            Arg::new("dev-disconnect")
                .help("ignored dev flag")
                .long("dev-disconnect")
                .takes_value(true),
        )
        .arg(arg!(--"log-io" "ignored dev flag"))
        .arg(arg!(--version "show a dummy version"))
        .arg(arg!(--test "run a test against the embedded device"))
        .arg(
            Arg::new("network")
                .help("bitcoin network")
                .long("network")
                .value_parser(["regtest", "signet", "testnet", "mainnet", "bitcoin"])
                .default_value("regtest"),
        );

    let matches = app.get_matches();

    let network_string: &String = matches.get_one("network").expect("expected a network");
    let network: Network = match network_string.as_str() {
        "bitcoin" => Network::Bitcoin,
        "mainnet" => Network::Bitcoin,
        "testnet" => Network::Testnet,
        "signet" => Network::Signet,
        "regtest" => Network::Regtest,
        _ => Network::Regtest,
    };

    if matches.is_present("version") {
        // Pretend to be the right version, given to us by an env var
        let version =
            env::var("GREENLIGHT_VERSION").expect("set GREENLIGHT_VERSION to match c-lightning");
        println!("{}", version);
        return Ok(());
    }

    log::info!("NETWORK: {}", network.to_string());
    if matches.is_present("test") {
        run_test::run_test();
        return Ok(());
    }

    let (tx, rx) = mpsc::channel(1000);
    let (status_tx, mut status_rx) = mpsc::channel(1000);
    log::info!("=> start broker");
    let runtime = start_broker(rx, status_tx, "sphinx-1");
    log::info!("=> wait for connected status");
    // wait for connection = true
    let status = status_rx.blocking_recv().expect("couldnt receive");
    log::info!("=> connection status: {}", status);
    assert_eq!(status, true, "expected connected = true");
    // runtime.block_on(async {
    init::blocking_connect(tx.clone(), network);
    log::info!("=====> sent seed!");

    if let Ok(btc_url) = env::var("BITCOIND_RPC_URL") {
        let signer_port = MqttSignerPort::new(tx.clone());
        let frontend = Frontend::new(
            Arc::new(SignerPortFront {
                signer_port: Box::new(signer_port),
            }),
            Url::parse(&btc_url).expect("malformed btc rpc url"),
        );
        runtime.block_on(async {
            frontend.start();
        });
    }
    // listen to reqs from CLN
    let conn = UnixConnection::new(parent_fd);
    let client = UnixClient::new(conn);
    // TODO pass status_rx into SignerLoop
    let mut signer_loop = SignerLoop::new(client, tx);
    signer_loop.start();
    // })

    Ok(())
}
