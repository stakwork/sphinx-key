use crate::conn::mqtt::RETURN_TOPIC;
use sphinx_key_signer::{self, InitResponse, PubKey};
use std::sync::{mpsc};

use esp_idf_sys;
use embedded_svc::httpd::Result;
use embedded_svc::mqtt::client::utils::ConnState;
use embedded_svc::mqtt::client::{MessageImpl, Publish, QoS};
use esp_idf_svc::mqtt::client::*;
use esp_idf_sys::EspError;
use log::*;

pub fn make_event_loop(mut mqtt: EspMqttClient<ConnState<MessageImpl, EspError>>, rx: mpsc::Receiver<Vec<u8>>) -> Result<()> {

    // initialize the RootHandler
    let init_msg_bytes = rx.recv().expect("NO INIT MSG");
    let InitResponse { root_handler, init_reply } = sphinx_key_signer::init(init_msg_bytes).expect("failed to init signer");
    mqtt.publish(RETURN_TOPIC, QoS::AtMostOnce, false, init_reply).expect("could not publish init response");

    // signing loop
    let dummy_peer = PubKey([0; 33]);
    while let Ok(msg_bytes) = rx.recv() {
        let _ret = match sphinx_key_signer::handle(&root_handler, msg_bytes, dummy_peer.clone()) {
            Ok(b) =>  mqtt.publish(RETURN_TOPIC, QoS::AtMostOnce, false, b).expect("could not publish init response"),
            Err(e) => panic!("HANDLE FAILED {:?}", e),
        };
    }

    Ok(())
}

pub fn make_test_event_loop(mut mqtt: EspMqttClient<ConnState<MessageImpl, EspError>>, rx: mpsc::Receiver<Vec<u8>>) -> Result<()> {

    info!("About to subscribe to the mpsc channel");
    while let Ok(msg_bytes) = rx.recv() {
        let b = sphinx_key_signer::parse_ping_and_form_response(msg_bytes);
        log::info!("GOT A PING MESSAGE! returning pong now...");
        mqtt.publish(RETURN_TOPIC, QoS::AtMostOnce, false, b).expect("could not publish init response");
    }

    Ok(())
}
