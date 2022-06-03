
use sphinx_key_parser::MsgDriver;

use rumqttc::{self, AsyncClient, MqttOptions, QoS, Event, Packet};
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

    client.publish(PUB_TOPIC, QoS::AtMostOnce, false, "READY".as_bytes().to_vec()).await.expect("could not pub");

    loop {
        let event = eventloop.poll().await;
        // println!("{:?}", event.unwrap());
        if let Some(mut m) = incoming_msg(event.expect("failed to unwrap event")) {
            let (sequence, dbid) = msgs::read_serial_request_header(&mut m).expect("read ping header");
            println!("sequence {}", sequence);
            println!("dbid {}", dbid);
            let ping: msgs::Ping =
                msgs::read_message(&mut m).expect("failed to read ping message");
            println!("INCOMING: {:?}", ping);
            let mut md = MsgDriver::new_empty();
            msgs::write_serial_response_header(&mut md, sequence)
                .expect("failed to write_serial_request_header");
            let pong = msgs::Pong {
                id: ping.id,
                message: ping.message
            };
            msgs::write(&mut md, pong).expect("failed to serial write");
            client
                .publish(PUB_TOPIC, QoS::AtMostOnce, false, md.bytes())
                .await
                .expect("could not mqtt publish");
        }
    }
}

fn incoming_msg(event: Event) -> Option<MsgDriver> {
    if let Event::Incoming(packet) = event {
        if let Packet::Publish(p) = packet {
            let m = MsgDriver::new(p.payload.to_vec());
            return Some(m)
        }
    }
    None
}