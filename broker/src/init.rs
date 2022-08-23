use crate::ChannelRequest;
use bitcoin::Network;
use sphinx_key_parser as parser;
use sphinx_key_parser::MsgDriver;
use tokio::sync::{mpsc, oneshot};
use vls_protocol::model::Secret;
use vls_protocol::{msgs, serde_bolt::WireString};
use vls_proxy::util::{read_allowlist, read_integration_test_seed};

pub fn blocking_connect(tx: mpsc::Sender<ChannelRequest>, network: Network) {
    let init_msg_2 = crate::init::make_init_msg(network).expect("couldnt make init msg");
    let (reply_tx, reply_rx) = oneshot::channel();
    // Send a request to the MQTT handler to send to signer
    let request = ChannelRequest {
        message: init_msg_2,
        reply_tx,
    };
    tx.blocking_send(request).expect("could not blocking send");
    let res = reply_rx.blocking_recv().expect("couldnt receive");
    let reply = parser::response_from_bytes(res.reply, 0).expect("couldnt parse init receive");
    println!("REPLY {:?}", reply);
}

pub async fn _connect(tx: mpsc::Sender<ChannelRequest>, network: Network) {
    let init_msg_2 = crate::init::make_init_msg(network).expect("could make init msg");
    let (reply_tx, reply_rx) = oneshot::channel();
    // Send a request to the MQTT handler to send to signer
    let request = ChannelRequest {
        message: init_msg_2,
        reply_tx,
    };
    let _ = tx.send(request).await;
    let res = reply_rx.await.expect("couldnt receive");
    let reply = parser::response_from_bytes(res.reply, 0).expect("could parse init receive");
    println!("REPLY {:?}", reply);
}

pub fn make_init_msg(network: Network) -> anyhow::Result<Vec<u8>> {
    let allowlist = read_allowlist()
        .into_iter()
        .map(|s| WireString(s.as_bytes().to_vec()))
        .collect::<Vec<_>>();
    let seed = if network == Network::Bitcoin {
        Some(Secret([
            0x8c, 0xe8, 0x62, 0xab, 0xd5, 0x6b, 0xb4, 0x6a, 0x61, 0x7f, 0xaf, 0x13, 0x50, 0xc1,
            0xca, 0xf5, 0xb1, 0xee, 0x02, 0x97, 0xbf, 0xf3, 0xb8, 0xc9, 0x56, 0x63, 0x58, 0x9f,
            0xec, 0x8c, 0x45, 0x79,
        ]))
    } else {
        read_integration_test_seed()
            .map(|s| Secret(s))
            .or(Some(Secret([1; 32])))
    };
    // FIXME remove this
    log::info!("allowlist {:?} seed {:?}", allowlist, seed);
    let init = msgs::HsmdInit2 {
        derivation_style: 0,
        network_name: WireString(network.to_string().as_bytes().to_vec()),
        dev_seed: seed,
        dev_allowlist: allowlist,
    };
    let sequence = 0;
    let mut md = MsgDriver::new_empty();
    msgs::write_serial_request_header(&mut md, sequence, 0)?;
    msgs::write(&mut md, init)?;
    Ok(md.bytes())
    // msgs::read_serial_response_header(&mut serial, sequence)?;
    // let init_reply: msgs::HsmdInit2Reply = msgs::read_message(&mut serial)?;
    // log::info!("init reply {:?}", init_reply);
    // Ok(())
}
