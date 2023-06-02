use crate::conn::mqtt::QOS;
use crate::ota::{update_sphinx_key, validate_ota_message};

use sphinx_signer::lightning_signer::bitcoin::Network;
use sphinx_signer::lightning_signer::persist::Persist;
use sphinx_signer::persist::FsPersister;
use sphinx_signer::sphinx_glyph::control::{
    Config, ControlMessage, ControlResponse, Controller, Policy,
};
use sphinx_signer::sphinx_glyph::error::Error as GlyphError;
use sphinx_signer::sphinx_glyph::topics;
use sphinx_signer::{self, RootHandler};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use embedded_svc::httpd::Result;
// use embedded_svc::mqtt::client::Client;
use embedded_svc::mqtt::client::MessageImpl;
use embedded_svc::utils::mqtt::client::ConnState;
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
    Ota,
    Reset1,
    Reset2,
    Reset3,
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
    client_id: &str,
) -> Result<()> {
    while let Ok(event) = rx.recv() {
        log::info!("BROKER IP AND PORT: {}", config.broker);
        // wait for a Connection first.
        match event {
            Event::Connected => {
                let vls_topic = format!("{}/{}", client_id, topics::VLS);
                log::info!("SUBSCRIBE to {}", vls_topic);
                mqtt.subscribe(&vls_topic, QOS)
                    .expect("could not MQTT subscribe");
                let control_topic = format!("{}/{}", client_id, topics::CONTROL);
                mqtt.subscribe(&control_topic, QOS)
                    .expect("could not MQTT subscribe");
                led_tx.send(Status::Connected).unwrap();
                break;
            }
            _ => (),
        }
    }

    // create the fs persister
    // 8 character max file names
    let persister: Arc<dyn Persist> = Arc::new(FsPersister::new(&ROOT_STORE, Some(8)));

    // initialize the RootHandler
    let handler_builder =
        sphinx_signer::root::builder(seed, network, policy, persister).expect("failed to init signer");
    let (root_handler, _) = handler_builder.build();

    // signing loop
    log::info!("=> starting the main signing loop...");
    while let Ok(event) = rx.recv() {
        match event {
            Event::Connected => {
                let vls_topic = format!("{}/{}", client_id, topics::VLS);
                mqtt.subscribe(&vls_topic, QOS)
                    .expect("could not MQTT subscribe");
                log::info!("SUBSCRIBE TO {}", vls_topic);
                let control_topic = format!("{}/{}", client_id, topics::CONTROL);
                mqtt.subscribe(&control_topic, QOS)
                    .expect("could not MQTT subscribe");
                led_tx.send(Status::Connected).unwrap();
            }
            Event::Disconnected => {
                led_tx.send(Status::ConnectingToMqtt).unwrap();
                log::info!("GOT A Event::Disconnected msg!");
            }
            Event::VlsMessage(ref msg_bytes) => {
                led_tx.send(Status::Signing).unwrap();
                let _ret =
                    match sphinx_signer::root::handle(&root_handler, msg_bytes.clone(), do_log) {
                        Ok(b) => {
                            let vls_return_topic = format!("{}/{}", client_id, topics::VLS_RETURN);
                            mqtt.publish(&vls_return_topic, QOS, false, &b)
                                .expect("could not publish VLS response");
                        }
                        Err(e) => {
                            let err_msg = GlyphError::new(1, &e.to_string());
                            log::error!("HANDLE FAILED {:?}", e);
                            let error_topic = format!("{}/{}", client_id, topics::ERROR);
                            mqtt.publish(&error_topic, QOS, false, &err_msg.to_vec()[..])
                                .expect("could not publish VLS error");
                            // panic!("HANDLE FAILED {:?}", e);
                        }
                    };
            }
            Event::Control(ref msg_bytes) => {
                log::info!("GOT A CONTROL MSG");
                let cres = ctrlr.handle(msg_bytes);
                if let Some(res) =
                    handle_control_response(&root_handler, cres, network, led_tx.clone())
                {
                    let res_data =
                        rmp_serde::to_vec_named(&res).expect("could not publish control response");
                    let control_return_topic = format!("{}/{}", client_id, topics::CONTROL_RETURN);
                    mqtt.publish(&control_return_topic, QOS, false, &res_data)
                        .expect("could not publish control response");
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
    led_tx: mpsc::Sender<Status>,
) -> Option<ControlResponse> {
    match cres {
        Ok((control_msg, mut control_res)) => {
            // the following msg types require other actions besides Flash persistence
            match control_msg {
                ControlMessage::UpdatePolicy(new_policy) => {
                    if let Err(e) =
                        sphinx_signer::policy::set_policy(&root_handler, network, new_policy)
                    {
                        log::error!("set policy failed {:?}", e);
                        control_res = ControlResponse::Error(format!("set policy failed {:?}", e))
                    }
                }
                ControlMessage::UpdateAllowlist(al) => {
                    if let Err(e) = sphinx_signer::policy::set_allowlist(&root_handler, &al) {
                        log::error!("set allowlist failed {:?}", e);
                        control_res =
                            ControlResponse::Error(format!("set allowlist failed {:?}", e))
                    }
                }
                // overwrite the real Allowlist response, loaded from Node
                ControlMessage::QueryAllowlist => {
                    match sphinx_signer::policy::get_allowlist(&root_handler) {
                        Ok(al) => control_res = ControlResponse::AllowlistCurrent(al),
                        Err(e) => {
                            log::error!("read allowlist failed {:?}", e);
                            control_res =
                                ControlResponse::Error(format!("read allowlist failed {:?}", e))
                        }
                    }
                }
                ControlMessage::Ota(params) => {
                    if let Err(e) = validate_ota_message(params.clone()) {
                        log::error!("OTA update cannot launch {:?}", e.to_string());
                        control_res =
                            ControlResponse::Error(format!("OTA update cannot launch {:?}", e))
                    } else {
                        thread::spawn(move || {
                            led_tx.send(Status::Ota).unwrap();
                            if let Err(e) = update_sphinx_key(params, led_tx) {
                                log::error!("OTA update failed {:?}", e.to_string());
                            } else {
                                log::info!("OTA flow complete, restarting esp...");
                                unsafe { esp_idf_sys::esp_restart() };
                            }
                        });
                        log::info!("OTA update launched...");
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
    client_id: &str,
) -> Result<()> {
    log::info!("About to subscribe to the mpsc channel");
    while let Ok(event) = rx.recv() {
        match event {
            Event::Connected => {
                led_tx.send(Status::ConnectedToMqtt).unwrap();
                let vls_topic = format!("{}/{}", client_id, topics::VLS);
                log::info!("SUBSCRIBE TO {}", vls_topic);
                mqtt.subscribe(&vls_topic, QOS)
                    .expect("could not MQTT subscribe");
            }
            Event::VlsMessage(msg_bytes) => {
                led_tx.send(Status::Signing).unwrap();
                let b = sphinx_signer::parse_ping_and_form_response(msg_bytes);
                if do_log {
                    log::info!("GOT A PING MESSAGE! returning pong now...");
                }
                let vls_return_topic = format!("{}/{}", client_id, topics::VLS_RETURN);
                mqtt.publish(&vls_return_topic, QOS, false, b)
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
