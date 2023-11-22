use crate::conn::mqtt::QOS;
use crate::core::lss;
use crate::ota::{update_sphinx_key, validate_ota_message};
use crate::status::Status;

use glyph::control::{Config, ControlMessage, ControlResponse, Controller, Policy, Velocity};
use glyph::error::Error as GlyphError;
use glyph::ser::{serialize_controlresponse, ByteBuf};
use glyph::topics;
use lss_connector::secp256k1::PublicKey;
use sphinx_signer::approver::SphinxApprover;
use sphinx_signer::lightning_signer::bitcoin::Network;
//use sphinx_signer::lightning_signer::persist::DummyPersister;
use sphinx_signer::kvv::{CloudKVVStore, FsKVVStore};
use sphinx_signer::lightning_signer::persist::Persist;
use sphinx_signer::root::VlsHandlerError;
use sphinx_signer::sphinx_glyph as glyph;
use sphinx_signer::{self, Handler, RootHandler};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use esp_idf_svc::mqtt::client::*;
use esp_idf_svc::sys::EspError;

#[derive(Debug)]
pub enum Event {
    Connected,
    Disconnected,
    VlsMessage(Vec<u8>),
    LssMessage(Vec<u8>),
    Control(Vec<u8>),
}

pub const ROOT_STORE: &str = "/sdcard/store";

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
#[allow(clippy::too_many_arguments)]
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
    signer_id: &[u8; 16],
    node_id: &PublicKey,
) {
    let client_id = hex::encode(signer_id);

    while let Ok(event) = rx.recv() {
        log::info!("BROKER IP AND PORT: {}", config.broker);
        // wait for a Connection first.
        if let Event::Connected = event {
            mqtt_sub(&mut mqtt, &client_id, topics::SIGNER_SUBS);
            break;
        }
    }

    let kvv_store = FsKVVStore::new(ROOT_STORE, *signer_id, None).0;
    let fs_persister = CloudKVVStore::new(kvv_store);

    let _ = fs_persister.enter();
    let initial_allowlist = match fs_persister.get_nodes() {
        Ok(ns) => {
            if !ns.is_empty() {
                match fs_persister.get_node_allowlist(node_id) {
                    Ok(al) => al,
                    Err(_) => {
                        log::warn!("no allowlist found in fs persister!");
                        Vec::new()
                    }
                }
            } else {
                Vec::new()
            }
        }
        Err(_) => Vec::new(),
    };
    let _ = fs_persister.prepare();
    let _ = fs_persister.commit();

    let persister = Arc::new(fs_persister);

    // initialize the RootHandler
    let (rhb, approver) = sphinx_signer::root::builder(
        seed,
        network,
        policy.clone(),
        initial_allowlist,
        velocity.clone(),
        persister,
    )
    .expect("failed to init signer");

    thread::sleep(std::time::Duration::from_secs(1));
    // send the initial HELLO
    mqtt_pub(&mut mqtt, &client_id, topics::HELLO, &[]);

    let (root_handler, lss_signer) = match lss::init_lss(signer_id, &rx, rhb, &mut mqtt) {
        Ok(rl) => rl,
        Err(e) => {
            log::error!("failed to init lss {:?}", e);
            unsafe { esp_idf_svc::sys::esp_restart() };
        }
    };

    // store the previous msgs processed, for LSS last step
    let mut msgs: Option<(Vec<u8>, [u8; 32])> = None;

    // signing loop
    log::info!("=> starting the main signing loop...");
    let flash_db = ctrlr.persister();
    let mut expected_sequence = None;
    let mut current_status = Status::ConnectingToMqtt;
    while let Ok(event) = rx.recv() {
        log::info!("new event loop!");
        check_memory();
        match event {
            Event::Connected => {
                log::info!("GOT A Event::Connected msg!");
                mqtt_sub(&mut mqtt, &client_id, topics::SIGNER_SUBS);
                thread::sleep(std::time::Duration::from_secs(1));
                // send the initial HELLO again
                mqtt_pub(&mut mqtt, &client_id, topics::HELLO, &[]);
                current_status = update_led(current_status, Status::Connected, &led_tx);
            }
            Event::Disconnected => {
                current_status = update_led(current_status, Status::ConnectingToMqtt, &led_tx);
                log::info!("GOT A Event::Disconnected msg!");
            }
            Event::VlsMessage(msg_bytes) => {
                current_status = update_led(current_status, Status::Signing, &led_tx);
                let state1 = approver.control().get_state();
                //log::info!("FULL MSG {:?}", &msg_bytes);
                match sphinx_signer::root::handle_with_lss(
                    &root_handler,
                    &lss_signer,
                    msg_bytes,
                    expected_sequence,
                    do_log,
                ) {
                    Ok((vls_b, lss_b, sequence, _cmd, server_hmac_opt)) => {
                        if let Some(server_hmac) = server_hmac_opt {
                            // muts! send LSS first!
                            mqtt_pub(&mut mqtt, &client_id, topics::LSS_RES, &lss_b);
                            msgs = Some((vls_b, server_hmac));
                        } else {
                            // no muts, respond directly back!
                            mqtt_pub(&mut mqtt, &client_id, topics::VLS_RES, &vls_b);
                            // and commit
                            if let Err(e) = root_handler.node().get_persister().commit() {
                                log::error!("LOCAL COMMIT ERROR! {:?}", e);
                                unsafe { esp_idf_svc::sys::esp_restart() };
                            }
                            restart_esp_if_memory_low();
                        }
                        expected_sequence = Some(sequence + 1);
                    }
                    Err(e) => match e {
                        VlsHandlerError::BadSequence(current, expected) => {
                            log::info!(
                                "caught a badsequence error, current: {}, expected: {}",
                                current,
                                expected
                            );
                            log::info!("restarting esp!");
                            unsafe { esp_idf_svc::sys::esp_restart() };
                        }
                        _ => {
                            let err_msg = GlyphError::new(1, &e.to_string());
                            log::error!("HANDLE FAILED {:?}", e);
                            mqtt_pub(&mut mqtt, &client_id, topics::ERROR, &err_msg.to_vec()[..]);
                        }
                    },
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
            Event::LssMessage(msg_bytes) => {
                match lss::handle_lss_msg(&msg_bytes, msgs, &lss_signer) {
                    Ok((ret_topic, bytes)) => {
                        // set msgs back to None
                        msgs = None;
                        mqtt_pub(&mut mqtt, &client_id, &ret_topic, &bytes);
                        if ret_topic == topics::VLS_RES {
                            log::info!("HMACs matched! commit now...");
                            // and commit
                            if let Err(e) = root_handler.node().get_persister().commit() {
                                log::error!("LOCAL COMMIT ERROR AFTER LSS! {:?}", e);
                                unsafe { esp_idf_svc::sys::esp_restart() };
                            }
                            restart_esp_if_memory_low();
                        }
                        if ret_topic == topics::LSS_CONFLICT_RES {
                            log::error!("LSS PUT CONFLICT! RESTART...");
                            unsafe { esp_idf_svc::sys::esp_restart() };
                        }
                    }
                    Err(e) => {
                        log::error!("LSS MESSAGE FAILED!");
                        log::error!("{}", &e.to_string());
                        msgs = None;
                        let err_msg = GlyphError::new(1, &e.to_string());
                        mqtt_pub(&mut mqtt, &client_id, topics::ERROR, &err_msg.to_vec()[..]);
                    }
                }
            }
            Event::Control(ref msg_bytes) => {
                log::info!("GOT A CONTROL MSG");
                let cres = ctrlr.handle(msg_bytes);
                let res = handle_control_response(&root_handler, &approver, cres, led_tx.clone());
                let mut bb = ByteBuf::new();
                serialize_controlresponse(&mut bb, &res).expect("failed serialize_lssresponse");
                mqtt_pub(&mut mqtt, &client_id, topics::CONTROL_RES, bb.as_slice());
                if let ControlResponse::OtaConfirm(ref params) = res {
                    if let Err(e) = update_sphinx_key(params) {
                        log::error!("OTA update failed {:?}", e.to_string());
                    } else {
                        log::info!("OTA flow complete, restarting esp...");
                        unsafe { esp_idf_svc::sys::esp_restart() };
                    }
                }
            }
        }
    }
}

fn update_led(current: Status, new: Status, led_tx: &mpsc::Sender<Status>) -> Status {
    if current != new {
        led_tx.send(new).unwrap();
        new
    } else {
        current
    }
}

pub(crate) fn restart_esp_if_memory_low() {
    unsafe {
        let size = esp_idf_svc::sys::heap_caps_get_free_size(4);
        let block = esp_idf_svc::sys::heap_caps_get_largest_free_block(4);
        let threshold = 25000;
        log::info!(
            "Available DRAM: {}, Max block: {}, Restart Threshold: {}",
            size,
            block,
            threshold
        );
        if block < threshold {
            log::info!("Restarting esp!");
            esp_idf_svc::sys::esp_restart();
        }
    }
}

pub(crate) fn check_memory() {
    unsafe {
        let size = esp_idf_svc::sys::heap_caps_get_free_size(4);
        let block = esp_idf_svc::sys::heap_caps_get_largest_free_block(4);
        log::info!("CHECK: Available DRAM: {}, Max block: {}", size, block,);
    }
}

fn handle_control_response(
    root_handler: &RootHandler,
    approver: &SphinxApprover,
    cres: anyhow::Result<(ControlMessage, ControlResponse)>,
    led_tx: mpsc::Sender<Status>,
) -> ControlResponse {
    match cres {
        Ok((control_msg, mut control_res)) => {
            // the following msg types require other actions besides Flash persistence
            match control_msg {
                ControlMessage::UpdatePolicy(new_policy) => {
                    if let Err(e) = sphinx_signer::policy::set_approver_policy(approver, new_policy)
                    {
                        log::error!("set policy failed {:?}", e);
                        control_res = ControlResponse::Error(format!("set policy failed {:?}", e))
                    }
                }
                ControlMessage::UpdateAllowlist(al) => {
                    if let Err(e) = sphinx_signer::policy::set_allowlist(root_handler, &al) {
                        log::error!("set allowlist failed {:?}", e);
                        control_res =
                            ControlResponse::Error(format!("set allowlist failed {:?}", e))
                    }
                }
                // overwrite the real Allowlist response, loaded from Node
                ControlMessage::QueryAllowlist => {
                    match sphinx_signer::policy::get_allowlist(root_handler) {
                        Ok(al) => control_res = ControlResponse::AllowlistCurrent(al),
                        Err(e) => {
                            log::error!("read allowlist failed {:?}", e);
                            control_res =
                                ControlResponse::Error(format!("read allowlist failed {:?}", e))
                        }
                    }
                }
                ControlMessage::Ota(ref params) => {
                    if let Err(e) = validate_ota_message(params) {
                        log::error!("OTA update cannot launch {:?}", e.to_string());
                        control_res =
                            ControlResponse::Error(format!("OTA update cannot launch {:?}", e))
                    } else {
                        led_tx.send(Status::Ota).unwrap();
                        log::info!("Launching OTA update...");
                    }
                }
                _ => (),
            };
            control_res
        }
        Err(e) => {
            let control_res = ControlResponse::Error(e.to_string());
            log::warn!("error parsing ctrl msg {:?}", e);
            control_res
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
