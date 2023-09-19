use crate::conn::{ChannelRequest, LssReq};
use anyhow::{anyhow, Result};
use lss_connector::{InitResponse, LssBroker, Response, SignerMutations};
use rocket::tokio;
use rumqttd::oneshot;
use rumqttd::oneshot as std_oneshot;
use sphinx_signer::sphinx_glyph::topics;
use tokio::sync::mpsc;

pub fn lss_tasks(
    uri: String,
    lss_rx: mpsc::Receiver<LssReq>,
    mut conn_rx: mpsc::Receiver<(String, oneshot::Sender<bool>)>,
    init_tx: mpsc::Sender<ChannelRequest>,
) {
    tokio::task::spawn(async move {
        // first connection - initializes lssbroker
        let lss_conn = loop {
            let (cid, dance_complete_tx) = conn_rx.recv().await.unwrap();
            match try_dance(&cid, &uri, None, &init_tx, dance_complete_tx).await {
                Some(broker) => break broker,
                None => log::warn!("broker not initialized, try connecting again..."),
            }
        };
        spawn_lss_rx(lss_conn.clone(), lss_rx);
        // connect handler for all subsequent connections
        while let Some((cid, dance_complete_tx)) = conn_rx.recv().await {
            log::info!("CLIENT {} connected!", cid);
            let _ = try_dance(&cid, &uri, Some(&lss_conn), &init_tx, dance_complete_tx).await;
        }
    });
}

fn spawn_lss_rx(lss_conn: LssBroker, mut lss_rx: mpsc::Receiver<LssReq>) {
    tokio::task::spawn(async move {
        while let Some(req) = lss_rx.recv().await {
            match lss_conn.handle_bytes(&req.message).await {
                Ok(msg) => {
                    let _ = req.reply_tx.send(msg);
                }
                Err(e) => {
                    log::error!("failed lss_handle {:?}", e);
                }
            }
        }
    });
}

async fn try_dance(
    cid: &str,
    uri: &str,
    lss_conn: Option<&LssBroker>,
    init_tx: &mpsc::Sender<ChannelRequest>,
    dance_complete_tx: std_oneshot::Sender<bool>,
) -> Option<LssBroker> {
    match connect_dance(cid, uri, lss_conn, init_tx).await {
        Ok(broker) => {
            let _ = dance_complete_tx.send(true);
            // none if lss_conn is some, some otherwise
            broker
        }
        Err(e) => {
            log::warn!("connect_dance failed: {:?}", e);
            let _ = dance_complete_tx.send(false);
            None
        }
    }
}

async fn connect_dance(
    cid: &str,
    uri: &str,
    lss_conn: Option<&LssBroker>,
    mqtt_tx: &mpsc::Sender<ChannelRequest>,
) -> Result<Option<LssBroker>> {
    let (new_broker, ir) = dance_step_1(cid, uri, lss_conn, mqtt_tx).await?;
    let lss_conn = new_broker.as_ref().xor(lss_conn).ok_or(anyhow!(
        "should never happen, either we use the newly initialized, or the one passed in"
    ))?;
    let _ = dance_step_2(cid, lss_conn, mqtt_tx, &ir).await?;
    // only some when lss_conn is none
    Ok(new_broker)
}

// initializes a new broker in case lss_conn is none
async fn dance_step_1(
    cid: &str,
    uri: &str,
    lss_conn: Option<&LssBroker>,
    mqtt_tx: &mpsc::Sender<ChannelRequest>,
) -> Result<(Option<LssBroker>, InitResponse)> {
    match lss_conn {
        Some(lss_conn) => {
            let init_bytes = lss_conn.make_init_msg().await?;
            let ir = send_init(cid, init_bytes, mqtt_tx).await?;
            Ok((None, ir))
        }
        None => {
            let (spk, init_bytes) = LssBroker::get_server_pubkey(uri).await?;
            let ir = send_init(cid, init_bytes, mqtt_tx).await?;
            let lss_conn = Some(LssBroker::new(uri, ir.clone(), spk).await?);
            Ok((lss_conn, ir))
        }
    }
}

async fn dance_step_2(
    cid: &str,
    lss_conn: &LssBroker,
    mqtt_tx: &mpsc::Sender<ChannelRequest>,
    ir: &InitResponse,
) -> Result<()> {
    let state_bytes = lss_conn.get_created_state_msg(ir).await?;
    let cr = send_created(cid, state_bytes, mqtt_tx).await?;
    lss_conn.handle(Response::Created(cr)).await;
    Ok(())
}

async fn send_init(
    cid: &str,
    msg_bytes: Vec<u8>,
    mqtt_tx: &mpsc::Sender<ChannelRequest>,
) -> Result<InitResponse> {
    let reply = ChannelRequest::send(cid, topics::INIT_1_MSG, msg_bytes, mqtt_tx).await?;
    if reply.is_empty() {
        return Err(anyhow!("send init did not complete, reply is empty"));
    }
    let ir = Response::from_slice(&reply)?.into_init()?;
    Ok(ir)
}

async fn send_created(
    cid: &str,
    msg_bytes: Vec<u8>,
    mqtt_tx: &mpsc::Sender<ChannelRequest>,
) -> Result<SignerMutations> {
    let reply = ChannelRequest::send(cid, topics::INIT_2_MSG, msg_bytes, mqtt_tx).await?;
    if reply.is_empty() {
        return Err(anyhow!("send created did not complete, reply is empty"));
    }
    let cr = Response::from_slice(&reply)?.into_created()?;
    Ok(cr)
}
