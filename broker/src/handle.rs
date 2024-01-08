use crate::conn::{
    current_client, current_client_and_synced, cycle_clients, ChannelRequest, LssReq,
};
use crate::looper::ClientId;
use rocket::tokio::sync::mpsc;
use sphinx_signer::{parser, sphinx_glyph::topics};
use std::sync::atomic::{AtomicU16, Ordering};
use std::thread;
use std::time::Duration;
use vls_protocol::{Error, Result};

static COUNTER: AtomicU16 = AtomicU16::new(0u16);
static CURRENT: AtomicU16 = AtomicU16::new(0u16);

pub fn take_a_ticket() -> u16 {
    COUNTER.fetch_add(1u16, Ordering::SeqCst)
}

pub fn is_my_turn(ticket: u16) -> bool {
    let curr = CURRENT.load(Ordering::SeqCst);
    curr == ticket
}

pub fn my_turn_is_done() {
    CURRENT.fetch_add(1u16, Ordering::SeqCst);
}

pub fn handle_message(
    client_id: &Option<ClientId>,
    message: Vec<u8>,
    vls_tx: &mpsc::Sender<ChannelRequest>,
    lss_tx: &mpsc::Sender<LssReq>,
) -> Vec<u8> {
    // wait until not busy
    let ticket = take_a_ticket();
    loop {
        if is_my_turn(ticket) {
            break;
        } else {
            thread::sleep(Duration::from_millis(96));
        }
    }

    let res_bytes = loop {
        let (cid, is_synced) = current_client_and_synced();
        if cid.is_none() {
            log::debug!("no client yet... retry");
            thread::sleep(Duration::from_millis(96));
            continue;
        }
        if !is_synced {
            log::debug!("current client still syncing...");
            thread::sleep(Duration::from_millis(96));
            continue;
        }
        let cid = cid.unwrap();

        let ret = handle_message_inner(client_id, message.clone(), vls_tx, lss_tx, ticket, &cid);

        match ret {
            Ok(b) => break b,
            Err(e) => {
                log::warn!("error handle_message_inner, trying again... {:?}", e);
                cycle_clients(&cid);
                thread::sleep(Duration::from_millis(96));
            }
        }
    };

    // next turn
    my_turn_is_done();

    res_bytes
}

pub fn handle_message_inner(
    client_id: &Option<ClientId>,
    message: Vec<u8>,
    vls_tx: &mpsc::Sender<ChannelRequest>,
    lss_tx: &mpsc::Sender<LssReq>,
    sequence: u16,
    cid: &str,
) -> Result<Vec<u8>> {
    let dbid = client_id.as_ref().map(|c| c.dbid).unwrap_or(0);
    let peer_id = client_id
        .as_ref()
        .map(|c| c.peer_id.serialize())
        .unwrap_or([0u8; 33]);
    let md = parser::raw_request_from_bytes(message, sequence, peer_id, dbid)?;
    // send to signer
    log::info!("SEND ON {}", topics::VLS);
    let (res_topic, res) = send_request_wait(vls_tx, cid, topics::VLS, md)?;
    cancel_if_current_client_changed(cid)?;

    log::info!("GOT ON {}", res_topic);
    let the_res = if res_topic == topics::LSS_RES {
        // send reply to LSS to store muts
        let lss_reply = send_lss(lss_tx, topics::LSS_MSG.to_string(), res)?;
        cancel_if_current_client_changed(cid)?;

        log::info!("LSS REPLY LEN {}", &lss_reply.1.len());
        // send to signer for HMAC validation, and get final reply
        log::info!("SEND ON {}", lss_reply.0);
        let (res_topic2, res2) = send_request_wait(vls_tx, cid, &lss_reply.0, lss_reply.1)?;
        cancel_if_current_client_changed(cid)?;

        log::info!("GOT ON {}, send to CLN?", res_topic2);
        if res_topic2 != topics::VLS_RES {
            log::warn!("got a topic NOT on {}", topics::VLS_RES);
            return Err(Error::Io("PutConflict".to_string()));
        }
        res2
    } else {
        res
    };
    // create reply bytes for CLN
    let reply = parser::raw_response_from_bytes(the_res, sequence)?;

    Ok(reply)
}

fn cancel_if_current_client_changed(cid: &str) -> Result<()> {
    if let Some(cc) = current_client() {
        if cc != cid {
            return Err(Error::Io("current client changed".to_string()));
        }
    }
    Ok(())
}

// returns (topic, payload)
// might halt if signer is offline
fn send_request_wait(
    chan: &mpsc::Sender<ChannelRequest>,
    cid: &str,
    topic: &str,
    message: Vec<u8>,
) -> Result<(String, Vec<u8>)> {
    // Send a request to the MQTT handler to send to signer
    let (request, reply_rx) = ChannelRequest::new(cid, topic, message);
    // This can fail if MQTT shuts down
    chan.blocking_send(request).map_err(|_| Error::Eof)?;
    let reply = reply_rx.blocking_recv().map_err(|_| Error::Eof)?;
    if reply.is_empty() {
        log::warn!("no reply from signer...");
        return Err(Error::Eof);
    }

    Ok((reply.topic_end, reply.reply))
}

fn send_lss(
    lss_tx: &mpsc::Sender<LssReq>,
    topic: String,
    message: Vec<u8>,
) -> Result<(String, Vec<u8>)> {
    // Send a request to the LSS server
    let (request, reply_rx) = LssReq::new(topic, message);
    lss_tx.blocking_send(request).map_err(|_| Error::Eof)?;
    let res = reply_rx.blocking_recv().map_err(|_| Error::Eof)?;
    Ok(res)
}
