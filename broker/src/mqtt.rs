use crate::util::Settings;
use crate::{ChannelReply, ChannelRequest};
use librumqttd::{
    async_locallink,
    consolelink::{self, ConsoleLink},
    rumqttlog::router::ConnectionMetrics,
    Config,
};
use rocket::tokio::time::timeout;
use rocket::tokio::{self, sync::mpsc, sync::broadcast};
use sphinx_key_parser::topics;
use std::sync::Arc;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

// must get a reply within this time, or disconnects
const REPLY_TIMEOUT_MS: u64 = 10000;

static CONNECTED: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));
fn set_connected(b: bool) {
    *CONNECTED.lock().unwrap() = b;
}
fn get_connected() -> bool {
    *CONNECTED.lock().unwrap()
}

pub async fn start_broker(
    mut receiver: mpsc::Receiver<ChannelRequest>,
    status_sender: mpsc::Sender<bool>,
    error_sender: broadcast::Sender<Vec<u8>>,
    expected_client_id: &str,
    settings: Settings,
) {
    let config = config(settings);
    let client_id = expected_client_id.to_string();

    let (mut router, servers, builder) = async_locallink::construct(config.clone());

    // std thread for the router
    std::thread::spawn(move || {
        log::info!("start mqtt router");
        router.start().expect("could not start router");
    });

    tokio::spawn(async move {
        log::info!("start mqtt relayer and localclient");
        let (msg_tx, mut msg_rx): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) =
            mpsc::channel(1000);
        let (mut link_tx, mut link_rx) = builder.clone().connect("localclient", 200).await.unwrap();
        link_tx
            .subscribe([topics::VLS_RETURN, topics::CONTROL_RETURN, topics::ERROR])
            .await
            .unwrap();

        let router_tx = builder.router_tx();
        let status_sender_ = status_sender.clone();
        tokio::spawn(async move {
            let config = config.clone().into();
            let router_tx = router_tx.clone();
            let console: Arc<ConsoleLink> = Arc::new(ConsoleLink::new(config, router_tx));
            loop {
                let metrics = consolelink::request_metrics(console.clone(), client_id.clone());
                if let Some(c) = metrics_to_status(metrics, get_connected()) {
                    set_connected(c);
                    log::info!("connection status changed to: {}", c);
                    let _ = status_sender_.send(c).await;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        });

        let sub_task = tokio::spawn(async move {
            while let Ok(message) = link_rx.recv().await {
                for payload in message.payload {
                    if message.topic == topics::ERROR {
                        let _ = error_sender.send(payload.to_vec());
                    }
                    if let Err(e) = msg_tx.send(payload.to_vec()).await {
                        log::warn!("pub err {:?}", e);
                    }
                }
            }
        });

        let relay_task = tokio::spawn(async move {
            while let Some(msg) = receiver.recv().await {
                link_tx
                    .publish(&msg.topic, false, msg.message)
                    .await
                    .expect("could not mqtt pub");
                match timeout(Duration::from_millis(REPLY_TIMEOUT_MS), msg_rx.recv()).await {
                    Ok(reply) => {
                        if let Err(_) = msg.reply_tx.send(ChannelReply {
                            reply: reply.unwrap(),
                        }) {
                            log::warn!("could not send on reply_tx");
                        }
                    }
                    Err(e) => {
                        log::warn!("reply_tx timed out {:?}", e);
                        set_connected(false);
                        let _ = status_sender.send(false).await;
                    }
                }
            }
        });

        servers.await;
        sub_task.await.unwrap();
        relay_task.await.unwrap();
    });

    // give one second for router to spawn listeners
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
}

fn metrics_to_status(metrics: ConnectionMetrics, client_connected: bool) -> Option<bool> {
    match metrics.tracker() {
        Some(t) => {
            // wait for subscription to be sure
            if t.concrete_subscriptions_count() > 0 {
                if !client_connected {
                    Some(true) // changed to true
                } else {
                    None
                }
            } else {
                None
            }
        }
        None => {
            if client_connected {
                Some(false)
            } else {
                None
            }
        }
    }
}

fn config(settings: Settings) -> Config {
    use librumqttd::rumqttlog::Config as RouterConfig;
    use librumqttd::{ConnectionSettings, SphinxLoginCredentials, ConsoleSettings, ServerSettings};
    use std::collections::HashMap;
    use std::net::{Ipv4Addr, SocketAddrV4};
    use std::path::PathBuf;
    let id = 0;
    let router = RouterConfig {
        id,
        dir: PathBuf::from("/tmp/rumqttd"),
        max_segment_size: 10240,
        max_segment_count: 10,
        max_connections: 10001,
    };
    let mut servers = HashMap::new();
    servers.insert(
        id.to_string(),
        ServerSettings {
            cert: None,
            listen: SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), settings.mqtt_port).into(),
            next_connection_delay_ms: 1,
            connections: ConnectionSettings {
                connection_timeout_ms: 5000,
                max_client_id_len: 256,
                throttle_delay_ms: 0,
                max_payload_size: 5120,
                max_inflight_count: 200,
                max_inflight_size: 1024,
                login_credentials: None,
                sphinx_auth: Some(SphinxLoginCredentials {
                    within: None,
                }),
            },
        },
    );
    Config {
        id,
        servers,
        router,
        console: ConsoleSettings {
            listen: SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 3030).into(),
        },
        cluster: None,
        replicator: None,
    }
}
