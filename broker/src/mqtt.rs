use crate::util::Settings;
use crate::{ChannelReply, ChannelRequest};
use rocket::tokio::time::timeout;
use rocket::tokio::{self, sync::broadcast, sync::mpsc};
use rumqttd::{Alert, AlertEvent, Broker, Config, Notification};
use sphinx_signer::sphinx_glyph::topics;
use std::time::Duration;

// pub const INTERNAL_CONTROL: &str = "INTERNAL_CONTROL";

// must get a reply within this time, or disconnects
const REPLY_TIMEOUT_MS: u64 = 10000;

pub fn start_broker(
    mut receiver: mpsc::Receiver<ChannelRequest>,
    status_sender: mpsc::Sender<bool>,
    error_sender: broadcast::Sender<Vec<u8>>,
    expected_client_id: &str,
    settings: Settings,
) -> anyhow::Result<()> {
    let conf = config(settings);
    let client_id = expected_client_id.to_string();

    let mut broker = Broker::new(conf);
    let mut alerts = broker.alerts(vec![
        // "/alerts/error/+".to_string(),
        "/alerts/event/connect/+".to_string(),
        "/alerts/event/disconnect/+".to_string(),
    ])?;
    let (mut link_tx, mut link_rx) = broker.link("localclient")?;

    std::thread::spawn(move || {
        broker.start().expect("could not start broker");
    });

    // connected/disconnected status alerts
    let status_sender_ = status_sender.clone();
    let _alerts_handle = tokio::spawn(async move {
        loop {
            let alert = alerts.poll();
            println!("Alert: {:?}", alert);
            match alert.1 {
                Alert::Event(cid, event) => {
                    if cid == client_id {
                        if let Some(status) = match event {
                            AlertEvent::Connect => Some(true),
                            AlertEvent::Disconnect => Some(false),
                            _ => None,
                        } {
                            let _ = status_sender_.send(status).await;
                        }
                    }
                }
                _ => (),
            }
        }
    });

    // msg forwarding
    let (msg_tx, mut msg_rx): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) =
        mpsc::channel(1000);
    // link_tx.subscribe(INTERNAL_CONTROL)?;
    link_tx.subscribe(topics::VLS_RETURN)?;
    link_tx.subscribe(topics::CONTROL_RETURN)?;
    link_tx.subscribe(topics::ERROR)?;

    let _sub_task = tokio::spawn(async move {
        println!("ummm....");
        while let Ok(message) = link_rx.recv() {
            if let Some(n) = message {
                match n {
                    Notification::Forward(f) => {
                        println!("GOT A FORWARDED MSG! FORWARD! {:?}", f.publish.topic);
                        if f.publish.topic == topics::ERROR {
                            let _ = error_sender.send(f.publish.topic.to_vec());
                        } else {
                            println!("send now on msg_tx {:?}", f.publish.payload.to_vec());
                            if let Err(e) = msg_tx.send(f.publish.payload.to_vec()).await {
                                log::error!("failed to pub to msg_tx! {:?}", e);
                            }
                            println!("sent  on msg_tx");
                        }
                    }
                    _ => (),
                };
            }
        }
    });

    let _relay_task = tokio::spawn(async move {
        while let Some(msg) = receiver.recv().await {
            if let Err(e) = link_tx.publish(msg.topic, msg.message) {
                log::error!("failed to pub to link_tx! {:?}", e);
            }
            println!("PUBBED TO LINKTX....");
            // let rep = msg_rx.recv().await;
            // println!("REPPPP {:?}", rep);
            match timeout(Duration::from_millis(REPLY_TIMEOUT_MS), msg_rx.recv()).await {
                Ok(rep) => {
                    println!("GOT A REPLY {:?}", rep);
                    if let Some(reply) = rep {
                        if let Err(_) = msg.reply_tx.send(ChannelReply { reply }) {
                            log::warn!("could not send on reply_tx");
                        }
                    }
                }
                Err(e) => {
                    log::warn!("reply_tx timed out {:?}", e);
                    let _ = status_sender.send(false).await;
                }
            }
        }
    });

    std::thread::sleep(Duration::from_secs(1));

    // alerts_handle.await?;
    // sub_task.await?;
    // relay_task.await?;
    Ok(())
}

fn config(settings: Settings) -> Config {
    use rumqttd::{ConnectionSettings, ConsoleSettings, ServerSettings, SphinxLoginCredentials};
    use std::collections::HashMap;
    use std::net::{Ipv4Addr, SocketAddrV4};
    let router = rumqttd::RouterConfig {
        instant_ack: true,
        max_segment_size: 104857600,
        max_segment_count: 10,
        max_connections: 10010,
        max_read_len: 10240,
        ..Default::default()
    };
    let mut servers = HashMap::new();
    servers.insert(
        "sphinx-broker".to_string(),
        ServerSettings {
            name: "sphinx-broker".to_string(),
            listen: SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), settings.mqtt_port).into(),
            next_connection_delay_ms: 1,
            connections: ConnectionSettings {
                connection_timeout_ms: 5000,
                throttle_delay_ms: 0,
                max_payload_size: 5120,
                max_inflight_count: 200,
                max_inflight_size: 1024,
                auth: None,
                sphinx_auth: Some(SphinxLoginCredentials { within: None }),
                dynamic_filters: true,
            },
            tls: None,
        },
    );
    Config {
        id: 0,
        v4: servers,
        router,
        console: ConsoleSettings::new("0.0.0.0:3030"),
        cluster: None,
        ..Default::default()
    }
}
