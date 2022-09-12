use parser::topics;
use sphinx_key_parser as parser;
use sphinx_key_signer::lightning_signer::bitcoin::Network;

use clap::{App, AppSettings, Arg};
use dotenv::dotenv;
use rumqttc::{self, AsyncClient, Event, MqttOptions, Packet, QoS};
use sphinx_key_signer::control::Controller;
use sphinx_key_signer::vls_protocol::{model::PubKey, msgs};
use sphinx_key_signer::{self, InitResponse};
use std::convert::TryInto;
use std::env;
use std::error::Error;
use std::str::FromStr;
use std::time::Duration;

const USERNAME: &str = "sphinx-key";
const PASSWORD: &str = "sphinx-key-pass";

#[tokio::main(worker_threads = 1)]
async fn main() -> Result<(), Box<dyn Error>> {
    setup_logging("sphinx-key-tester  ", "info");

    dotenv().ok();

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
                Err(e) => {
                    try_i = try_i + 1;
                    println!("reconnect.... {} {:?}", try_i, e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        };

        client
            .subscribe(topics::VLS, QoS::AtMostOnce)
            .await
            .expect("could not mqtt subscribe");
        client
            .subscribe(topics::CONTROL, QoS::AtMostOnce)
            .await
            .expect("could not mqtt subscribe");

        let network = Network::Regtest;
        let seed_string: String = env::var("SEED").expect("no seed");
        let seed = hex::decode(seed_string).expect("couldnt decode seed");
        // make the controller to validate Control messages
        let mut ctrlr = controller_from_seed(&network, &seed);

        if is_test {
            // test handler loop
            loop {
                match eventloop.poll().await {
                    Ok(event) => {
                        // println!("{:?}", event);
                        if let Some((topic, msg_bytes)) = incoming_bytes(event) {
                            match topic.as_str() {
                                topics::VLS => {
                                    let (ping, sequence, dbid): (msgs::Ping, u16, u64) =
                                        parser::request_from_bytes(msg_bytes)
                                            .expect("read ping header");
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
                                        .publish(topics::VLS_RETURN, QoS::AtMostOnce, false, bytes)
                                        .await
                                        .expect("could not mqtt publish");
                                }
                                topics::CONTROL => {
                                    match ctrlr.handle(&msg_bytes) {
                                        Ok((response, _new_policy)) => {
                                            client
                                                .publish(
                                                    topics::CONTROL_RETURN,
                                                    QoS::AtMostOnce,
                                                    false,
                                                    response,
                                                )
                                                .await
                                                .expect("could not mqtt publish");
                                        }
                                        Err(e) => log::warn!("error parsing ctrl msg {:?}", e),
                                    };
                                }
                                _ => log::info!("invalid topic"),
                            }
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
            let seed32: [u8; 32] = seed.try_into().expect("wrong seed");
            let init_msg =
                sphinx_key_signer::make_init_msg(network, seed32).expect("failed to make init msg");
            let InitResponse {
                root_handler,
                init_reply: _,
            } = sphinx_key_signer::init(init_msg, network, &Default::default())
                .expect("failed to init signer");
            // the actual handler loop
            loop {
                match eventloop.poll().await {
                    Ok(event) => {
                        let dummy_peer = PubKey([0; 33]);
                        if let Some((topic, msg_bytes)) = incoming_bytes(event) {
                            match topic.as_str() {
                                topics::VLS => {
                                    match sphinx_key_signer::handle(
                                        &root_handler,
                                        msg_bytes,
                                        dummy_peer.clone(),
                                        is_log,
                                    ) {
                                        Ok(b) => client
                                            .publish(topics::VLS_RETURN, QoS::AtMostOnce, false, b)
                                            .await
                                            .expect("could not publish init response"),
                                        Err(e) => panic!("HANDLE FAILED {:?}", e),
                                    };
                                }
                                topics::CONTROL => {
                                    match ctrlr.handle(&msg_bytes) {
                                        Ok((response, _new_policy)) => {
                                            client
                                                .publish(
                                                    topics::CONTROL_RETURN,
                                                    QoS::AtMostOnce,
                                                    false,
                                                    response,
                                                )
                                                .await
                                                .expect("could not mqtt publish");
                                        }
                                        Err(e) => log::warn!("error parsing ctrl msg {:?}", e),
                                    };
                                }
                                _ => log::info!("invalid topic"),
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("diconnected {:?}", e);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        break; // break out of this loop to reconnect
                    }
                }
            }
        }
    }
}

fn incoming_bytes(event: Event) -> Option<(String, Vec<u8>)> {
    if let Event::Incoming(packet) = event {
        if let Packet::Publish(p) = packet {
            return Some((p.topic, p.payload.to_vec()));
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

pub fn controller_from_seed(network: &Network, seed: &[u8]) -> Controller {
    let (pk, sk) = sphinx_key_signer::derive_node_keys(network, seed);
    Controller::new(sk, pk, 0)
}
