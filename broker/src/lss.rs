use crate::conn::{ChannelRequest, LssReq};
use anyhow::{anyhow, Result};
use lss_connector::{InitResponse, LssBroker, Response, SignerMutations};
use rocket::tokio;
use rumqttd::oneshot;
use rumqttd::oneshot as std_oneshot;
use sphinx_signer::parser;
use sphinx_signer::sphinx_glyph::topics;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use vls_protocol::msgs::{self, Message, SerBolt};
use vls_proxy::client::{Client, UnixClient};

pub fn lss_tasks(
    uri: String,
    lss_rx: mpsc::Receiver<LssReq>,
    mut conn_rx: mpsc::Receiver<(String, oneshot::Sender<bool>)>,
    init_tx: mpsc::Sender<ChannelRequest>,
    mut cln_client: UnixClient,
    mut hsmd_raw: Vec<u8>,
    task_set: &mut JoinSet<()>,
) {
    task_set.spawn(async move {
        // first connection - initializes lssbroker
        let (lss_conn, hsmd_init_reply) = loop {
            let (cid, dance_complete_tx) = conn_rx.recv().await.unwrap();
            match try_dance(&cid, &uri, None, &init_tx, dance_complete_tx, &mut hsmd_raw).await {
                Some(ret) => break ret,
                None => log::warn!("broker not initialized, try connecting again..."),
            }
        };
        cln_client.write_vec(hsmd_init_reply).unwrap();
        spawn_lss_rx(lss_conn.clone(), lss_rx);
        // connect handler for all subsequent connections
        while let Some((cid, dance_complete_tx)) = conn_rx.recv().await {
            log::info!("CLIENT {} connected!", cid);
            let _ = try_dance(
                &cid,
                &uri,
                Some(&lss_conn),
                &init_tx,
                dance_complete_tx,
                &mut hsmd_raw,
            )
            .await;
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
    hsmd_raw: &mut Vec<u8>,
) -> Option<(LssBroker, Vec<u8>)> {
    match connect_dance(cid, uri, lss_conn, init_tx, hsmd_raw).await {
        Ok(ret) => {
            let _ = dance_complete_tx.send(true);
            // none if lss_conn is some, some otherwise
            ret
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
    lss_conn_opt: Option<&LssBroker>,
    mqtt_tx: &mpsc::Sender<ChannelRequest>,
    hsmd_raw: &mut Vec<u8>,
) -> Result<Option<(LssBroker, Vec<u8>)>> {
    let (new_broker, ir) = dance_step_1(cid, uri, lss_conn_opt, mqtt_tx).await?;
    let lss_conn = new_broker.as_ref().xor(lss_conn_opt).ok_or(anyhow!(
        "should never happen, either we use the newly initialized, or the one passed in"
    ))?;
    dance_step_2(cid, lss_conn, mqtt_tx, &ir).await?;
    let hsmd_init_reply = dance_step_3(cid, mqtt_tx, hsmd_raw).await?;
    // only some when lss_conn_opt is none
    Ok(new_broker.map(|broker| (broker, hsmd_init_reply)))
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

async fn dance_step_3(
    cid: &str,
    mqtt_tx: &mpsc::Sender<ChannelRequest>,
    hsmd_raw: &mut Vec<u8>,
) -> Result<Vec<u8>> {
    let Message::HsmdInit(mut hsmd_init) = msgs::from_vec(hsmd_raw.clone()).unwrap() else {
        panic!("Expected a hsmd init message here")
    };
    let hsmd_init_bytes = parser::raw_request_from_bytes(hsmd_raw.clone(), 0, [0u8; 33], 0)?;
    let reply = ChannelRequest::send(cid, topics::INIT_3_MSG, hsmd_init_bytes, mqtt_tx).await?;
    if reply.is_empty() {
        return Err(anyhow!("Hsmd init failed !"));
    }
    let hsmd_init_reply = parser::raw_response_from_bytes(reply, 0).unwrap();
    // this match is a noop after the first pass
    match msgs::from_vec(hsmd_init_reply.clone()) {
        Ok(Message::HsmdInitReplyV4(hir)) => {
            hsmd_init.hsm_wire_max_version = hir.hsm_version;
            hsmd_init.hsm_wire_min_version = hir.hsm_version;
            *hsmd_raw = hsmd_init.as_vec();
        }
        _ => panic!("Not a hsmd init reply v4"),
    };
    Ok(hsmd_init_reply)
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
