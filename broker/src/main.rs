#![feature(once_cell)]
mod init;
mod mqtt;
mod run_test;
mod unix_fd;
mod util;

use crate::mqtt::start_broker;
use crate::unix_fd::SignerLoop;
use clap::{App, AppSettings, Arg, arg};
use std::env;
use tokio::sync::{mpsc, oneshot};
use vls_proxy::client::UnixClient;
use vls_proxy::connection::{open_parent_fd, UnixConnection};
use bitcoin::Network;

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
                .default_value("regtest")
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
    } else {
        let (tx, rx) = mpsc::channel(1000);
        let (status_tx, mut status_rx) = mpsc::channel(1000);
        log::info!("=> start broker");
        let _runtime = start_broker(rx, status_tx, "sphinx-1");
        log::info!("=> wait for connected status");
        // wait for connection = true
        let status = status_rx.blocking_recv().expect("couldnt receive");
        log::info!("=> connection status: {}", status);
        assert_eq!(status, true, "expected connected = true");
        // runtime.block_on(async {
        init::blocking_connect(tx.clone(), network);
        log::info!("=====> sent seed!");

        // listen to reqs from CLN
        let conn = UnixConnection::new(parent_fd);
        let client = UnixClient::new(conn);
        // TODO pass status_rx into SignerLoop
        let mut signer_loop = SignerLoop::new(client, tx);
        signer_loop.start();
        // })
    }

    Ok(())
}
