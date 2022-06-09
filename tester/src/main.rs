use sphinx_key_parser as parser;

use clap::{App, AppSettings, Arg};
use rumqttc::{self, AsyncClient, Event, MqttOptions, Packet, QoS};
use sphinx_key_signer::{self, InitResponse, PubKey};
use std::error::Error;
use std::time::Duration;
use vls_protocol::msgs;

const SUB_TOPIC: &str = "sphinx";
const PUB_TOPIC: &str = "sphinx-return";
const USERNAME: &str = "sphinx-key";
const PASSWORD: &str = "sphinx-key-pass";

#[tokio::main(worker_threads = 1)]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let app = App::new("tester")
        .setting(AppSettings::NoAutoVersion)
        .about("CLN:mqtt-tester - MQTT client signer")
        .arg(Arg::from("--test run a test against the embedded device"));

    let mut mqttoptions = MqttOptions::new("test-1", "localhost", 1883);
    mqttoptions.set_credentials(USERNAME, PASSWORD);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    client
        .subscribe(SUB_TOPIC, QoS::AtMostOnce)
        .await
        .expect("could not mqtt subscribe");

    client
        .publish(
            PUB_TOPIC,
            QoS::AtMostOnce,
            false,
            "READY".as_bytes().to_vec(),
        )
        .await
        .expect("could not pub");

    let matches = app.get_matches();
    if matches.is_present("test") {
        loop {
            let event = eventloop.poll().await.expect("failed to unwrap event");
            // println!("{:?}", event);
            if let Some(ping_bytes) = incoming_bytes(event) {
                let (ping, sequence, dbid): (msgs::Ping, u16, u64) =
                    parser::request_from_bytes(ping_bytes).expect("read ping header");
                println!("sequence {}", sequence);
                println!("dbid {}", dbid);
                println!("INCOMING: {:?}", ping);
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
    } else {
        // once the init loop is done, the root_handler is returned
        let root_handler = loop {
            let init_event = eventloop.poll().await.expect("failed to unwrap event");
            // this may be another kind of message like MQTT ConnAck
            // loop around again and wait for the init
            if let Some(init_msg_bytes) = incoming_bytes(init_event) {
                let InitResponse {
                    root_handler,
                    init_reply,
                } = sphinx_key_signer::init(init_msg_bytes).expect("failed to init signer");
                client
                    .publish(PUB_TOPIC, QoS::AtMostOnce, false, init_reply)
                    .await
                    .expect("could not publish init response");
                // return the root_handler and finish the init loop
                break root_handler;
            }
        };
        // the actual loop
        loop {
            let event = eventloop.poll().await.expect("failed to unwrap event");
            let dummy_peer = PubKey([0; 33]);
            if let Some(msg_bytes) = incoming_bytes(event) {
                match sphinx_key_signer::handle(&root_handler, msg_bytes, dummy_peer.clone()) {
                    Ok(b) => client
                        .publish(PUB_TOPIC, QoS::AtMostOnce, false, b)
                        .await
                        .expect("could not publish init response"),
                    Err(e) => panic!("HANDLE FAILED {:?}", e),
                };
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
