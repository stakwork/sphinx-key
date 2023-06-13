use crate::conn::mqtt::QOS;
use crate::core::lss;
use crate::ota::{update_sphinx_key, validate_ota_message};
use crate::status::Status;

use lss_connector::secp256k1::PublicKey;
use sphinx_signer::lightning_signer::bitcoin::Network;
use sphinx_signer::lightning_signer::persist::Persist;
use sphinx_signer::persist::{BackupPersister, FsPersister, ThreadMemoPersister};
use sphinx_signer::sphinx_glyph::control::{
    Config, ControlMessage, ControlResponse, Controller, Policy, Velocity,
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
    LssMessage(Vec<u8>),
    Control(Vec<u8>),
}

pub const ROOT_STORE: &str = "/sdcard/store";

pub const SUB_TOPICS: &[&str] = &[topics::VLS, topics::LSS_MSG, topics::CONTROL];

fn mqtt_sub(
    mqtt: &mut EspMqttClient<ConnState<MessageImpl, EspError>>,
    client_id: &str,
    topics: &[&str],
) {
    for top in topics {
        let topic = format!("{}/{}", client_id, top);
        log::info!("SUBSCRIBE to {}", topic);
        mqtt.subscribe(&topic, QOS)
            .expect("could not MQTT subscribe");
    }
}

fn mqtt_pub(
    mqtt: &mut EspMqttClient<ConnState<MessageImpl, EspError>>,
    client_id: &str,
    top: &str,
    payload: &[u8],
) {
    let topic = format!("{}/{}", client_id, top);
    mqtt.publish(&topic, QOS, false, payload)
        .expect("could not MQTT publish");
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
    velocity: &Option<Velocity>,
    mut ctrlr: Controller,
    client_id: &str,
    node_id: &PublicKey,
) -> Result<()> {
    while let Ok(event) = rx.recv() {
        log::info!("BROKER IP AND PORT: {}", config.broker);
        // wait for a Connection first.
        match event {
            Event::Connected => {
                mqtt_sub(&mut mqtt, client_id, SUB_TOPICS);
                break;
            }
            _ => (),
        }
    }

    // create the fs persister
    // 8 character max file names
    // let persister: Arc<dyn Persist> = Arc::new(FsPersister::new(&ROOT_STORE, Some(8)));
    // let persister = Arc::new(ThreadMemoPersister {});

    let sd_persister = FsPersister::new(&ROOT_STORE, Some(8));
    let initial_allowlist = match sd_persister.get_node_allowlist(node_id) {
        Ok(al) => al,
        Err(_) => {
            log::warn!("no allowlist found in fs persister!");
            Vec::new()
        }
    };

    let lss_persister = ThreadMemoPersister {};
    let persister = Arc::new(BackupPersister::new(sd_persister, lss_persister));

    // initialize the RootHandler
    let (rhb, approver) = sphinx_signer::root::builder(
        seed,
        network,
        policy.clone(),
        velocity.clone(),
        initial_allowlist,
        persister,
    )
    .expect("failed to init signer");

    // FIXME it right to restart here?
    let (root_handler, lss_signer) = match lss::init_lss(client_id, &rx, rhb, &mut mqtt) {
        Ok(rl) => rl,
        Err(e) => {
            log::error!("failed to init lss {:?}", e);
            unsafe { esp_idf_sys::esp_restart() };
        }
    };

    // store the previous msgs processed, for LSS last step
    let mut msgs: Option<(Vec<u8>, Vec<u8>)> = None;

    // signing loop
    log::info!("=> starting the main signing loop...");
    let flash_db = ctrlr.persister();
    while let Ok(event) = rx.recv() {
        match event {
            Event::Connected => {
                mqtt_sub(&mut mqtt, client_id, SUB_TOPICS);
                led_tx.send(Status::Connected).unwrap();
            }
            Event::Disconnected => {
                led_tx.send(Status::ConnectingToMqtt).unwrap();
                log::info!("GOT A Event::Disconnected msg!");
            }
            Event::VlsMessage(ref msg_bytes) => {
                led_tx.send(Status::Signing).unwrap();
                let state1 = approver.control().get_state();
                let _ret = match sphinx_signer::root::handle_with_lss(
                    &root_handler,
                    &lss_signer,
                    msg_bytes.clone(),
                    do_log,
                ) {
                    Ok((vls_b, lss_b)) => {
                        if lss_b.len() == 0 {
                            // no muts, respond directly back!
                            mqtt_pub(&mut mqtt, client_id, topics::VLS_RETURN, &vls_b);
                        } else {
                            // muts! send LSS first!
                            msgs = Some((vls_b, lss_b.clone()));
                            mqtt_pub(&mut mqtt, client_id, topics::LSS_RES, &lss_b);
                        }
                    }
                    Err(e) => {
                        let err_msg = GlyphError::new(1, &e.to_string());
                        log::error!("HANDLE FAILED {:?}", e);
                        mqtt_pub(&mut mqtt, client_id, topics::ERROR, &err_msg.to_vec()[..]);
                    }
                };
                let state2 = approver.control().get_state();
                if state1 != state2 {
                    // save the velocity state in case of crash or restart
                    let mut flash_db = flash_db.lock().unwrap();
                    if let Err(e) = flash_db.write_velocity(state2) {
                        log::error!("failed to set velocity state {:?}", e);
                    }
                    drop(flash_db);
                }
            }
            Event::LssMessage(ref msg_bytes) => {
                match lss::handle_lss_msg(msg_bytes, &msgs, &lss_signer) {
                    Ok((ret_topic, bytes)) => {
                        // set msgs back to None
                        msgs = None;
                        mqtt_pub(&mut mqtt, client_id, &ret_topic, &bytes);
                    }
                    Err(e) => {
                        let err_msg = GlyphError::new(1, &e.to_string());
                        mqtt_pub(&mut mqtt, client_id, topics::ERROR, &err_msg.to_vec()[..]);
                    }
                }
            }
            Event::Control(ref msg_bytes) => {
                log::info!("GOT A CONTROL MSG");
                let cres = ctrlr.handle(msg_bytes);
                if let Some(res) =
                    handle_control_response(&root_handler, cres, network, led_tx.clone())
                {
                    let res_data =
                        rmp_serde::to_vec_named(&res).expect("could not publish control response");
                    mqtt_pub(&mut mqtt, client_id, topics::CONTROL_RETURN, &res_data);
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
    _velocity: &Option<Velocity>,
    mut _ctrlr: Controller,
    client_id: &str,
    _node_id: &PublicKey,
) -> Result<()> {
    log::info!("About to subscribe to the mpsc channel");
    while let Ok(event) = rx.recv() {
        match event {
            Event::Connected => {
                led_tx.send(Status::ConnectedToMqtt).unwrap();
                mqtt_sub(&mut mqtt, client_id, &[topics::VLS]);
            }
            Event::VlsMessage(msg_bytes) => {
                led_tx.send(Status::Signing).unwrap();
                let b = sphinx_signer::parse_ping_and_form_response(msg_bytes);
                if do_log {
                    log::info!("GOT A PING MESSAGE! returning pong now...");
                }
                mqtt_pub(&mut mqtt, client_id, topics::VLS_RETURN, &b);
            }
            Event::LssMessage(_) => (),
            Event::Disconnected => {
                led_tx.send(Status::ConnectingToMqtt).unwrap();
                log::info!("GOT A Event::Disconnected msg!");
            }
            Event::Control(_) => (),
        }
    }

    Ok(())
}
