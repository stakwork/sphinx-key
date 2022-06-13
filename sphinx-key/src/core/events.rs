use crate::conn::mqtt::{RETURN_TOPIC, TOPIC, QOS};
use sphinx_key_signer::{self, InitResponse, PubKey};
use std::sync::mpsc;

use esp_idf_sys;
use embedded_svc::mqtt::client::Client;
use embedded_svc::httpd::Result;
use embedded_svc::mqtt::client::utils::ConnState;
use embedded_svc::mqtt::client::{MessageImpl, Publish};
use esp_idf_svc::mqtt::client::*;
use esp_idf_sys::EspError;

pub enum Event {
    Connected,
    Disconnected,
    Message(Vec<u8>)
}

#[cfg(not(feature = "pingpong"))]
pub fn make_event_loop(mut mqtt: EspMqttClient<ConnState<MessageImpl, EspError>>, rx: mpsc::Receiver<Event>, do_log: bool) -> Result<()> {

    // initialize the RootHandler
    let root_handler = loop {
        if let Ok(event) = rx.recv() {
            if let Event::Message(msg_bytes) = event {
                let InitResponse { root_handler, init_reply } = sphinx_key_signer::init(msg_bytes).expect("failed to init signer");
                mqtt.publish(RETURN_TOPIC, QOS, false, init_reply).expect("could not publish init response");
                break root_handler
            }
        }
    };

    // signing loop
    let dummy_peer = PubKey([0; 33]);
    while let Ok(event) = rx.recv() {
        match event {
            Event::Connected => {
                log::info!("SUBSCRIBE TO {}", TOPIC);
                mqtt.subscribe(TOPIC, QOS).expect("could not MQTT subscribe");
            },
            Event::Message(ref msg_bytes) => {
                let _ret = match sphinx_key_signer::handle(&root_handler, msg_bytes.clone(), dummy_peer.clone(), do_log) {
                    Ok(b) =>  mqtt.publish(RETURN_TOPIC, QOS, false, b).expect("could not publish init response"),
                    Err(e) => panic!("HANDLE FAILED {:?}", e),
                };
            },
            Event::Disconnected => {
                log::info!("GOT A Event::Disconnected msg!");
            }
        }
    }

    Ok(())
}

#[cfg(feature = "pingpong")]
pub fn make_event_loop(mut mqtt: EspMqttClient<ConnState<MessageImpl, EspError>>, rx: mpsc::Receiver<Event>, do_log: bool) -> Result<()> {

    log::info!("About to subscribe to the mpsc channel");
    while let Ok(event) = rx.recv() {
        match event {
            Event::Connected => {
                log::info!("SUBSCRIBE TO {}", TOPIC);
                mqtt.subscribe(TOPIC, QOS).expect("could not MQTT subscribe");
            },
            Event::Message(msg_bytes) => {
                let b = sphinx_key_signer::parse_ping_and_form_response(msg_bytes);
                if do_log {
                    log::info!("GOT A PING MESSAGE! returning pong now...");
                }
                mqtt.publish(RETURN_TOPIC, QOS, false, b).expect("could not publish init response");
            },
            Event::Disconnected => {
                log::info!("GOT A Event::Disconnected msg!");
            }
        }
    }

    Ok(())
}
