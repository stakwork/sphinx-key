mod chain_tracker;
mod conn;
mod error_log;
mod handle;
mod looper;
mod lss;
mod mqtt;
mod routes;
mod run_test;
mod util;

pub(crate) use sphinx_signer::lightning_signer::bitcoin::{self, secp256k1};

use crate::bitcoin::blockdata::constants::ChainHash;
use crate::chain_tracker::MqttSignerPort;
use crate::conn::{conns_set_pubkey, current_pubkey, new_connection, ChannelRequest, LssReq};
use crate::looper::SignerLoop;
use crate::mqtt::{check_auth, start_broker};
use crate::util::{read_broker_config, Settings};
use clap::{arg, App};
use rocket::tokio::{
    self, select,
    sync::{broadcast, mpsc},
    task::JoinSet,
};
use rocket::{Build, Rocket};
use rumqttd::{oneshot as std_oneshot, AuthMsg, AuthType};
use std::env;
use std::sync::Arc;
use url::Url;
use vls_frontend::{frontend::SourceFactory, Frontend};
use vls_protocol::{msgs, msgs::Message};
use vls_proxy::client::{Client, UnixClient};
use vls_proxy::connection::{open_parent_fd, UnixConnection};
use vls_proxy::portfront::SignerPortFront;
use vls_proxy::util::{add_hsmd_args, handle_hsmd_version};

#[rocket::main]
async fn main() {
    let mut task_set: JoinSet<()> = JoinSet::new();
    let web_server = rocket(&mut task_set).await;
    select! {
        _ = task_set.join_next() => {
            log::warn!("a spawned task shut down");
        }
        _ = web_server.launch() => {
            log::warn!("the rocket web server shut down");
        }

    };
    log::info!("shutting down");
}

async fn rocket(task_set: &mut JoinSet<()>) -> Rocket<Build> {
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
    } else if matches.is_present("test") {
        run_test::run_test()
    } else {
        run_main(parent_fd, task_set)
    }
}

fn make_clap_app() -> App<'static> {
    let app = App::new("signer")
        .about("CLN:mqtt - connects to a remote signer via MQTT")
        .arg(arg!(--test "run a test against the embedded device"));
    add_hsmd_args(app)
}

fn run_main(parent_fd: i32, task_set: &mut JoinSet<()>) -> rocket::Rocket<rocket::Build> {
    let settings = read_broker_config();

    let (mqtt_tx, mqtt_rx) = mpsc::channel(10000);
    let (init_tx, init_rx) = mpsc::channel(10000);
    let (error_tx, error_rx) = broadcast::channel(10000);
    error_log::log_errors(error_rx, task_set);

    let (conn_tx, conn_rx) = mpsc::channel::<(String, std_oneshot::Sender<bool>)>(10000);

    broker_setup(
        settings,
        mqtt_rx,
        init_rx,
        conn_tx,
        error_tx.clone(),
        task_set,
    );

    let mut cln_client_a = UnixClient::new(UnixConnection::new(parent_fd));
    let hsmd_raw = cln_client_a.read_raw().unwrap();
    let msg = msgs::from_vec(hsmd_raw.clone()).unwrap();
    let Message::HsmdInit(ref m) = msg else {
        panic!("Expected a hsmd init message first");
    };
    if ChainHash::using_genesis_block(settings.network).as_bytes() != m.chain_params.as_ref() {
        panic!("The network settings of CLN and broker don't match!");
    }
    let (lss_tx, lss_rx) = mpsc::channel::<LssReq>(10000);
    // TODO: add a validation here of the uri setting to make sure LSS is running
    if let Ok(lss_uri) = env::var("VLS_LSS") {
        log::info!("Spawning lss tasks...");
        lss::lss_tasks(
            lss_uri,
            lss_rx,
            conn_rx,
            init_tx,
            cln_client_a,
            hsmd_raw,
            task_set,
        );
    } else {
        log::warn!("running without LSS");
    }

    if let Ok(btc_url) = env::var("BITCOIND_RPC_URL") {
        let signer_port = MqttSignerPort::new(mqtt_tx.clone(), lss_tx.clone());
        let port_front = SignerPortFront::new(Arc::new(signer_port), settings.network);
        let source_factory = Arc::new(SourceFactory::new(".", settings.network));
        let (_trigger, listener) = triggered::trigger();
        let frontend = Frontend::new(
            Arc::new(port_front),
            source_factory,
            Url::parse(&btc_url).expect("malformed btc rpc url"),
            listener,
        );
        tokio::spawn(async move {
            frontend.start();
        });
    } else {
        log::warn!("Running without a frontend")
    }

    // tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    std::thread::sleep(std::time::Duration::from_secs(1));

    // handle::handle_message(&None, vec![], &mqtt_tx, &lss_tx);

    let cln_client = UnixClient::new(UnixConnection::new(parent_fd));
    // TODO pass status_rx into SignerLoop?
    let mut signer_loop = SignerLoop::new(cln_client, mqtt_tx.clone(), lss_tx);
    // spawn CLN listener
    task_set.spawn_blocking(move || {
        signer_loop.start();
    });

    routes::launch_rocket(mqtt_tx, error_tx, settings)
}

// blocks until a connection received
pub fn broker_setup(
    settings: Settings,
    mqtt_rx: mpsc::Receiver<ChannelRequest>,
    init_rx: mpsc::Receiver<ChannelRequest>,
    conn_tx: mpsc::Sender<(String, std_oneshot::Sender<bool>)>,
    error_tx: broadcast::Sender<Vec<u8>>,
    task_set: &mut JoinSet<()>,
) {
    let (auth_tx, auth_rx) = std::sync::mpsc::channel::<AuthMsg>();
    let (status_tx, status_rx) = std::sync::mpsc::channel();

    // authenticator
    task_set.spawn_blocking(move || {
        while let Ok(am) = auth_rx.recv() {
            let pubkey = current_pubkey();
            let (ok, new_pubkey) = match am.msg {
                AuthType::Login(login) => check_auth(&login.username, &login.password, &pubkey),
                _ => (true, None),
            };
            if let Some(np) = new_pubkey {
                conns_set_pubkey(np);
            }
            let _ = am.tx.send(ok);
        }
    });

    // broker
    log::info!("=> start broker on network: {}", settings.network);
    start_broker(
        settings, mqtt_rx, init_rx, status_tx, error_tx, auth_tx, task_set,
    )
    .expect("BROKER FAILED TO START");

    // client connections state
    task_set.spawn_blocking(move || {
        log::info!("=> waiting first connection...");
        while let Ok((cid, connected)) = status_rx.recv() {
            log::info!("=> connection status: {}: {}", cid, connected);
            // drop it from list until ready
            new_connection(&cid, false);
            if connected {
                let (dance_complete_tx, dance_complete_rx) = std_oneshot::channel::<bool>();
                let _ = conn_tx.blocking_send((cid.clone(), dance_complete_tx));
                let dance_complete = dance_complete_rx.recv().unwrap_or_else(|e| {
                    log::info!(
                        "dance_complete channel died before receiving response: {}",
                        e
                    );
                    false
                });
                log::info!("adding client to the list? {}", dance_complete);
                new_connection(&cid, dance_complete);
            }
        }
    });
}
