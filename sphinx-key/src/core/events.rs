use crate::conn::mqtt::{QOS, RETURN_TOPIC, TOPIC};
use crate::core::init::make_init_msg;
use crate::core::config::Config;

use sphinx_key_signer::lightning_signer::bitcoin::Network;
use sphinx_key_signer::vls_protocol::model::PubKey;
use sphinx_key_signer::{self, InitResponse};
use std::sync::mpsc;

use embedded_svc::httpd::Result;
use embedded_svc::mqtt::client::utils::ConnState;
use embedded_svc::mqtt::client::Client;
use embedded_svc::mqtt::client::{MessageImpl, Publish};
use esp_idf_svc::mqtt::client::*;
use esp_idf_sys;
use esp_idf_sys::EspError;

pub enum Event {
    Connected,
    Disconnected,
    Message(Vec<u8>),
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Status {
    Starting,
    MountingSDCard,
    WifiAccessPoint,
    Configuring,
    ConnectingToWifi,
    ConnectingToMqtt,
    Connected,
    Signing,
}

// the main event loop
#[cfg(not(feature = "pingpong"))]
pub fn make_event_loop(
    mut mqtt: EspMqttClient<ConnState<MessageImpl, EspError>>,
    rx: mpsc::Receiver<Event>,
    network: Network,
    do_log: bool,
    led_tx: mpsc::Sender<Status>,
    config: Config
) -> Result<()> {
    while let Ok(event) = rx.recv() {
        log::info!("BROKER IP AND PORT: {}", config.broker);
        match event {
            Event::Connected => {
                log::info!("SUBSCRIBE to {}", TOPIC);
                mqtt.subscribe(TOPIC, QOS)
                    .expect("could not MQTT subscribe");
                led_tx.send(Status::Connected).unwrap();
                break;
            }
            Event::Message(ref _msg_bytes) => {
                panic!("should not be a message before connection");
            }
            Event::Disconnected => {
                led_tx.send(Status::ConnectingToMqtt).unwrap();
                log::info!("GOT an early Event::Disconnected msg!");
            }
        }
    }

    // initialize the RootHandler
    let init_msg = make_init_msg(network, config.seed).expect("failed to make init msg");
    let InitResponse {
        root_handler,
        init_reply: _,
    } = sphinx_key_signer::init(init_msg, network).expect("failed to init signer");
    // signing loop
    let dummy_peer = PubKey([0; 33]);
    while let Ok(event) = rx.recv() {
        match event {
            Event::Connected => {
                log::info!("SUBSCRIBE TO {}", TOPIC);
                mqtt.subscribe(TOPIC, QOS)
                    .expect("could not MQTT subscribe");
                led_tx.send(Status::Connected).unwrap();
            }
            Event::Message(ref msg_bytes) => {
                led_tx.send(Status::Signing).unwrap();
                let _ret = match sphinx_key_signer::handle(
                    &root_handler,
                    msg_bytes.clone(),
                    dummy_peer.clone(),
                    do_log,
                ) {
                    Ok(b) => {
                        mqtt.publish(RETURN_TOPIC, QOS, false, &b)
                            .expect("could not publish init response");
                    }
                    Err(e) => {
                        log::error!("HANDLE FAILED {:?}", e);
                        // panic!("HANDLE FAILED {:?}", e);
                    }
                };
            }
            Event::Disconnected => {
                led_tx.send(Status::ConnectingToMqtt).unwrap();
                log::info!("GOT A Event::Disconnected msg!");
            }
        }
    }

    Ok(())
}

#[cfg(feature = "pingpong")]
pub fn make_event_loop(
    mut mqtt: EspMqttClient<ConnState<MessageImpl, EspError>>,
    rx: mpsc::Receiver<Event>,
    _network: Network,
    do_log: bool,
    led_tx: mpsc::Sender<Status>,
    _seed: [u8; 32],
) -> Result<()> {
    log::info!("About to subscribe to the mpsc channel");
    while let Ok(event) = rx.recv() {
        match event {
            Event::Connected => {
                led_tx.send(Status::ConnectedToMqtt).unwrap();
                log::info!("SUBSCRIBE TO {}", TOPIC);
                mqtt.subscribe(TOPIC, QOS)
                    .expect("could not MQTT subscribe");
            }
            Event::Message(msg_bytes) => {
                led_tx.send(Status::Signing).unwrap();
                let b = sphinx_key_signer::parse_ping_and_form_response(msg_bytes);
                if do_log {
                    log::info!("GOT A PING MESSAGE! returning pong now...");
                }
                mqtt.publish(RETURN_TOPIC, QOS, false, b)
                    .expect("could not publish ping response");
            }
            Event::Disconnected => {
                led_tx.send(Status::ConnectingToMqtt).unwrap();
                log::info!("GOT A Event::Disconnected msg!");
            }
        }
    }

    Ok(())
}
