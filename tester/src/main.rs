use sphinx_key_parser as parser;
use sphinx_key_signer::lightning_signer::bitcoin::Network;

use clap::{App, AppSettings, Arg};
use rumqttc::{self, AsyncClient, Event, MqttOptions, Packet, QoS};
use sphinx_key_signer::{self, InitResponse};
use sphinx_key_signer::vls_protocol::model::PubKey;
use std::error::Error;
use std::time::Duration;
use vls_protocol::msgs;
use std::env;
use std::str::FromStr;

const SUB_TOPIC: &str = "sphinx";
const PUB_TOPIC: &str = "sphinx-return";
const USERNAME: &str = "sphinx-key";
const PASSWORD: &str = "sphinx-key-pass";

#[tokio::main(worker_threads = 1)]
async fn main() -> Result<(), Box<dyn Error>> {
    setup_logging("sphinx-key-tester  ", "info");

    let app = App::new("tester")
        .setting(AppSettings::NoAutoVersion)
        .about("CLN:mqtt-tester - MQTT client signer")
        .arg(Arg::from("--test run a test against the embedded device"))
        .arg(Arg::from("--log log each VLS message"));
    let matches = app.get_matches();
    let is_test = matches.is_present("test");
    let is_log = matches.is_present("log");
    if is_log {
        log::info!("==> log each incoming message!");
    }
    // main loop - alternate between "reconnection" and "handler"
    loop {
        let mut try_i = 0;
        let (client, mut eventloop) = loop {
            let mut mqttoptions = MqttOptions::new("test-1", "localhost", 1883);
            mqttoptions.set_credentials(USERNAME, PASSWORD);
            mqttoptions.set_keep_alive(Duration::from_secs(5));
            let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
            match eventloop.poll().await {
                Ok(event) => {
                    if let Some(_) = incoming_conn_ack(event) {
                        println!("==========> MQTT connected!");
                        break (client, eventloop);
                    }
                }
                Err(_) => {
                    try_i = try_i + 1;
                    println!("reconnect.... {}", try_i);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        };

        client
            .subscribe(SUB_TOPIC, QoS::AtMostOnce)
            .await
            .expect("could not mqtt subscribe");

        if is_test {
            // test handler loop
            loop {
                match eventloop.poll().await {
                    Ok(event) => {
                        // println!("{:?}", event);
                        if let Some(ping_bytes) = incoming_bytes(event) {
                            let (ping, sequence, dbid): (msgs::Ping, u16, u64) =
                                parser::request_from_bytes(ping_bytes).expect("read ping header");
                            if is_log {
                                println!("sequence {}", sequence);
                                println!("dbid {}", dbid);
                                println!("INCOMING: {:?}", ping);
                            }
                            let pong = msgs::Pong {
                                id: ping.id,
                                message: ping.message,
                            };
                            let bytes = parser::raw_response_from_msg(pong, sequence)
                                .expect("couldnt parse raw response");
                            client
                                .publish(PUB_TOPIC, QoS::AtMostOnce, false, bytes)
                                .await
                                .expect("could not mqtt publish");
                        }
                    }
                    Err(e) => {
                        log::warn!("diconnected {:?}", e);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        break; // break out of this loop to reconnect
                    }
                }
            }
        } else {
            // once the init loop is done, the root_handler is returned
            let root_handler = loop {
                if let Ok(init_event) = eventloop.poll().await {
                    // this may be another kind of message like MQTT ConnAck
                    // loop around again and wait for the init
                    if let Some(init_msg_bytes) = incoming_bytes(init_event) {
                        let InitResponse {
                            root_handler,
                            init_reply,
                        } = sphinx_key_signer::init(init_msg_bytes, Network::Regtest).expect("failed to init signer");
                        client
                            .publish(PUB_TOPIC, QoS::AtMostOnce, false, init_reply)
                            .await
                            .expect("could not publish init response");
                        // return the root_handler and finish the init loop
                        break Some(root_handler);
                    }
                } else {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    log::warn!("failed to initialize! Lost connection");
                    break None;
                }
            };
            // the actual handler loop
            loop {
                if let Some(rh) = &root_handler {
                    match eventloop.poll().await {
                        Ok(event) => {
                            let dummy_peer = PubKey([0; 33]);
                            if let Some(msg_bytes) = incoming_bytes(event) {
                                match sphinx_key_signer::handle(rh, msg_bytes, dummy_peer.clone(), is_log) {
                                    Ok(b) => client
                                        .publish(PUB_TOPIC, QoS::AtMostOnce, false, b)
                                        .await
                                        .expect("could not publish init response"),
                                    Err(e) => panic!("HANDLE FAILED {:?}", e),
                                };
                            }
                        }
                        Err(e) => {
                            log::warn!("diconnected {:?}", e);
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            break; // break out of this loop to reconnect
                        }
                    }
                } else {
                    break;
                }
            }
        }
    }
}

fn incoming_bytes(event: Event) -> Option<Vec<u8>> {
    if let Event::Incoming(packet) = event {
        if let Packet::Publish(p) = packet {
            return Some(p.payload.to_vec());
        }
    }
    None
}

fn incoming_conn_ack(event: Event) -> Option<()> {
    if let Event::Incoming(packet) = event {
        if let Packet::ConnAck(_) = packet {
            return Some(());
        }
    }
    None
}

pub fn setup_logging(who: &str, level_arg: &str) {
    use fern::colors::{Color, ColoredLevelConfig};
    let colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .error(Color::Red)
        .warn(Color::Yellow);
    let level = env::var("RUST_LOG").unwrap_or(level_arg.to_string());
    let who = who.to_string();
    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{} {}/{} {}] {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f"),
                who,
                record.target(),
                colors.color(record.level()),
                message
            ))
        })
        .level(log::LevelFilter::from_str(&level).expect("level"))
        .level_for("h2", log::LevelFilter::Info)
        .level_for("sled", log::LevelFilter::Info)
        .level_for(
            "librumqttd::rumqttlog::router::router",
            log::LevelFilter::Warn,
        )
        .chain(std::io::stdout())
        // .chain(fern::log_file("/tmp/output.log")?)
        .apply()
        .expect("log config");
}
