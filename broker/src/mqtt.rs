use crate::util::Settings;
use crate::{ChannelReply, ChannelRequest};
use rocket::tokio::{sync::broadcast, sync::mpsc};
use rumqttd::{Alert, AlertEvent, AuthMsg, Broker, Config, Notification};
use sphinx_signer::sphinx_glyph::sphinx_auther::token::Token;
use sphinx_signer::sphinx_glyph::topics;
use std::time::Duration;

// must get a reply within this time, or disconnects
// const REPLY_TIMEOUT_MS: u64 = 10000;

pub fn start_broker(
    mut receiver: mpsc::Receiver<ChannelRequest>,
    status_sender: mpsc::Sender<(String, bool)>,
    error_sender: broadcast::Sender<Vec<u8>>,
    settings: Settings,
    auth_sender: std::sync::mpsc::Sender<AuthMsg>,
) -> anyhow::Result<()> {
    let conf = config(settings);
    // let client_id = expected_client_id.to_string();

    let mut broker = Broker::new(conf);
    let mut alerts = broker.alerts(vec![
        // "/alerts/error/+".to_string(),
        "/alerts/event/connect/+".to_string(),
        "/alerts/event/disconnect/+".to_string(),
    ])?;
    let (mut link_tx, mut link_rx) = broker.link("localclient")?;

    let auth_sender_ = auth_sender.clone();
    std::thread::spawn(move || {
        broker
            .start(Some(auth_sender_))
            .expect("could not start broker");
    });

    // connected/disconnected status alerts
    let status_sender_ = status_sender.clone();
    let _alerts_handle = std::thread::spawn(move || loop {
        let alert = alerts.poll();
        log::info!("Alert: {:?}", alert);
        match alert.1 {
            Alert::Event(cid, event) => {
                // dont alert for local connections
                let locals = vec!["console", "localclient"];
                if !locals.contains(&cid.as_str()) {
                    if let Some(status) = match event {
                        AlertEvent::Connect => Some(true),
                        AlertEvent::Disconnect => Some(false),
                        _ => None,
                    } {
                        let _ = status_sender_.blocking_send((cid, status));
                    }
                }
            }
            _ => (),
        }
    });

    let (msg_tx, mut msg_rx): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) =
        mpsc::channel(10000);
    link_tx.subscribe(topics::VLS_RETURN).unwrap();
    link_tx.subscribe(topics::CONTROL_RETURN).unwrap();
    link_tx.subscribe(topics::ERROR).unwrap();

    let _sub_task = std::thread::spawn(move || {
        while let Ok(message) = link_rx.recv() {
            if let Some(n) = message {
                match n {
                    Notification::Forward(f) => {
                        if f.publish.topic == topics::ERROR {
                            let _ = error_sender.send(f.publish.topic.to_vec());
                        } else {
                            if let Err(e) = msg_tx.blocking_send(f.publish.payload.to_vec()) {
                                log::error!("failed to pub to msg_tx! {:?}", e);
                            }
                        }
                    }
                    _ => (),
                };
            }
        }
    });

    let _relay_task = std::thread::spawn(move || {
        while let Some(msg) = receiver.blocking_recv() {
            if let Err(e) = link_tx.publish(msg.topic, msg.message) {
                log::error!("failed to pub to link_tx! {:?}", e);
            }
            let rep = msg_rx.blocking_recv();
            if let Some(reply) = rep {
                if let Err(_) = msg.reply_tx.send(ChannelReply { reply }) {
                    log::warn!("could not send on reply_tx");
                }
            }
        }
    });

    // _sub_task.await.unwrap();
    // _relay_task.await.unwrap();
    // _alerts_handle.await?;

    std::thread::sleep(Duration::from_secs(1));

    Ok(())
}

pub fn check_auth(username: &str, password: &str, conns: &mut crate::Connections) -> bool {
    match Token::from_base64(password) {
        Ok(t) => match t.recover() {
            Ok(pubkey) => {
                if &pubkey.to_string() == username {
                    if let Some(pk) = &conns.pubkey {
                        // if there is an existing then it must match it
                        pk == username
                    } else {
                        conns.set_pubkey(username);
                        true
                    }
                } else {
                    false
                }
            }
            Err(_) => false,
        },
        Err(_) => false,
    }
}

fn config(settings: Settings) -> Config {
    use rumqttd::{ConnectionSettings, ConsoleSettings, ServerSettings};
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
