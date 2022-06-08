use sphinx_key_parser as parser;

use rumqttc::{self, AsyncClient, Event, MqttOptions, Packet, QoS};
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
    // color_backtrace::install();

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

    loop {
        let event = eventloop.poll().await;
        // println!("{:?}", event.unwrap());
        if let Some(bs) = incoming_bytes(event.expect("failed to unwrap event")) {
            let (ping, sequence, dbid): (msgs::Ping, u16, u64) =
                parser::request_from_bytes(bs).expect("read ping header");
            println!("sequence {}", sequence);
            println!("dbid {}", dbid);
            println!("INCOMING: {:?}", ping);
            let pong = msgs::Pong {
                id: ping.id,
                message: ping.message,
            };
            let bytes = parser::raw_response_from_msg(pong, sequence)?;
            client
                .publish(PUB_TOPIC, QoS::AtMostOnce, false, bytes)
                .await
                .expect("could not mqtt publish");
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
