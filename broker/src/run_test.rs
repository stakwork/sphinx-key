use crate::mqtt::start_broker;
use crate::ChannelRequest;
use sphinx_key_parser as parser;
use tokio::sync::{mpsc, oneshot};
use vls_protocol::serde_bolt::WireString;
use vls_protocol::{msgs, msgs::Message};

const CLIENT_ID: &str = "test-1";

pub fn run_test() {
    log::info!("TEST...");

    let mut id = 0u16;
    let mut sequence = 1;

    let (tx, rx) = mpsc::channel(1000);
    let (status_tx, mut status_rx) = mpsc::channel(1000);
    let runtime = start_broker(rx, status_tx, CLIENT_ID);
    runtime.block_on(async {
        let mut connected = false;
        loop {
            tokio::select! {
                status = status_rx.recv() => {
                    if let Some(connection_status) = status {
                        connected = connection_status;
                        id = 0;
                        sequence = 1;
                        log::info!("========> CONNECTED! {}", connection_status);
                    }
                }
                res = iteration(id, sequence, tx.clone(), connected) => {
                    if let Err(e) = res {
                        log::warn!("===> iteration failed {:?}", e);
                        // connected = false;
                        // id = 0;
                        // sequence = 1;
                    } else if connected {
                        sequence = sequence.wrapping_add(1);
                        id += 1;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            };
        }
    });
}

pub async fn iteration(
    id: u16,
    sequence: u16,
    tx: mpsc::Sender<ChannelRequest>,
    connected: bool,
) -> anyhow::Result<()> {
    if !connected {
        return Ok(());
    }
    log::info!("do a ping!");
    let ping = msgs::Ping {
        id,
        message: WireString("ping".as_bytes().to_vec()),
    };
    let ping_bytes = parser::request_from_msg(ping, sequence, 0)?;
    let (reply_tx, reply_rx) = oneshot::channel();
    // Send a request to the MQTT handler to send to signer
    let request = ChannelRequest {
        message: ping_bytes,
        reply_tx,
    };
    tx.send(request).await?;
    println!("tx.send(request)");
    let res = reply_rx.await?;
    println!("reply_rx.await");
    let reply = parser::response_from_bytes(res.reply, sequence)?;
    match reply {
        Message::Pong(p) => {
            log::info!(
                "got reply {} {}",
                p.id,
                String::from_utf8(p.message.0).unwrap()
            );
            assert_eq!(p.id, id);
        }
        _ => {
            panic!("unknown response");
        }
    }
    Ok(())
}