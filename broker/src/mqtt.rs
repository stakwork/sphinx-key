use crate::conn::Connections;
use crate::conn::{ChannelReply, ChannelRequest};
use crate::util::Settings;
use rocket::tokio::{sync::broadcast, sync::mpsc};
use rumqttd::{local::LinkTx, AuthMsg, Broker, Config, Notification};
use sphinx_signer::sphinx_glyph::sphinx_auther::token::Token;
use sphinx_signer::sphinx_glyph::topics;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// must get a reply within this time, or disconnects
// const REPLY_TIMEOUT_MS: u64 = 10000;

pub fn start_broker(
    settings: Settings,
    mut receiver: mpsc::Receiver<ChannelRequest>,
    mut init_receiver: mpsc::Receiver<ChannelRequest>,
    status_sender: std::sync::mpsc::Sender<(String, bool)>,
    error_sender: broadcast::Sender<Vec<u8>>,
    auth_sender: std::sync::mpsc::Sender<AuthMsg>,
    connections: Arc<Mutex<Connections>>,
) -> anyhow::Result<()> {
    let conf = config(settings);
    // println!("CONF {:?}", conf);
    // let client_id = expected_client_id.to_string();

    let mut broker = Broker::new(conf);

    let (mut link_tx, mut link_rx) = broker.link("localclient")?;

    let _ = link_tx.subscribe(format!("+/{}", topics::HELLO));
    let _ = link_tx.subscribe(format!("+/{}", topics::BYE));

    let auth_sender_ = auth_sender.clone();
    std::thread::spawn(move || {
        broker
            .start(Some(auth_sender_))
            .expect("could not start broker");
    });

    // connected/disconnected status alerts
    let (internal_status_tx, internal_status_rx) = std::sync::mpsc::channel::<(bool, String)>();

    // track connections
    let link_tx_ = link_tx.clone();
    let _conns_task = std::thread::spawn(move || {
        while let Ok((is, cid)) = internal_status_rx.recv() {
            if is {
                subs(&cid, link_tx_.clone());
            } else {
                unsubs(&cid, link_tx_.clone());
            }
            let _ = status_sender.send((cid, is));
        }
    });

    // (client_id, topic_end, payload). topic_end is always topics::LSS_RES
    let (init_tx, init_rx) = std::sync::mpsc::channel::<(String, String, Vec<u8>)>();

    let mut link_tx_ = link_tx.clone();
    let conns_ = connections.clone();
    // receive replies from LSS initialization
    let _init_task = std::thread::spawn(move || {
        while let Some(msg) = init_receiver.blocking_recv() {
            // Retry three times
            pub_and_wait(msg, &conns_, &init_rx, &mut link_tx_, Some(3));
        }
    });

    // (client_id, topic_end, payload)
    let (msg_tx, msg_rx) = std::sync::mpsc::channel::<(String, String, Vec<u8>)>();

    // receive from CLN, Frontend, Controller, or LSS
    let _relay_task = std::thread::spawn(move || {
        while let Some(msg) = receiver.blocking_recv() {
            log::debug!("Received message here: {:?}", msg);
            let retries = if msg.topic == topics::CONTROL {
                // Don't retry
                Some(0)
            } else {
                // Retry indefinitely
                None
            };
            pub_and_wait(msg, &connections, &msg_rx, &mut link_tx, retries);
        }
    });

    // receive replies back from glyph
    let _sub_task = std::thread::spawn(move || {
        while let Ok(message) = link_rx.recv() {
            if let None = message {
                continue;
            }
            match message.unwrap() {
                Notification::Forward(f) => {
                    let topic_res = std::str::from_utf8(&f.publish.topic);
                    if let Err(_) = topic_res {
                        continue;
                    }

                    let topic = topic_res.unwrap();
                    if topic.ends_with(topics::ERROR) {
                        let _ = error_sender.send(f.publish.payload.to_vec());
                        continue;
                    }

                    let ts: Vec<&str> = topic.split("/").collect();
                    if ts.len() != 2 {
                        continue;
                    }
                    let cid = ts[0].to_string();
                    let topic_end = ts[1].to_string();

                    if topic.ends_with(topics::HELLO) {
                        let _ = internal_status_tx.send((true, cid));
                    } else if topic.ends_with(topics::BYE) {
                        let _ = internal_status_tx.send((false, cid));
                    } else {
                        // VLS, CONTROL, LSS
                        let pld = f.publish.payload.to_vec();
                        if topic_end == topics::INIT_1_RES || topic_end == topics::INIT_2_RES {
                            if let Err(e) = init_tx.send((cid, topic_end, pld)) {
                                log::error!("failed to pub to init_tx! {:?}", e);
                            }
                        } else {
                            if let Err(e) = msg_tx.send((cid, topic_end, pld)) {
                                log::error!("failed to pub to msg_tx! {:?}", e);
                            }
                        }
                    }
                }
                _ => continue,
            }
        }
    });

    // _init_task.await.unwrap();
    // _relay_task.await.unwrap();
    // _sub_task.await.unwrap();
    // _alerts_handle.await?;

    std::thread::sleep(Duration::from_secs(1));

    Ok(())
}

// waits forever until the reply is returned
fn pub_and_wait(
    msg: ChannelRequest,
    conns_: &Arc<Mutex<Connections>>,
    msg_rx: &std::sync::mpsc::Receiver<(String, String, Vec<u8>)>,
    link_tx: &mut LinkTx,
    retries: Option<u8>,
) {
    let mut counter = 0u8;
    loop {
        log::debug!("looping in pub_and_wait");
        let reply = if let Some(cid) = msg.cid.clone() {
            // for a specific client
            log::debug!("publishing to a specific client");
            pub_timeout(&cid, &msg.topic, &msg.message, &msg_rx, link_tx)
        } else {
            log::debug!("publishing to all clients");
            let cs = conns_.lock().unwrap();
            let current = cs.current.clone();
            let client_list = cs.clients.clone();
            log::debug!("got the list lock!");
            drop(cs);
            // send to each client in turn
            if client_list.len() == 0 || current.is_none() {
                // wait a second if there are no clients
                std::thread::sleep(Duration::from_secs(1));
                None
            } else {
                let current_cid = current.clone().unwrap();
                // Try the current connection
                let mut rep = pub_timeout(&current_cid, &msg.topic, &msg.message, &msg_rx, link_tx);

                // We restart the loop in case a new signer connects while pinging for connections
                // as this could mean that the LSS state advanced broker side only, and signer side
                // it's one step behind. Here and also in the for loop.
                // We do it as soon as we know.
                let cs = conns_.lock().unwrap();
                let new_current = cs.current.clone();
                drop(cs);
                if new_current != current {
                    log::warn!("Client list changed, starting over!");
                    counter = 0u8;
                    continue;
                }

                // If that failed, try looking for some other signer
                if rep.is_none() {
                    for cid in client_list.into_keys().filter(|k| k != &current_cid) {
                        rep = pub_timeout(&cid, &msg.topic, &msg.message, &msg_rx, link_tx);
                        let mut cs = conns_.lock().unwrap();
                        log::debug!("got the list lock!");
                        let new_current = cs.current.clone();
                        if new_current != current {
                            log::info!("Client list changed, starting over!");
                            drop(cs);
                            counter = 0u8;
                            rep = None;
                            break;
                        }
                        if rep.is_some() {
                            cs.set_current(cid.to_string());
                            drop(cs);
                            break;
                        }
                        drop(cs);
                    }
                }
                rep
            }
        };
        if let Some(reply) = reply {
            log::debug!("MQTT got this response: {:?}", reply);
            if let Err(_) = msg.reply_tx.send(reply) {
                log::warn!("could not send on reply_tx");
            }
            break;
        } else {
            log::debug!("couldn't reach any clients...");
        }
        if let Some(max) = retries {
            log::debug!("counter: {}, retries: {}", counter, max);
            if counter == max {
                if let Err(_) = msg.reply_tx.send(ChannelReply::empty()) {
                    log::warn!("could not send on reply_tx");
                }
                break;
            }
        } else {
            log::debug!("retrying indefinitely");
        }
        counter = counter.wrapping_add(1u8);
    }
}

// publish to signer and wait for response
fn pub_timeout(
    client_id: &str,
    topic: &str,
    payload: &[u8],
    msg_rx: &std::sync::mpsc::Receiver<(String, String, Vec<u8>)>,
    link_tx: &mut LinkTx,
) -> Option<ChannelReply> {
    let pub_topic = format!("{}/{}", client_id, topic);
    log::info!("SENDING TO {} on topic {}", client_id, topic);
    if let Err(e) = link_tx.publish(pub_topic, payload.to_vec()) {
        log::error!("failed to pub to link_tx! {:?}", e);
    }
    // and receive from the correct client (or timeout to next)
    let dur = Duration::from_secs(4);
    if let Ok((cid, topic_end, reply)) = msg_rx.recv_timeout(dur) {
        if &cid == client_id {
            return Some(ChannelReply::new(topic_end, reply));
        } else {
            log::warn!("Mismatched client id!");
            // wait a second before trying again
            std::thread::sleep(Duration::from_secs(1));
        }
    }
    None
}

fn subs(cid: &str, mut ltx: LinkTx) {
    for t in topics::BROKER_SUBS {
        ltx.subscribe(format!("{}/{}", cid, t)).unwrap();
    }
}

fn unsubs(_cid: &str, mut _ltx: LinkTx) {
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
    let conns = ConnectionSettings {
        connection_timeout_ms: 5000,
        throttle_delay_ms: 0,
        max_payload_size: 262144,
        max_inflight_count: 256,
        max_inflight_size: 1024,
        auth: None,
        dynamic_filters: true,
    };
    let mut v4_servers = HashMap::new();
    v4_servers.insert(
        "v4".to_string(),
        ServerSettings {
            name: "v4".to_string(),
            listen: SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), settings.mqtt_port).into(),
            next_connection_delay_ms: 1,
            connections: conns.clone(),
            tls: None,
        },
    );
    let mut ws_servers = None;
    if let Some(wsp) = settings.websocket_port {
        let mut ws = HashMap::new();
        ws.insert(
            "ws".to_string(),
            ServerSettings {
                name: "ws".to_string(),
                listen: SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), wsp).into(),
                next_connection_delay_ms: 1,
                connections: conns,
                tls: None,
            },
        );
        ws_servers = Some(ws);
    }
    Config {
        id: 0,
        v4: v4_servers,
        ws: ws_servers,
        router,
        console: ConsoleSettings::new("0.0.0.0:3030"),
        cluster: None,
        ..Default::default()
    }
}
