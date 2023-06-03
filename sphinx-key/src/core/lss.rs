use crate::conn::mqtt::QOS;
use crate::core::events::Event;
use anyhow::{anyhow, Result};
use embedded_svc::mqtt::client::MessageImpl;
use embedded_svc::utils::mqtt::client::ConnState;
use esp_idf_svc::mqtt::client::EspMqttClient;
use esp_idf_sys::EspError;
use lss_connector::{secp256k1::PublicKey, LssSigner, Msg as LssMsg, Response as LssRes};
use sphinx_signer::sphinx_glyph::topics;
use sphinx_signer::{self, RootHandler, RootHandlerBuilder};
use std::sync::mpsc;

pub fn init_lss(
    client_id: &str,
    rx: &mpsc::Receiver<Event>,
    handler_builder: RootHandlerBuilder,
    mqtt: &mut EspMqttClient<ConnState<MessageImpl, EspError>>,
) -> Result<(RootHandler, LssSigner)> {
    let first_lss_msg = match rx.recv()? {
        Event::LssMessage(b) => b,
        _ => return Err(anyhow!("not a lss msg")),
    };
    let init = LssMsg::from_slice(&first_lss_msg)?.as_init()?;
    let server_pubkey = PublicKey::from_slice(&init.server_pubkey)?;

    let (lss_signer, res1) = LssSigner::new(&handler_builder, &server_pubkey);
    let lss_res_topic = format!("{}/{}", client_id, topics::LSS_RES);
    mqtt.publish(&lss_res_topic, QOS, false, &res1)
        .expect("could not publish LSS response");

    let second_lss_msg = match rx.recv()? {
        Event::LssMessage(b) => b,
        _ => return Err(anyhow!("not a lss msg")),
    };
    let created = LssMsg::from_slice(&second_lss_msg)?.as_created()?;
    let (root_handler, res2) = lss_signer.build_with_lss(created, handler_builder)?;
    mqtt.publish(&lss_res_topic, QOS, false, &res2)
        .expect("could not publish LSS response 2");

    Ok((root_handler, lss_signer))
}
