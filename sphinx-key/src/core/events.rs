use crate::conn::mqtt::{QOS, RETURN_TOPIC, TOPIC};
use sphinx_key_signer::vls_protocol::model::PubKey;
use sphinx_key_signer::{self, InitResponse};
use sphinx_key_signer::lightning_signer::bitcoin::Network;
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

#[derive(Debug)]
pub enum Status {
    WifiAccessPoint,
    WifiAccessPointClientConnected,
    ConnectingToWifi,
    ConnectingToMqtt,
    ConnectedToMqtt,
    MessageReceived,
}

// the main event loop
#[cfg(not(feature = "pingpong"))]
pub fn make_event_loop(
    mut mqtt: EspMqttClient<ConnState<MessageImpl, EspError>>,
    rx: mpsc::Receiver<Event>,
    network: Network,
    do_log: bool,
    led_tx: mpsc::Sender<Status>
) -> Result<()> {
    // initialize the RootHandler
    let root_handler = loop {
        if let Ok(event) = rx.recv() {
            match event {
                Event::Connected => {
                    log::info!("SUBSCRIBE to {}", TOPIC);
                    mqtt.subscribe(TOPIC, QOS)
                        .expect("could not MQTT subscribe");
                    led_tx.send(Status::ConnectedToMqtt).unwrap();
                }
                Event::Message(ref msg_bytes) => {
                    let InitResponse {
                        root_handler,
                        init_reply,
                    } = sphinx_key_signer::init(msg_bytes.clone(), network).expect("failed to init signer");
                    mqtt.publish(RETURN_TOPIC, QOS, false, init_reply)
                        .expect("could not publish init response");
                    break root_handler;
                }
                Event::Disconnected => {
                    led_tx.send(Status::ConnectingToMqtt).unwrap();
                    log::info!("GOT an early Event::Disconnected msg!");
                }
            }
        }
    };

    // signing loop
    let dummy_peer = PubKey([0; 33]);
    while let Ok(event) = rx.recv() {
        match event {
            Event::Connected => {
                log::info!("SUBSCRIBE TO {}", TOPIC);
                mqtt.subscribe(TOPIC, QOS)
                    .expect("could not MQTT subscribe");
                led_tx.send(Status::ConnectedToMqtt).unwrap();
            }
            Event::Message(ref msg_bytes) => {
                led_tx.send(Status::MessageReceived).unwrap();
                let _ret = match sphinx_key_signer::handle(
                    &root_handler,
                    msg_bytes.clone(),
                    dummy_peer.clone(),
                    do_log,
                ) {
                    Ok(b) => mqtt
                        .publish(RETURN_TOPIC, QOS, false, b)
                        .expect("could not publish init response"),
                    Err(e) => panic!("HANDLE FAILED {:?}", e),
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
    led_tx: mpsc::Sender<Status>
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
                led_tx.send(Status::MessageReceived).unwrap();
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
