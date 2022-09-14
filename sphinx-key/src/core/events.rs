use crate::conn::mqtt::QOS;
use crate::core::control::{controller_from_seed, FlashPersister};

use sphinx_key_signer::control::{Config, ControlMessage, ControlResponse, Policy};
use sphinx_key_signer::lightning_signer::bitcoin::Network;
use sphinx_key_signer::vls_protocol::model::PubKey;
use sphinx_key_signer::{self, make_init_msg, topics, InitResponse, RootHandler};
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
    seed: [u8; 32],
    policy: &Policy,
    flash: Arc<Mutex<FlashPersister>>,
) -> Result<()> {
    while let Ok(event) = rx.recv() {
        log::info!("BROKER IP AND PORT: {}", config.broker);
        // wait for a Connection first.
        match event {
            Event::Connected => {
                log::info!("SUBSCRIBE to {}", topics::VLS);
                mqtt.subscribe(topics::VLS, QOS)
                    .expect("could not MQTT subscribe");
                mqtt.subscribe(topics::CONTROL, QOS)
                    .expect("could not MQTT subscribe");
                led_tx.send(Status::Connected).unwrap();
                break;
            }
            _ => (),
        }
    }

    // initialize the RootHandler
    let init_msg = make_init_msg(network, seed).expect("failed to make init msg");
    let InitResponse {
        root_handler,
        init_reply: _,
    } = sphinx_key_signer::init(init_msg, network, policy).expect("failed to init signer");

    // make the controller to validate Control messages
    let mut ctrlr = controller_from_seed(&network, &seed[..], flash);

    // signing loop
    let dummy_peer = PubKey([0; 33]);
    while let Ok(event) = rx.recv() {
        match event {
            Event::Connected => {
                log::info!("SUBSCRIBE TO {}", topics::VLS);
                mqtt.subscribe(topics::VLS, QOS)
                    .expect("could not MQTT subscribe");
                mqtt.subscribe(topics::CONTROL, QOS)
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
                        mqtt.publish(topics::VLS_RETURN, QOS, false, &b)
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
                let cres = ctrlr.handle(msg_bytes);
                if let Some(res_data) = handle_control_response(&root_handler, cres, network) {
                    mqtt.publish(topics::CONTROL_RETURN, QOS, false, &res_data)
                        .expect("could not publish control response");
                }
            }
        }
    }

    Ok(())
}

fn handle_control_response(
    root_handler: &RootHandler,
    cres: anyhow::Result<(Vec<u8>, ControlMessage)>,
    network: Network,
) -> Option<Vec<u8>> {
    match cres {
        Ok((mut response, parsed_msg)) => {
            // the following msg types require other actions besides Flash persistence
            match parsed_msg {
                ControlMessage::UpdatePolicy(new_policy) => {
                    if let Err(e) =
                        sphinx_key_signer::set_policy(&root_handler, network, new_policy)
                    {
                        log::error!("set policy failed {:?}", e);
                    }
                }
                ControlMessage::UpdateAllowlist(al) => {
                    if let Err(e) = sphinx_key_signer::set_allowlist(&root_handler, &al) {
                        log::error!("set allowlist failed {:?}", e);
                    }
                }
                // overwrite the real Allowlist response, loaded from Node
                ControlMessage::QueryAllowlist => {
                    if let Ok(al) = sphinx_key_signer::get_allowlist(&root_handler) {
                        response = rmp_serde::to_vec(&ControlResponse::AllowlistCurrent(al))
                            .expect("couldnt build ControlResponse::AllowlistCurrent");
                    } else {
                        log::error!("read allowlist failed");
                    }
                }
                _ => (),
            };
            Some(response)
        }
        Err(e) => {
            let response = rmp_serde::to_vec(&ControlResponse::Error(e.to_string()))
                .expect("couldnt build ControlResponse::Error");
            log::warn!("error parsing ctrl msg {:?}", e);
            Some(response)
        }
    }
}

#[cfg(feature = "pingpong")]
pub fn make_event_loop(
    mut mqtt: EspMqttClient<ConnState<MessageImpl, EspError>>,
    rx: mpsc::Receiver<Event>,
    _network: Network,
    do_log: bool,
    led_tx: mpsc::Sender<Status>,
    _config: Config,
    _seed: [u8; 32],
    _policy: &Policy,
    _flash: Arc<Mutex<FlashPersister>>,
) -> Result<()> {
    log::info!("About to subscribe to the mpsc channel");
    while let Ok(event) = rx.recv() {
        match event {
            Event::Connected => {
                led_tx.send(Status::ConnectedToMqtt).unwrap();
                log::info!("SUBSCRIBE TO {}", topics::VLS);
                mqtt.subscribe(topics::VLS, QOS)
                    .expect("could not MQTT subscribe");
            }
            Event::VlsMessage(msg_bytes) => {
                led_tx.send(Status::Signing).unwrap();
                let b = sphinx_key_signer::parse_ping_and_form_response(msg_bytes);
                if do_log {
                    log::info!("GOT A PING MESSAGE! returning pong now...");
                }
                mqtt.publish(topics::VLS_RETURN, QOS, false, b)
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
