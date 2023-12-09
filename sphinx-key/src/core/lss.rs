use crate::conn::mqtt::QOS;
use crate::core::events::Event;
use anyhow::{anyhow, Result};
use esp_idf_svc::mqtt::client::ConnState;
use esp_idf_svc::mqtt::client::EspMqttClient;
use esp_idf_svc::mqtt::client::MessageImpl;
use esp_idf_svc::sys::EspError;
use lss_connector::{secp256k1::PublicKey, BrokerMutations, LssSigner, Msg as LssMsg};
use sphinx_signer::sphinx_glyph::topics;
use sphinx_signer::{self, RootHandler, RootHandlerBuilder};
use std::sync::mpsc;
use std::time::Duration;

pub use lss_connector::handle_lss_msg;

pub fn init_lss(
    signer_id: &[u8; 16],
    rx: &mpsc::Receiver<Event>,
    handler_builder: RootHandlerBuilder,
    mqtt: &mut EspMqttClient<ConnState<MessageImpl, EspError>>,
) -> Result<(RootHandler, LssSigner)> {
    let client_id = hex::encode(signer_id);

    let server_pubkey = loop {
        let event = rx.recv_timeout(Duration::from_secs(30))?;
        match server_pubkey_from_event(event) {
            Ok(spk) => break spk,
            Err(e) => log::warn!("not server_pubkey_from_event {:?}", e),
        }
    };

    let (lss_signer, res1) = LssSigner::new(&handler_builder, &server_pubkey, None);
    let lss_res_1_topic = format!("{}/{}", client_id, topics::INIT_1_RES);
    mqtt.publish(&lss_res_1_topic, QOS, false, &res1)
        .expect("could not publish LSS response");

    let created = loop {
        let event = rx.recv_timeout(Duration::from_secs(30))?;
        match created_from_event(event) {
            Ok(c) => break c,
            Err(e) => log::warn!("not created_from_event {:?}", e),
        }
    };

    let (root_handler, res2) = lss_signer.build_with_lss(created, handler_builder, None)?;
    let lss_res_2_topic = format!("{}/{}", client_id, topics::INIT_2_RES);
    mqtt.publish(&lss_res_2_topic, QOS, false, &res2)
        .expect("could not publish LSS response 2");

    Ok((root_handler, lss_signer))
}

fn server_pubkey_from_event(event: Event) -> anyhow::Result<PublicKey> {
    match event {
        Event::LssMessage(b) => {
            let init = LssMsg::from_slice(&b)?.into_init()?;
            let server_pubkey = PublicKey::from_slice(&init.server_pubkey)?;
            Ok(server_pubkey)
        }
        _m => Err(anyhow!("not an LSS msg")),
    }
}
fn created_from_event(event: Event) -> anyhow::Result<BrokerMutations> {
    match event {
        Event::LssMessage(b) => Ok(LssMsg::from_slice(&b)?.into_created()?),
        _ => Err(anyhow!("not an LSS msg")),
    }
}
