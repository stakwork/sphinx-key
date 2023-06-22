// #![feature(once_cell)]
mod chain_tracker;
mod error_log;
mod mqtt;
mod routes;
mod run_test;
mod looper;
mod util;
mod conn;
mod lss;

use crate::conn::{Connections, ChannelRequest, LssReq};
use crate::chain_tracker::MqttSignerPort;
use crate::mqtt::{check_auth, start_broker};
use crate::looper::SignerLoop;
use crate::util::{read_broker_config, Settings};
use clap::{arg, App};
use rocket::tokio::{
    self,
    sync::{broadcast, mpsc},
};
use rumqttd::{oneshot as std_oneshot, AuthMsg};
use std::{env, time::Duration};
use std::sync::{Arc, Mutex};
use url::Url;
use vls_frontend::{frontend::SourceFactory, Frontend};
use vls_proxy::client::UnixClient;
use vls_proxy::connection::{open_parent_fd, UnixConnection};
use vls_proxy::portfront::SignerPortFront;
use vls_proxy::util::{add_hsmd_args, handle_hsmd_version};

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
    let settings = read_broker_config();

    let (mqtt_tx, mqtt_rx) = mpsc::channel(10000);
    let (init_tx, init_rx) = mpsc::channel(10000);
    let (error_tx, error_rx) = broadcast::channel(10000);
    error_log::log_errors(error_rx);

    let (reconn_tx, reconn_rx) = mpsc::channel::<(String, bool)>(10000);

    // waits until first connection
    let conns = broker_setup(settings, mqtt_rx, init_rx, reconn_tx.clone(), error_tx.clone()).await;

    tokio::time::sleep(Duration::from_secs(1)).await;

    let (lss_tx, lss_rx) = mpsc::channel::<LssReq>(10000);
    let _lss_broker = if let Ok(lss_uri) = env::var("VLS_LSS") {
        // waits until LSS confirmation from signer
        let lss_broker = loop {
            match lss::lss_setup(&lss_uri, init_tx.clone()).await{
                Ok(l) => {
                    break l;
                },
                Err(e) => {
                    let _ = error_tx.send(e.to_string().as_bytes().to_vec());
                    log::error!("failed LSS setup, trying again...");
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            }
        };
        lss::lss_tasks(lss_broker.clone(), lss_rx, reconn_rx, init_tx);
        log::info!("=> lss broker connection created!");
        Some(lss_broker)
    } else {
        log::warn!("running without LSS");
        None
    };

    if let Ok(btc_url) = env::var("BITCOIND_RPC_URL") {
        let signer_port = MqttSignerPort::new(mqtt_tx.clone());
        let port_front = SignerPortFront::new(Arc::new(signer_port), settings.network);
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

    // test sleep FIXME
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let cln_client = UnixClient::new(UnixConnection::new(parent_fd));
    // TODO pass status_rx into SignerLoop?
    let mut signer_loop = SignerLoop::new(cln_client, lss_tx.clone(), mqtt_tx.clone());
    // spawn CLN listener
    std::thread::spawn(move || {
        signer_loop.start(Some(settings));
    });

    routes::launch_rocket(mqtt_tx, error_tx, settings, conns)
}

// blocks until a connection received
pub async fn broker_setup(
    settings: Settings,
    mqtt_rx: mpsc::Receiver<ChannelRequest>,
    init_rx: mpsc::Receiver<ChannelRequest>,
    reconn_tx: mpsc::Sender<(String, bool)>,
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
        init_rx,
        status_tx,
        error_tx.clone(),
        auth_tx,
        conns.clone(),
    )
    .expect("BROKER FAILED TO START");

    // client connections state
    let (startup_tx, startup_rx) = std_oneshot::channel::<String>();
    let conns_ = conns.clone();
    let reconn_tx_ = reconn_tx.clone();
    std::thread::spawn(move || {
        log::info!("=> wait for connected status");
        // wait for connection = true
        let (cid, connected) = status_rx.recv().expect("couldnt receive");
        let mut cs = conns_.lock().unwrap();
        cs.client_action(&cid, connected);
        drop(cs);
        log::info!("=> connected: {}: {}", cid, connected);
        let _ = startup_tx.send(cid.to_string());
        while let Ok((cid, connected)) = status_rx.recv() {
            log::info!("=> reconnected: {}: {}", cid, connected);
            let mut cs = conns_.lock().unwrap();
            cs.client_action(&cid, connected);
            drop(cs);
            let _ = reconn_tx_.blocking_send((cid, connected));
        }
    });
    let _first_client_id = startup_rx.recv();

    conns
}

