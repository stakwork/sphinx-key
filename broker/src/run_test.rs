use crate::conn::ChannelRequest;
use crate::routes::launch_rocket;
use crate::util::Settings;
use rocket::tokio::{self, sync::broadcast, sync::mpsc, task::JoinSet};
use sphinx_signer::vls_protocol::{msgs, msgs::Message};
use sphinx_signer::{parser, sphinx_glyph::topics};
use vls_protocol::serde_bolt::WireString;

// const CLIENT_ID: &str = "test-1";

pub fn run_test() -> rocket::Rocket<rocket::Build> {
    let mut set = JoinSet::<()>::new();
    log::info!("TEST...");

    // let mut id = 0u16;
    // let mut sequence = 1;

    let settings = Settings::default();
    let (mqtt_tx, mqtt_rx) = mpsc::channel(10000);
    let (_init_tx, init_rx) = mpsc::channel(10000);
    let (error_tx, error_rx) = broadcast::channel(10000);
    let (conn_tx, _conn_rx) = mpsc::channel(10000);

    crate::error_log::log_errors(error_rx, &mut set);

    // block until connection
    crate::broker_setup(
        settings,
        mqtt_rx,
        init_rx,
        conn_tx,
        error_tx.clone(),
        &mut set,
    );
    log::info!("=> off to the races!");

    let tx_ = mqtt_tx.clone();
    set.spawn(async move {
        let mut id = 0;
        let mut sequence = 0;
        loop {
            // select! (
            //     status = status_rx.recv() => {
            //         if let Some(connection_status) = status {
            //             connected = connection_status;
            //             id = 0;
            //             sequence = 1;
            //             log::info!("========> CONNECTED! {}", connection_status);
            //         }
            //     }
            //     res = iteration(id, sequence, tx_.clone(), connected) => {
            //         if let Err(e) = res {
            //             log::warn!("===> iteration failed {:?}", e);
            //             // connected = false;
            //             // id = 0;
            //             // sequence = 1;
            //         } else if connected {
            //             sequence = sequence.wrapping_add(1);
            //             id += 1;
            //         }
            //         std::thread::sleep(std::time::Duration::from_secs(1)).await;
            //     }
            // )
            let res = iteration(id, sequence, tx_.clone()).await;
            if let Err(e) = res {
                log::warn!("===> iteration failed {:?}", e);
            } else {
                sequence = sequence.wrapping_add(1);
                id += 1;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });

    launch_rocket(mqtt_tx, error_tx, settings)
}

#[allow(dead_code)]
#[allow(unused)]
pub async fn iteration(
    id: u16,
    sequence: u16,
    tx: mpsc::Sender<ChannelRequest>,
    // connected: bool,
) -> anyhow::Result<()> {
    // return Ok(());
    // if !connected {
    //     return Ok(());
    // }
    log::info!("do a ping!");
    let ping = msgs::Ping {
        id,
        message: WireString("ping".as_bytes().to_vec()),
    };
    let peer_id = [0u8; 33];
    let ping_bytes = parser::request_from_msg(ping, sequence, peer_id, 0)?;
    // Send a request to the MQTT handler to send to signer
    let cid = hex::encode(peer_id);
    let (request, reply_rx) = ChannelRequest::new(&cid, topics::VLS, ping_bytes);
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
