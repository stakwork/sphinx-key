mod init;
mod mqtt;
mod run_test;
mod unix_fd;

use crate::mqtt::start_broker;
use crate::unix_fd::SignerLoop;
use clap::{App, AppSettings, Arg};
use sphinx_key_parser as parser;
use std::env;
use tokio::sync::{mpsc, oneshot};
use vls_proxy::client::UnixClient;
use vls_proxy::connection::{open_parent_fd, UnixConnection};
use vls_proxy::util::setup_logging;

pub struct Channel {
    pub sequence: u16,
    pub sender: mpsc::Sender<ChannelRequest>,
}

/// Responses are received on the oneshot sender
pub struct ChannelRequest {
    pub message: Vec<u8>,
    pub reply_tx: oneshot::Sender<ChannelReply>,
}

// mpsc reply
pub struct ChannelReply {
    pub reply: Vec<u8>,
}

fn main() -> anyhow::Result<()> {
    let parent_fd = open_parent_fd();

    setup_logging("hsmd  ", "info");
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
        return Ok(());
    }

    if matches.is_present("test") {
        run_test::run_test();
    } else {
        let (tx, rx) = mpsc::channel(1000);
        let runtime = start_broker(true, rx);
        runtime.block_on(async {
            let init_msg_2 = crate::init::make_init_msg().expect("could make init msg");
            let (reply_tx, reply_rx) = oneshot::channel();
            // Send a request to the MQTT handler to send to signer
            let request = ChannelRequest {
                message: init_msg_2,
                reply_tx,
            };
            let _ = tx.send(request).await;
            let res = reply_rx.await.expect("couldnt receive");
            let reply =
                parser::response_from_bytes(res.reply, 0).expect("could parse init receive");
            println!("REPLY {:?}", reply);
            // listen to reqs from CLN
            let conn = UnixConnection::new(parent_fd);
            let client = UnixClient::new(conn);
            let mut signer_loop = SignerLoop::new(client, tx);
            signer_loop.start();
        })
    }

    Ok(())
}
