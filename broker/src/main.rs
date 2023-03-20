// #![feature(once_cell)]
mod chain_tracker;
mod error_log;
mod mqtt;
mod routes;
mod run_test;
mod unix_fd;
mod util;

use crate::chain_tracker::MqttSignerPort;
use crate::mqtt::{check_auth, start_broker};
use crate::unix_fd::SignerLoop;
use crate::util::read_broker_config;
use clap::{arg, App};
use rocket::tokio::{
    self,
    sync::{broadcast, mpsc, oneshot},
};
use rumqttd::AuthMsg;
use std::env;
use std::sync::Arc;
use url::Url;
use vls_frontend::{frontend::SourceFactory, Frontend};
use vls_proxy::client::UnixClient;
use vls_proxy::connection::{open_parent_fd, UnixConnection};
use vls_proxy::portfront::SignerPortFront;
use vls_proxy::util::{add_hsmd_args, handle_hsmd_version};

pub struct Connections {
    pub pubkey: Option<String>,
    pub clients: Vec<String>,
}

impl Connections {
    pub fn new() -> Self {
        Self {
            pubkey: None,
            clients: Vec::new(),
        }
    }
    pub fn set_pubkey(&mut self, pk: &str) {
        self.pubkey = Some(pk.to_string())
    }
}

pub struct Channel {
    pub sequence: u16,
    pub sender: mpsc::Sender<ChannelRequest>,
    pub pubkey: [u8; 33],
}

/// Responses are received on the oneshot sender
#[derive(Debug)]
pub struct ChannelRequest {
    pub topic: String,
    pub message: Vec<u8>,
    pub reply_tx: oneshot::Sender<ChannelReply>,
}
impl ChannelRequest {
    pub fn new(topic: &str, message: Vec<u8>) -> (Self, oneshot::Receiver<ChannelReply>) {
        let (reply_tx, reply_rx) = oneshot::channel();
        let cr = ChannelRequest {
            topic: topic.to_string(),
            message,
            reply_tx,
        };
        (cr, reply_rx)
    }
}

// mpsc reply
#[derive(Debug)]
pub struct ChannelReply {
    pub reply: Vec<u8>,
}

// const CLIENT_ID: &str = "sphinx-1";
const BROKER_CONFIG_PATH: &str = "../broker.conf";

#[rocket::launch]
async fn rocket() -> _ {
    let parent_fd = open_parent_fd();

    util::setup_logging("hsmd  ", "info");
    let app = make_clap_app();
    let matches = app.get_matches();
    if matches.is_present("git-desc") {
        println!("remote_hsmd_socket git_desc={}", vls_proxy::GIT_DESC);
        panic!("end")
    }
    if handle_hsmd_version(&matches) {
        panic!("end")
    }

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

fn make_clap_app() -> App<'static> {
    let app = App::new("signer")
        .about("CLN:mqtt - connects to a remote signer via MQTT")
        .arg(arg!(--test "run a test against the embedded device"));
    add_hsmd_args(app)
}

async fn run_main(parent_fd: i32) -> rocket::Rocket<rocket::Build> {
    let settings = read_broker_config(BROKER_CONFIG_PATH);

    let (mqtt_tx, mqtt_rx) = mpsc::channel(10000);
    let (auth_tx, auth_rx) = std::sync::mpsc::channel::<AuthMsg>();
    // let (unix_tx, mut unix_rx) = mpsc::channel(10000);
    let (status_tx, mut status_rx) = mpsc::channel(10000);
    let (error_tx, error_rx) = broadcast::channel(10000);
    error_log::log_errors(error_rx);

    let mut conns = Connections::new();

    std::thread::spawn(move || {
        while let Ok(am) = auth_rx.recv() {
            let ok = check_auth(&am.username, &am.password, &mut conns);
            let _ = am.reply.send(ok);
        }
    });

    log::info!("=> start broker on network: {}", settings.network);
    start_broker(mqtt_rx, status_tx, error_tx.clone(), settings, auth_tx)
        .expect("BROKER FAILED TO START");
    log::info!("=> wait for connected status");
    // wait for connection = true
    let status = status_rx.recv().await.expect("couldnt receive");
    log::info!("=> connected: {}: {}", status.0, status.1);

    // let mqtt_tx_ = mqtt_tx.clone();
    // tokio::spawn(async move {
    //     while let Some(msg) = unix_rx.recv().await {
    //         // update LSS here?
    //         if let Err(e) = mqtt_tx_.send(msg).await {
    //             log::error!("failed to send on mqtt_tx {:?}", e);
    //         }
    //     }
    // });

    if let Ok(btc_url) = env::var("BITCOIND_RPC_URL") {
        let signer_port = Box::new(MqttSignerPort::new(mqtt_tx.clone()));
        let port_front = SignerPortFront::new(signer_port, settings.network);
        let source_factory = Arc::new(SourceFactory::new(".", settings.network));
        let frontend = Frontend::new(
            Arc::new(port_front),
            source_factory,
            Url::parse(&btc_url).expect("malformed btc rpc url"),
        );
        tokio::spawn(async move {
            frontend.start();
        });
    }
    let conn = UnixConnection::new(parent_fd);
    let client = UnixClient::new(conn);
    // TODO pass status_rx into SignerLoop
    let mut signer_loop = SignerLoop::new(client, mqtt_tx.clone());
    // spawn CLN listener on a std thread
    std::thread::spawn(move || {
        signer_loop.start(Some(settings));
    });

    routes::launch_rocket(mqtt_tx, error_tx, settings)
}
