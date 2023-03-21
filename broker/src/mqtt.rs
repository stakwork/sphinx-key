use crate::util::Settings;
use crate::Connections;
use crate::{ChannelReply, ChannelRequest};
use rocket::tokio::{sync::broadcast, sync::mpsc};
use rumqttd::{local::LinkTx, Alert, AlertEvent, AuthMsg, Broker, Config, Notification};
use sphinx_signer::sphinx_glyph::sphinx_auther::token::Token;
use sphinx_signer::sphinx_glyph::topics;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// must get a reply within this time, or disconnects
// const REPLY_TIMEOUT_MS: u64 = 10000;

pub fn start_broker(
    settings: Settings,
    mut receiver: mpsc::Receiver<ChannelRequest>,
    status_sender: std::sync::mpsc::Sender<(String, bool)>,
    error_sender: broadcast::Sender<Vec<u8>>,
    auth_sender: std::sync::mpsc::Sender<AuthMsg>,
    connections: Arc<Mutex<Connections>>,
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
    let (internal_status_tx, internal_status_rx) = std::sync::mpsc::channel();
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
                        let _ = internal_status_tx.send((cid, status));
                    }
                }
            }
            _ => (),
        }
    });

    // track connections
    let status_sender_ = status_sender.clone();
    let link_tx_ = link_tx.clone();
    let _conns_task = std::thread::spawn(move || {
        while let Ok((cid, is)) = internal_status_rx.recv() {
            if is {
                subs(&cid, link_tx_.clone());
            } else {
                unsubs(&cid, link_tx_.clone());
            }
            let _ = status_sender_.send((cid, is));
        }
    });

    // String is the client id
    let (msg_tx, msg_rx) = std::sync::mpsc::channel::<(String, Vec<u8>)>();

    // receive from CLN, Frontend, or Controller
    let conns_ = connections.clone();
    let _relay_task = std::thread::spawn(move || {
        while let Some(msg) = receiver.blocking_recv() {
            if let Some(cid) = msg.cid {
                // for a specific client
                let pub_topic = format!("{}/{}", cid, msg.topic);
                if let Err(e) = link_tx.publish(pub_topic, msg.message.clone()) {
                    log::error!("failed to pub to link_tx! {} {:?}", cid, e);
                }
                let rep = msg_rx.recv();
                if let Ok((cid, reply)) = rep {
                    if let Err(_) = msg.reply_tx.send(ChannelReply { reply }) {
                        log::warn!("could not send on reply_tx {}", cid);
                    }
                }
            } else {
                // send to each client in turn
                'retry_loop: loop {
                    // get the current list of connected clients
                    let cs = conns_.lock().unwrap();
                    let client_list = cs.clients.clone();
                    drop(cs);
                    // wait a second if there are no clients
                    if client_list.len() == 0 {
                        std::thread::sleep(Duration::from_secs(1));
                    }
                    for client in client_list.iter() {
                        let pub_topic = format!("{}/{}", client, msg.topic);
                        if let Err(e) = link_tx.publish(pub_topic, msg.message.clone()) {
                            log::error!("failed to pub to link_tx! {:?}", e);
                        }
                        // and receive from the correct client (or timeout to next)
                        let dur = Duration::from_secs(9);
                        let rep = msg_rx.recv_timeout(dur);
                        if let Ok((cid, reply)) = rep {
                            if &cid == client {
                                if let Err(_) = msg.reply_tx.send(ChannelReply { reply }) {
                                    log::warn!("could not send on reply_tx");
                                }
                                break 'retry_loop;
                            } else {
                                log::warn!("Mismatched client id!");
                                // wait a second before trying again
                                std::thread::sleep(Duration::from_secs(1));
                            }
                        }
                    }
                }
            }
        }
    });

    // receive replies back from glyph
    let _sub_task = std::thread::spawn(move || {
        while let Ok(message) = link_rx.recv() {
            if let Some(n) = message {
                match n {
                    Notification::Forward(f) => {
                        let topic_res = std::str::from_utf8(&f.publish.topic);
                        if let Err(_) = topic_res {
                            continue;
                        }
                        let topic = topic_res.unwrap();
                        if topic.ends_with(topics::ERROR) {
                            let _ = error_sender.send(f.publish.payload.to_vec());
                        } else {
                            let ts: Vec<&str> = topic.split("/").collect();
                            if ts.len() != 2 {
                                continue;
                            }
                            let cid = ts[0].to_string();
                            if let Err(e) = msg_tx.send((cid, f.publish.payload.to_vec())) {
                                log::error!("failed to pub to msg_tx! {:?}", e);
                            }
                        }
                    }
                    _ => (),
                };
            }
        }
    });

    // _relay_task.await.unwrap();
    // _sub_task.await.unwrap();
    // _alerts_handle.await?;

    std::thread::sleep(Duration::from_secs(1));

    Ok(())
}

fn subs(cid: &str, mut ltx: LinkTx) {
    ltx.subscribe(format!("{}/{}", cid, topics::VLS_RETURN))
        .unwrap();
    ltx.subscribe(format!("{}/{}", cid, topics::CONTROL_RETURN))
        .unwrap();
    ltx.subscribe(format!("{}/{}", cid, topics::ERROR)).unwrap();
}

fn unsubs(cid: &str, mut ltx: LinkTx) {
    // ltx.unsubscribe(format!("{}/{}", cid, topics::VLS_RETURN))
    //     .unwrap();
    // ltx.unsubscribe(format!("{}/{}", cid, topics::CONTROL_RETURN))
    //     .unwrap();
    // ltx.unsubscribe(format!("{}/{}", cid, topics::ERROR))
    //     .unwrap();
}

pub fn check_auth(username: &str, password: &str, conns: &mut crate::Connections) -> bool {
    match Token::from_base64(password) {
        Ok(t) => match t.recover() {
            Ok(pubkey) => {
                // pubkey must match signature
                if &pubkey.to_string() == username {
                    if let Some(pk) = &conns.pubkey {
                        // if there is an existing pubkey then new client must match
                        pk == username
                    } else {
                        // set the Connections pubkey
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
