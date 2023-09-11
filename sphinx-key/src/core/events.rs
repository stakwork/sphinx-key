use crate::conn::mqtt::QOS;
use crate::core::lss;
use crate::core::FlashPersister;
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
use sphinx_signer::lightning_signer::persist::Persist;
use sphinx_signer::persist::{BackupPersister, FsPersister, ThreadMemoPersister};
use sphinx_signer::root::VlsHandlerError;
use sphinx_signer::sphinx_glyph as glyph;
use sphinx_signer::{self, RootHandler};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use embedded_svc::httpd::Result;
// use embedded_svc::mqtt::client::Client;
use embedded_svc::mqtt::client::MessageImpl;
use embedded_svc::utils::mqtt::client::ConnState;
use esp_idf_svc::mqtt::client::*;
use esp_idf_sys;
use esp_idf_sys::EspError;

#[derive(Debug)]
pub enum Event {
    Connected,
    Disconnected,
    VlsMessage(Vec<u8>),
    LssMessage(Vec<u8>),
    Control(Vec<u8>),
}

pub const ROOT_STORE: &str = "/sdcard/store";

pub const SUB_TOPICS: &[&str] = &[
    topics::INIT_1_MSG,
    topics::INIT_2_MSG,
    topics::LSS_MSG,
    topics::VLS,
    topics::CONTROL,
];

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
    msgs_persister: Arc<Mutex<FlashPersister>>,
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

    //let sd_persister = DummyPersister {};
    //let initial_allowlist = Vec::new();

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
        initial_allowlist,
        velocity.clone(),
        persister,
    )
    .expect("failed to init signer");

    thread::sleep(std::time::Duration::from_secs(1));
    // send the initial HELLO
    mqtt_pub(&mut mqtt, client_id, topics::HELLO, &[]);

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
    let mut expected_sequence = None;
    while let Ok(event) = rx.recv() {
        match event {
            Event::Connected => {
                log::info!("GOT A Event::Connected msg!");
                mqtt_sub(&mut mqtt, client_id, SUB_TOPICS);
                thread::sleep(std::time::Duration::from_secs(1));
                // send the initial HELLO again
                mqtt_pub(&mut mqtt, client_id, topics::HELLO, &[]);
                led_tx.send(Status::Connected).unwrap();
            }
            Event::Disconnected => {
                led_tx.send(Status::ConnectingToMqtt).unwrap();
                log::info!("GOT A Event::Disconnected msg!");
            }
            Event::VlsMessage(msg_bytes) => {
                led_tx.send(Status::Signing).unwrap();
                let state1 = approver.control().get_state();
                log::info!("FULL MSG {:?}", &msg_bytes);
                let _ret = match sphinx_signer::root::handle_with_lss(
                    &root_handler,
                    &lss_signer,
                    msg_bytes,
                    expected_sequence,
                    do_log,
                ) {
                    Ok((vls_b, lss_b, sequence, _cmd)) => {
                        if lss_b.len() == 0 {
                            // no muts, respond directly back!
                            mqtt_pub(&mut mqtt, client_id, topics::VLS_RES, &vls_b);
                            restart_esp_if_memory_low();
                        } else {
                            // muts! send LSS first!
                            mqtt_pub(&mut mqtt, client_id, topics::LSS_RES, &lss_b);
                            msgs_persister.lock().unwrap().set_prevs(&vls_b, &lss_b)?;
                            msgs = Some((vls_b, lss_b));
                        }
                        expected_sequence = Some(sequence + 1);
                    }
                    Err(e) => match e {
                        VlsHandlerError::BadSequence(current, expected) => unsafe {
                            log::info!(
                                "caught a badsequence error, current: {}, expected: {}",
                                current,
                                expected
                            );
                            log::info!("restarting esp!");
                            esp_idf_sys::esp_restart();
                        },
                        _ => {
                            let err_msg = GlyphError::new(1, &e.to_string());
                            log::error!("HANDLE FAILED {:?}", e);
                            mqtt_pub(&mut mqtt, client_id, topics::ERROR, &err_msg.to_vec()[..]);
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
                if msgs.is_none() {
                    log::warn!("Restoring previous message from sd card");
                    msgs = Some(
                        msgs_persister
                            .lock()
                            .unwrap()
                            .read_prevs()
                            .map_err(|e| anyhow::anyhow!("{:?}", e))?,
                    )
                }
                match lss::handle_lss_msg(&msg_bytes, msgs, &lss_signer) {
                    Ok((ret_topic, bytes)) => {
                        // set msgs back to None
                        msgs = None;
                        mqtt_pub(&mut mqtt, client_id, &ret_topic, &bytes);
                        if ret_topic == topics::VLS_RES {
                            restart_esp_if_memory_low();
                        }
                    }
                    Err(e) => {
                        log::error!("LSS MESSAGE FAILED!");
                        log::error!("{}", &e.to_string());
                        msgs = None;
                        let err_msg = GlyphError::new(1, &e.to_string());
                        mqtt_pub(&mut mqtt, client_id, topics::ERROR, &err_msg.to_vec()[..]);
                    }
                }
            }
            Event::Control(ref msg_bytes) => {
                log::info!("GOT A CONTROL MSG");
                let cres = ctrlr.handle(msg_bytes);
                if let Some(res) =
                    handle_control_response(&root_handler, &approver, cres, led_tx.clone())
                {
                    let mut bb = ByteBuf::new();
                    serialize_controlresponse(&mut bb, &res).expect("failed serialize_lssresponse");
                    mqtt_pub(&mut mqtt, client_id, topics::CONTROL_RES, bb.as_slice());
                }
            }
        }
    }

    Ok(())
}

fn restart_esp_if_memory_low() {
    unsafe {
        let size = esp_idf_sys::heap_caps_get_free_size(4);
        let block = esp_idf_sys::heap_caps_get_largest_free_block(4);
        let threshold = 25000;
        log::info!(
            "Available DRAM: {}, Max block: {}, Restart Threshold: {}",
            size,
            block,
            threshold
        );
        if block < threshold {
            log::info!("Restarting esp!");
            esp_idf_sys::esp_restart();
        }
    }
}

fn handle_control_response(
    root_handler: &RootHandler,
    approver: &SphinxApprover,
    cres: anyhow::Result<(ControlMessage, ControlResponse)>,
    led_tx: mpsc::Sender<Status>,
) -> Option<ControlResponse> {
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
                        // A 10kB size stack was consistently overflowing when doing a factory reset
                        let builder = thread::Builder::new().stack_size(15000usize);
                        builder
                            .spawn(move || {
                                led_tx.send(Status::Ota).unwrap();
                                if let Err(e) = update_sphinx_key(params, led_tx) {
                                    log::error!("OTA update failed {:?}", e.to_string());
                                } else {
                                    log::info!("OTA flow complete, restarting esp...");
                                    unsafe { esp_idf_sys::esp_restart() };
                                }
                            })
                            .unwrap();
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
