use sphinx_key_parser::MsgDriver;
use librumqttd::{async_locallink::construct_broker, Config};
use std::thread;
use vls_protocol::msgs;
use vls_protocol::serde_bolt::WireString;
use tokio::sync::mpsc;

const SUB_TOPIC: &str = "sphinx-return";
const TRIGGER_TOPIC: &str = "trigger";
const PUB_TOPIC: &str = "sphinx";

fn main() {
    pretty_env_logger::init();
    let config: Config = confy::load_path("config/rumqttd.conf").unwrap();

    let (mut router, console, servers, builder) = construct_broker(config);

    thread::spawn(move || {
        router.start().unwrap();
    });

    let mut rt = tokio::runtime::Builder::new_multi_thread();
    rt.enable_all();
    rt.build().unwrap().block_on(async {
        let (msg_tx, mut msg_rx): (mpsc::UnboundedSender<Vec<u8>>, mpsc::UnboundedReceiver<Vec<u8>>) = mpsc::unbounded_channel();
        let (mut tx, mut rx) = builder.connect("localclient", 200).await.unwrap();
        tx.subscribe([TRIGGER_TOPIC]).await.unwrap();

        let console_task = tokio::spawn(console);

        let pub_task = tokio::spawn(async move {
            while let Some(_) = msg_rx.recv().await {
                let sequence = 0;
                let mut md = MsgDriver::new_empty(); 
                msgs::write_serial_request_header(&mut md, sequence, 0).expect("failed to write_serial_request_header");
                let ping = msgs::Ping {
                    id: 0,
                    message: WireString("ping".as_bytes().to_vec()),
                };
                msgs::write(&mut md, ping).expect("failed to serial write");
                tx.publish(PUB_TOPIC, false, md.bytes()).await.unwrap();
            }
        });

        let sub_task = tokio::spawn(async move {
            loop {
                let message = rx.recv().await.unwrap();
                // println!("T = {}, P = {:?}", message.topic, message.payload.len());
                // println!("count {}", message.payload.len());
                for payload in message.payload {
                    if let Err(e) = msg_tx.send(payload.to_vec()) {
                        println!("pub err {:?}", e);
                    }
                }
            }
        });

        servers.await;
        println!("server awaited");
        pub_task.await.unwrap();
        println!("pub awaited");
        sub_task.await.unwrap();
        println!("sub awaited");
        console_task.await.unwrap();
    });
}


