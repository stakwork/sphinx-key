use crate::conn::mqtt::{CONTROL_RETURN_TOPIC, CONTROL_TOPIC, QOS, RETURN_TOPIC, VLS_TOPIC};
use crate::core::config::Config;
use crate::core::control::{controller_from_seed, FlashPersister};
use crate::core::init::make_init_msg;

use sphinx_key_signer::lightning_signer::bitcoin::Network;
use sphinx_key_signer::vls_protocol::model::PubKey;
use sphinx_key_signer::{self, InitResponse};
use std::sync::{mpsc, Arc, Mutex};

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
    VlsMessage(Vec<u8>),
    Control(Vec<u8>),
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Status {
    Starting,
    MountingSDCard,
    SyncingTime,
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
    config: Config,
    flash: Arc<Mutex<FlashPersister>>,
) -> Result<()> {
    while let Ok(event) = rx.recv() {
        log::info!("BROKER IP AND PORT: {}", config.broker);
        // wait for a Connection first.
        match event {
            Event::Connected => {
                log::info!("SUBSCRIBE to {}", VLS_TOPIC);
                mqtt.subscribe(VLS_TOPIC, QOS)
                    .expect("could not MQTT subscribe");
                mqtt.subscribe(CONTROL_TOPIC, QOS)
                    .expect("could not MQTT subscribe");
                led_tx.send(Status::Connected).unwrap();
                break;
            }
            _ => (),
        }
    }

    // initialize the RootHandler
    let init_msg = make_init_msg(network, config.seed).expect("failed to make init msg");
    let InitResponse {
        root_handler,
        init_reply: _,
    } = sphinx_key_signer::init(init_msg, network).expect("failed to init signer");

    // make the controller to validate Control messages
    let mut ctrlr = controller_from_seed(&network, &config.seed[..], flash);

    // signing loop
    let dummy_peer = PubKey([0; 33]);
    while let Ok(event) = rx.recv() {
        match event {
            Event::Connected => {
                log::info!("SUBSCRIBE TO {}", VLS_TOPIC);
                mqtt.subscribe(VLS_TOPIC, QOS)
                    .expect("could not MQTT subscribe");
                mqtt.subscribe(CONTROL_TOPIC, QOS)
                    .expect("could not MQTT subscribe");
                led_tx.send(Status::Connected).unwrap();
            }
            Event::Disconnected => {
                led_tx.send(Status::ConnectingToMqtt).unwrap();
                log::info!("GOT A Event::Disconnected msg!");
            }
            Event::VlsMessage(ref msg_bytes) => {
                led_tx.send(Status::Signing).unwrap();
                let _ret = match sphinx_key_signer::handle(
                    &root_handler,
                    msg_bytes.clone(),
                    dummy_peer.clone(),
                    do_log,
                ) {
                    Ok(b) => {
                        mqtt.publish(RETURN_TOPIC, QOS, false, &b)
                            .expect("could not publish VLS response");
                    }
                    Err(e) => {
                        log::error!("HANDLE FAILED {:?}", e);
                        // panic!("HANDLE FAILED {:?}", e);
                    }
                };
            }
            Event::Control(ref msg_bytes) => {
                log::info!("GOT A CONTROL MSG");
                match ctrlr.handle(msg_bytes) {
                    Ok(response) => {
                        // log::info!("CONTROL MSG {:?}", response);
                        mqtt.publish(CONTROL_RETURN_TOPIC, QOS, false, &response)
                            .expect("could not publish control response");
                    }
                    Err(e) => log::warn!("error parsing ctrl msg {:?}", e),
                };
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
            Event::VlsMessage(msg_bytes) => {
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
            Event::Control(_) => (),
        }
    }

    Ok(())
}
