use crate::{ChannelRequest,ChannelReply};
use librumqttd::{async_locallink::construct_broker, Config};
use std::thread;
use tokio::sync::{oneshot, mpsc};

const SUB_TOPIC: &str = "sphinx-return";
const PUB_TOPIC: &str = "sphinx";

pub fn start_broker(wait_for_ready_message: bool, mut receiver: mpsc::Receiver<ChannelRequest>) -> tokio::runtime::Runtime {

    let config: Config = confy::load_path("config/rumqttd.conf").unwrap();

    let (mut router, console, servers, builder) = construct_broker(config);

    thread::spawn(move || {
        router.start().expect("could not start router");
    });

    let mut rt_builder = tokio::runtime::Builder::new_multi_thread();
    rt_builder.enable_all();
    let rt = rt_builder.build().unwrap();
    rt.block_on(async {
        // channel to block until READY received
        let (ready_tx, ready_rx) = oneshot::channel();
        tokio::spawn(async move {
            let (msg_tx, mut msg_rx): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) = mpsc::channel(1000);
            let (mut tx, mut rx) = builder.connect("localclient", 200).await.unwrap();
            tx.subscribe([SUB_TOPIC]).await.unwrap();

            let console_task = tokio::spawn(console);

            let sub_task = tokio::spawn(async move {
                // ready message loop
                // let ready_tx_ = ready_tx.clone();
                if wait_for_ready_message {
                    loop {
                        let message = rx.recv().await.unwrap();
                        if let Some(payload) = message.payload.get(0) {
                            let content = String::from_utf8_lossy(&payload[..]);
                            if content == "READY" {
                                ready_tx.send(true).expect("could not send ready");
                                break;
                            }
                        }
                    }
                }
                loop {
                    let message = rx.recv().await.unwrap();
                    // println!("T = {}, P = {:?}", message.topic, message.payload.len());
                    // println!("count {}", message.payload.len());
                    for payload in message.payload {
                        if let Err(e) = msg_tx.send(payload.to_vec()).await {
                            println!("pub err {:?}", e);
                        }
                    }
                }
            });

            let relay_task = tokio::spawn(async move {
                while let Some(msg) = receiver.recv().await {
                    tx.publish(PUB_TOPIC, false, msg.message).await.expect("could not mqtt pub");
                    let reply = msg_rx.recv().await.expect("could not unwrap msg_rx.recv()");
                    if let Err(_) = msg.reply_tx.send(ChannelReply { reply }) {
                        log::warn!("could not send on reply_tx");
                    }
                }
            });

            servers.await;
            sub_task.await.unwrap();
            relay_task.await.unwrap();
            console_task.await.unwrap();
        });
        if wait_for_ready_message {
            log::info!("waiting for READY...");
            ready_rx.await.expect("Could not receive from channel.");
        }
    });

    rt
}
