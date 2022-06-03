use crate::mqtt::start_broker;
use crate::ChannelRequest;
use sphinx_key_parser::MsgDriver;
use tokio::sync::{mpsc, oneshot};
use vls_protocol::serde_bolt::WireString;
use vls_protocol::{msgs, msgs::Message};

pub fn run_test() {
    log::info!("TEST...");

    let mut id = 0u16;
    let mut sequence = 1;

    let (tx, rx) = mpsc::channel(1000);
    let runtime = start_broker(true, rx);
    log::info!("======> READY received! start now");
    runtime.block_on(async {
        loop {
            if let Err(e) = iteration(id, sequence, tx.clone()).await {
                panic!("iteration failed {:?}", e);
            }
            sequence = sequence.wrapping_add(1);
            id += 1;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });
}

pub async fn iteration(
    id: u16,
    sequence: u16,
    tx: mpsc::Sender<ChannelRequest>,
) -> anyhow::Result<()> {
    let mut md = MsgDriver::new_empty();
    msgs::write_serial_request_header(&mut md, sequence, 0)?;
    let ping = msgs::Ping {
        id,
        message: WireString("ping".as_bytes().to_vec()),
    };
    msgs::write(&mut md, ping)?;
    let (reply_tx, reply_rx) = oneshot::channel();
    // Send a request to the MQTT handler to send to signer
    let request = ChannelRequest {
        message: md.bytes(),
        reply_tx,
    };
    let _ = tx.send(request).await;
    let res = reply_rx.await?;
    let mut ret = MsgDriver::new(res.reply);
    msgs::read_serial_response_header(&mut ret, sequence)?;
    let reply = msgs::read(&mut ret)?;
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
