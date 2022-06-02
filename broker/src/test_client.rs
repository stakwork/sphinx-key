use tokio::{task, time};

use rumqttc::{self, AsyncClient, MqttOptions, QoS, Event, Packet};
use std::error::Error;
use std::time::Duration;

#[tokio::main(worker_threads = 1)]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    // color_backtrace::install();

    let mut mqttoptions = MqttOptions::new("test-1", "localhost", 1883);
    mqttoptions.set_credentials("sphinx-key", "sphinx-key-pass");
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    task::spawn(async move {
        requests(client).await;
        time::sleep(Duration::from_secs(3)).await;
    });

    loop {
        let event = eventloop.poll().await;
        // println!("{:?}", event.unwrap());
        if let Event::Incoming(packet) = event.unwrap() {
            if let Packet::Publish(p) = packet {
                println!("incoming {:?}", p.payload);
            }
        } 
    }
}

async fn requests(client: AsyncClient) {

    client
        .subscribe("sphinx", QoS::AtMostOnce)
        .await
        .unwrap();

    for _ in 1..=10 {
        client
            .publish("trigger", QoS::AtMostOnce, false, vec![1; 1])
            .await
            .unwrap();

        time::sleep(Duration::from_secs(1)).await;
    }

    time::sleep(Duration::from_secs(120)).await;
}