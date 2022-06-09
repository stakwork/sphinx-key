use crate::{ChannelReply, ChannelRequest};
use librumqttd::{
    async_locallink::construct_broker,
    consolelink::{self, ConsoleLink},
    Config,
};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc;

const SUB_TOPIC: &str = "sphinx-return";
const PUB_TOPIC: &str = "sphinx";

pub fn start_broker(
    mut receiver: mpsc::Receiver<ChannelRequest>,
    status_sender: mpsc::Sender<bool>,
    expected_client_id: &str,
) -> tokio::runtime::Runtime {
    let config: Config = confy::load_path("config/rumqttd.conf").unwrap();
    let client_id = expected_client_id.to_string();

    let (mut router, servers, builder) = construct_broker(config.clone());

    thread::spawn(move || {
        router.start().expect("could not start router");
    });

    let mut client_connected = false;

    // let (status_tx, mut status_rx): (mpsc::Sender<bool>, mpsc::Receiver<bool>) =
    //     mpsc::channel(1000);

    let mut rt_builder = tokio::runtime::Builder::new_multi_thread();
    rt_builder.enable_all();
    let rt = rt_builder.build().unwrap();
    rt.block_on(async {
        tokio::spawn(async move {
            let (msg_tx, mut msg_rx): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) =
                mpsc::channel(1000);
            let (mut tx, mut rx) = builder.clone().connect("localclient", 200).await.unwrap();
            tx.subscribe([SUB_TOPIC]).await.unwrap();

            let router_tx = builder.router_tx();
            tokio::spawn(async move {
                let config = config.clone().into();
                let router_tx = router_tx.clone();
                let console: Arc<ConsoleLink> = Arc::new(ConsoleLink::new(config, router_tx));
                loop {
                    let metrics = consolelink::request_metrics(console.clone(), client_id.clone());
                    match metrics.tracker() {
                        Some(t) => {
                            // wait for subscription to be sure
                            if t.concrete_subscriptions_len() > 0 {
                                if !client_connected {
                                    println!("CLIENT CONNECTED!");
                                    client_connected = true;
                                    status_sender
                                        .send(true)
                                        .await
                                        .expect("couldnt send true statu");
                                }
                            }
                        }
                        None => {
                            if client_connected {
                                println!("CLIENT DIsCONNECTED!");
                                client_connected = false;
                                status_sender
                                    .send(false)
                                    .await
                                    .expect("couldnt send false status");
                            }
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(850)).await;
                }
            });

            let sub_task = tokio::spawn(async move {
                // ready message loop
                // let ready_tx_ = ready_tx.clone();
                loop {
                    // wait for CONNECTED
                    // loop {
                    //     let status = status_rx.recv().await.unwrap();
                    //     if status {
                    //         break;
                    //     }
                    // }
                    // now wait for READY
                    // loop {
                    //     let message = rx.recv().await.unwrap();
                    //     if let Some(payload) = message.payload.get(0) {
                    //         let content = String::from_utf8_lossy(&payload[..]);
                    //         log::info!("received message content: {}", content);
                    //         if content == "READY" {
                    //             // ready_tx.send(true).expect("could not send ready");
                    //             break;
                    //         }
                    //     }
                    // }
                    // now start parsing... or break for DISCONNECT
                    println!("OK START PARSING!");
                    loop {
                        let message = rx.recv().await.unwrap();
                        println!("T = {}, P = {:?}", message.topic, message.payload.len());
                        // println!("count {}", message.payload.len());
                        for payload in message.payload {
                            if let Err(e) = msg_tx.send(payload.to_vec()).await {
                                println!("pub err {:?}", e);
                            }
                        }
                    }
                }
            });

            let relay_task = tokio::spawn(async move {
                while let Some(msg) = receiver.recv().await {
                    tx.publish(PUB_TOPIC, false, msg.message)
                        .await
                        .expect("could not mqtt pub");
                    let reply = msg_rx.recv().await.expect("could not unwrap msg_rx.recv()");
                    if let Err(_) = msg.reply_tx.send(ChannelReply { reply }) {
                        log::warn!("could not send on reply_tx");
                    }
                }
            });

            servers.await;
            sub_task.await.unwrap();
            relay_task.await.unwrap();
        });
    });

    rt
}
