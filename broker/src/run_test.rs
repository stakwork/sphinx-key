use crate::mqtt::start_broker;
use crate::routes::launch_rocket;
use crate::util::Settings;
use crate::ChannelRequest;
use rocket::tokio::{self, sync::broadcast, sync::mpsc};
use sphinx_signer::{parser, sphinx_glyph::topics};
use vls_protocol::serde_bolt::WireString;
use vls_protocol::{msgs, msgs::Message};

const CLIENT_ID: &str = "test-1";

pub async fn run_test() -> rocket::Rocket<rocket::Build> {
    log::info!("TEST...");

    // let mut id = 0u16;
    // let mut sequence = 1;

    let settings = Settings::default();

    let (tx, rx) = mpsc::channel(1000);
    let (status_tx, mut status_rx) = mpsc::channel(1000);
    let (error_tx, error_rx) = broadcast::channel(1000);
    crate::error_log::log_errors(error_rx);

    start_broker(rx, status_tx, error_tx.clone(), CLIENT_ID, settings)
        .expect("FAILED TO START BROKER");
    log::info!("BROKER started!");

    log::info!("=> wait for connected status");
    // wait for connection = true
    let status = status_rx.recv().await.expect("couldnt receive");
    log::info!("=> connection status: {}", status);

    // let mut connected = false;
    // let tx_ = tx.clone();
    tokio::spawn(async move {
        while let Some(status) = status_rx.recv().await {
            log::info!("========> CONNECTED! {}", status);
        }
    });
    // tokio::spawn(async move {
    //     loop {
    //         tokio::select! {
    //             status = status_rx.recv() => {
    //                 if let Some(connection_status) = status {
    //                     connected = connection_status;
    //                     id = 0;
    //                     sequence = 1;
    //                     log::info!("========> CONNECTED! {}", connection_status);
    //                 }
    //             }
    //             res = iteration(id, sequence, tx_.clone(), connected) => {
    //                 if let Err(e) = res {
    //                     log::warn!("===> iteration failed {:?}", e);
    //                     // connected = false;
    //                     // id = 0;
    //                     // sequence = 1;
    //                 } else if connected {
    //                     sequence = sequence.wrapping_add(1);
    //                     id += 1;
    //                 }
    //                 tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    //             }
    //         };
    //     }
    // });
    launch_rocket(tx, error_tx, settings)
}

#[allow(dead_code)]
#[allow(unused)]
pub async fn iteration(
    id: u16,
    sequence: u16,
    tx: mpsc::Sender<ChannelRequest>,
    connected: bool,
) -> anyhow::Result<()> {
    return Ok(());
    if !connected {
        return Ok(());
    }
    log::info!("do a ping!");
    let ping = msgs::Ping {
        id,
        message: WireString("ping".as_bytes().to_vec()),
    };
    let ping_bytes = parser::request_from_msg(ping, sequence, 0)?;
    // Send a request to the MQTT handler to send to signer
    let (request, reply_rx) = ChannelRequest::new(topics::VLS, ping_bytes);
    tx.send(request).await?;
    let res = reply_rx.await?;
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
