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
use crate::util::{read_broker_config, Settings};
use clap::{arg, App};
use rocket::tokio::{
    self,
    sync::{broadcast, mpsc, oneshot},
};
use rumqttd::{oneshot as std_oneshot, AuthMsg};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::{Arc, Mutex};
use url::Url;
use vls_frontend::{frontend::SourceFactory, Frontend};
use vls_proxy::client::UnixClient;
use vls_proxy::connection::{open_parent_fd, UnixConnection};
use vls_proxy::portfront::SignerPortFront;
use vls_proxy::util::{add_hsmd_args, handle_hsmd_version};

#[derive(Debug, Serialize, Deserialize)]
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
    pub fn add_client(&mut self, cid: &str) {
        let cids = cid.to_string();
        if !self.clients.contains(&cids) {
            self.clients.push(cids)
        }
    }
    pub fn remove_client(&mut self, cid: &str) {
        let cids = cid.to_string();
        if self.clients.contains(&cids) {
            self.clients.retain(|x| x != cid)
        }
    }
    pub fn client_action(&mut self, cid: &str, connected: bool) {
        if connected {
            self.add_client(cid);
        } else {
            self.remove_client(cid);
        }
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
    pub cid: Option<String>, // if it exists, only try the one client
}
impl ChannelRequest {
    pub fn new(topic: &str, message: Vec<u8>) -> (Self, oneshot::Receiver<ChannelReply>) {
        let (reply_tx, reply_rx) = oneshot::channel();
        let cr = ChannelRequest {
            topic: topic.to_string(),
            message,
            reply_tx,
            cid: None,
        };
        (cr, reply_rx)
    }
    pub fn for_cid(&mut self, cid: &str) {
        self.cid = Some(cid.to_string())
    }
    pub fn new_for(
        cid: &str,
        topic: &str,
        message: Vec<u8>,
    ) -> (Self, oneshot::Receiver<ChannelReply>) {
        let (mut cr, reply_rx) = ChannelRequest::new(topic, message);
        cr.for_cid(cid);
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
            run_test::run_test()
        } else {
            run_main(parent_fd)
        }
    }
}

fn make_clap_app() -> App<'static> {
    let app = App::new("signer")
        .about("CLN:mqtt - connects to a remote signer via MQTT")
        .arg(arg!(--test "run a test against the embedded device"));
    add_hsmd_args(app)
}

// blocks until a connection received
pub fn main_setup(
    settings: Settings,
    mqtt_rx: mpsc::Receiver<ChannelRequest>,
    error_tx: broadcast::Sender<Vec<u8>>,
) -> Arc<Mutex<Connections>> {
    let (auth_tx, auth_rx) = std::sync::mpsc::channel::<AuthMsg>();
    let (status_tx, status_rx) = std::sync::mpsc::channel();

    let conns1 = Connections::new();
    let conns = Arc::new(Mutex::new(conns1));

    // authenticator
    let conns_ = conns.clone();
    std::thread::spawn(move || {
        while let Ok(am) = auth_rx.recv() {
            let mut cs = conns_.lock().unwrap();
            let ok = check_auth(&am.username, &am.password, &mut cs);
            let _ = am.reply.send(ok);
        }
    });

    // broker
    log::info!("=> start broker on network: {}", settings.network);
    start_broker(
        settings,
        mqtt_rx,
        status_tx,
        error_tx.clone(),
        auth_tx,
        conns.clone(),
    )
    .expect("BROKER FAILED TO START");

    // client connections state
    let (startup_tx, startup_rx) = std_oneshot::channel();
    let conns_ = conns.clone();
    std::thread::spawn(move || {
        log::info!("=> wait for connected status");
        // wait for connection = true
        let (cid, connected) = status_rx.recv().expect("couldnt receive");
        let mut cs = conns_.lock().unwrap();
        cs.client_action(&cid, connected);
        drop(cs);
        log::info!("=> connected: {}: {}", cid, connected);
        let _ = startup_tx.send(true);
        while let Ok((cid, connected)) = status_rx.recv() {
            let mut cs = conns_.lock().unwrap();
            cs.client_action(&cid, connected);
            drop(cs)
        }
    });
    let _ = startup_rx.recv();

    conns
}

fn run_main(parent_fd: i32) -> rocket::Rocket<rocket::Build> {
    let settings = read_broker_config(BROKER_CONFIG_PATH);

    let (mqtt_tx, mqtt_rx) = mpsc::channel(10000);
    let (error_tx, error_rx) = broadcast::channel(10000);
    error_log::log_errors(error_rx);

    let conns = main_setup(settings, mqtt_rx, error_tx.clone());

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
    } else {
        log::warn!("Running without a frontend")
    }
    let cln_client = UnixClient::new(UnixConnection::new(parent_fd));
    // TODO pass status_rx into SignerLoop?
    let mut signer_loop = SignerLoop::new(cln_client, mqtt_tx.clone());
    // spawn CLN listener
    std::thread::spawn(move || {
        signer_loop.start(Some(settings));
    });

    routes::launch_rocket(mqtt_tx, error_tx, settings, conns)
}
