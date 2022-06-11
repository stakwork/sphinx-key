use crate::{ChannelReply, ChannelRequest};
use librumqttd::{
    async_locallink,
    consolelink::{self, ConsoleLink},
    rumqttlog::router::ConnectionMetrics,
    Config,
};

use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;

const SUB_TOPIC: &str = "sphinx-return";
const PUB_TOPIC: &str = "sphinx";
const USERNAME: &str = "sphinx-key";
const PASSWORD: &str = "sphinx-key-pass";

pub fn start_broker(
    mut receiver: mpsc::Receiver<ChannelRequest>,
    status_sender: mpsc::Sender<bool>,
    expected_client_id: &str,
) -> tokio::runtime::Runtime {
    let config = config();
    let client_id = expected_client_id.to_string();

    let (mut router, servers, builder) = async_locallink::construct(config.clone());

    thread::spawn(move || {
        router.start().expect("could not start router");
    });

    let mut client_connected = false;

    let mut rt_builder = tokio::runtime::Builder::new_multi_thread();
    rt_builder.enable_all();
    let rt = rt_builder.build().unwrap();
    rt.block_on(async {
        tokio::spawn(async move {
            let (msg_tx, mut msg_rx): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) =
                mpsc::channel(1000);
            let (mut link_tx, mut link_rx) =
                builder.clone().connect("localclient", 200).await.unwrap();
            link_tx.subscribe([SUB_TOPIC]).await.unwrap();

            let router_tx = builder.router_tx();
            tokio::spawn(async move {
                let config = config.clone().into();
                let router_tx = router_tx.clone();
                let console: Arc<ConsoleLink> = Arc::new(ConsoleLink::new(config, router_tx));
                loop {
                    let metrics = consolelink::request_metrics(console.clone(), client_id.clone());
                    if let Some(c) = metrics_to_status(metrics, client_connected) {
                        client_connected = c;
                        log::info!("connection status changed to: {}", c);
                        status_sender
                            .send(c)
                            .await
                            .expect("couldnt send connection status");
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            });

            let sub_task = tokio::spawn(async move {
                while let Ok(message) = link_rx.recv().await {
                    for payload in message.payload {
                        if let Err(e) = msg_tx.send(payload.to_vec()).await {
                            log::warn!("pub err {:?}", e);
                        }
                    }
                }
            });

            let relay_task = tokio::spawn(async move {
                while let Some(msg) = receiver.recv().await {
                    link_tx
                        .publish(PUB_TOPIC, false, msg.message)
                        .await
                        .expect("could not mqtt pub");
                    if let Ok(reply) = timeout(Duration::from_millis(1000), msg_rx.recv()).await {
                        if let Err(_) = msg.reply_tx.send(ChannelReply {
                            reply: reply.unwrap(),
                        }) {
                            log::warn!("could not send on reply_tx");
                        }
                    }
                }
            });

            servers.await;
            sub_task.await.unwrap();
            relay_task.await.unwrap();
        });
    });

    // give one second for router to spawn listeners
    std::thread::sleep(std::time::Duration::from_secs(1));

    rt
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

fn config() -> Config {
    use librumqttd::rumqttlog::Config as RouterConfig;
    use librumqttd::{
        ConnectionLoginCredentials, ConnectionSettings, ConsoleSettings, ServerSettings,
    };
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
        "0".to_string(),
        ServerSettings {
            cert: None,
            listen: SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1883).into(),
            next_connection_delay_ms: 1,
            connections: ConnectionSettings {
                connection_timeout_ms: 5000,
                max_client_id_len: 256,
                throttle_delay_ms: 0,
                max_payload_size: 5120,
                max_inflight_count: 200,
                max_inflight_size: 1024,
                login_credentials: Some(vec![ConnectionLoginCredentials {
                    username: USERNAME.to_string(),
                    password: PASSWORD.to_string(),
                }]),
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
