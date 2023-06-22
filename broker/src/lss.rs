use anyhow::Result;
use rocket::tokio::{
    self,
    sync::{mpsc},
};
use std::time::Duration;
use crate::conn::{ChannelRequest, LssReq};
use lss_connector::{LssBroker, Response};
use sphinx_signer::sphinx_glyph::topics;

pub async fn lss_setup(uri: &str, mqtt_tx: mpsc::Sender<ChannelRequest>) -> Result<LssBroker> {
    
    // LSS required
    let (spk, msg_bytes) = LssBroker::get_server_pubkey(uri).await?;
    let reply = ChannelRequest::send(topics::INIT_MSG, msg_bytes, &mqtt_tx).await?;
    let ir = Response::from_slice(&reply)?.as_init()?;

    let lss_conn = LssBroker::new(uri, ir.clone(), spk).await?;
    // this only returns the initial state if it was requested by signer
    let msg_bytes2 = lss_conn.get_created_state_msg(&ir).await?;

    let reply2 = ChannelRequest::send(topics::INIT_MSG, msg_bytes2, &mqtt_tx).await?;
    let cr = Response::from_slice(&reply2)?.as_created()?;

    lss_conn.handle(Response::Created(cr)).await?;

    Ok(lss_conn)
}

pub fn lss_tasks(lss_conn: LssBroker, mut lss_rx: mpsc::Receiver<LssReq>, mut reconn_rx: mpsc::Receiver<(String, bool)>, init_tx: mpsc::Sender<ChannelRequest>) {
    // msg handler (from CLN looper)
    let lss_conn_ = lss_conn.clone();
    tokio::task::spawn(async move{
        while let Some(req) = lss_rx.recv().await {
            match lss_conn_.handle_bytes(&req.message).await {
                Ok(msg) => {
                    let _ = req.reply_tx.send(msg);
                }, 
                Err(e) => {
                    log::error!("failed lss_handle {:?}", e);
                }
            }
        }
    });

    // reconnect handler (when a client reconnects)
    let lss_conn_ = lss_conn.clone();
    let init_tx_ = init_tx.clone();
    tokio::task::spawn(async move{
        while let Some((cid, connected)) = reconn_rx.recv().await {
            if connected {
                log::info!("CLIENT {} reconnected!", cid);
                if let Err(e) = reconnect_dance(&cid, &lss_conn_, &init_tx_).await {
                    log::error!("reconnect dance failed {:?}", e);
                }
            }
        }
    });
}

async fn reconnect_dance(cid: &str, lss_conn: &LssBroker, mqtt_tx: &mpsc::Sender<ChannelRequest>) -> Result<()> {
    // sleep 3 seconds to make sure ESP32 subscription is active
    tokio::time::sleep(Duration::from_secs(3)).await;
    let init_bytes = lss_conn.make_init_msg().await?;
    let reply = ChannelRequest::send_for(cid, topics::INIT_MSG, init_bytes, mqtt_tx).await?;
    let ir = Response::from_slice(&reply)?.as_init()?;
    let state_bytes = lss_conn.get_created_state_msg(&ir).await?;
    let reply2 = ChannelRequest::send_for(cid, topics::INIT_MSG, state_bytes, mqtt_tx).await?;
    let cr = Response::from_slice(&reply2)?.as_created()?;
    lss_conn.handle(Response::Created(cr)).await?;
    Ok(())
}
