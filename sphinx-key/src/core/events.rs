use crate::conn::mqtt::QOS;
use crate::ota::update_sphinx_key;

use sphinx_key_signer::control::{Config, ControlMessage, ControlResponse, Controller, Policy};
use sphinx_key_signer::lightning_signer::bitcoin::Network;
use sphinx_key_signer::vls_protocol::model::PubKey;
use sphinx_key_signer::{self, make_init_msg, topics, InitResponse, ParserError, RootHandler};
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

pub const ROOT_STORE: &str = "/sdcard/store";

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
    mut ctrlr: Controller,
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
    } = sphinx_key_signer::init(init_msg, network, policy, ROOT_STORE)
        .expect("failed to init signer");

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
                        let err_msg = ParserError::new(1, &e.to_string());
                        log::error!("HANDLE FAILED {:?}", e);
                        mqtt.publish(topics::ERROR, QOS, false, &err_msg.to_vec()[..])
                            .expect("could not publish VLS error");
                        // panic!("HANDLE FAILED {:?}", e);
                    }
                };
            }
            Event::Control(ref msg_bytes) => {
                log::info!("GOT A CONTROL MSG");
                let cres = ctrlr.handle(msg_bytes);
                if let Some(res) = handle_control_response(&root_handler, cres, network) {
                    let res_data =
                        rmp_serde::to_vec(&res).expect("could not publish control response");
                    mqtt.publish(topics::CONTROL_RETURN, QOS, false, &res_data)
                        .expect("could not publish control response");
                    if let ControlResponse::OtaConfirm(_) = res {
                        unsafe { esp_idf_sys::esp_restart() };
                    }
                }
            }
        }
    }

    Ok(())
}

fn handle_control_response(
    root_handler: &RootHandler,
    cres: anyhow::Result<(ControlMessage, ControlResponse)>,
    network: Network,
) -> Option<ControlResponse> {
    match cres {
        Ok((control_msg, mut control_res)) => {
            // the following msg types require other actions besides Flash persistence
            match control_msg {
                ControlMessage::UpdatePolicy(new_policy) => {
                    if let Err(e) =
                        sphinx_key_signer::set_policy(&root_handler, network, new_policy)
                    {
                        log::error!("set policy failed {:?}", e);
                        control_res = ControlResponse::Error(format!("set policy failed {:?}", e))
                    }
                }
                ControlMessage::UpdateAllowlist(al) => {
                    if let Err(e) = sphinx_key_signer::set_allowlist(&root_handler, &al) {
                        log::error!("set allowlist failed {:?}", e);
                        control_res =
                            ControlResponse::Error(format!("set allowlist failed {:?}", e))
                    }
                }
                // overwrite the real Allowlist response, loaded from Node
                ControlMessage::QueryAllowlist => {
                    match sphinx_key_signer::get_allowlist(&root_handler) {
                        Ok(al) => control_res = ControlResponse::AllowlistCurrent(al),
                        Err(e) => {
                            log::error!("read allowlist failed {:?}", e);
                            control_res =
                                ControlResponse::Error(format!("read allowlist failed {:?}", e))
                        }
                    }
                }
                ControlMessage::Ota(params) => {
                    if let Err(e) = update_sphinx_key(params.version, params.url.clone()) {
                        log::error!("OTA update failed {:?}", e.to_string());
                        control_res =
                            ControlResponse::Error(format!("OTA update failed {:?}", e))
                    } else {
                        log::info!("OTA update completed, about to restart the glyph...");
                    }
                }
                _ => (),
            };
            Some(control_res)
        }
        Err(e) => {
            let control_res = ControlResponse::Error(e.to_string());
            log::warn!("error parsing ctrl msg {:?}", e);
            Some(control_res)
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
    mut _ctrlr: Controller,
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
