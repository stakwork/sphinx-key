use crate::conn::mqtt::RETURN_TOPIC;
use sphinx_key_signer::{self, InitResponse};
use std::sync::{mpsc};
use std::thread;

use embedded_svc::httpd::Result;
use esp_idf_sys;
use embedded_svc::mqtt::client::utils::ConnState;
use embedded_svc::mqtt::client::{MessageImpl, Publish, QoS};
use esp_idf_svc::mqtt::client::*;
use esp_idf_sys::EspError;
use std::sync::{Arc, Mutex};
use log::*;

pub fn make_event_thread(mqtt: Arc<Mutex<EspMqttClient<ConnState<MessageImpl, EspError>>>>, rx: mpsc::Receiver<Vec<u8>>) -> Result<()> {

    thread::spawn(move||{
        let mut client = mqtt.lock().unwrap();
        info!("About to subscribe to the mpsc channel");

        let init_msg_bytes = rx.recv().expect("NO INIT MSG");
        let InitResponse { root_handler, init_reply } = sphinx_key_signer::init(init_msg_bytes).expect("failed to init signer");
        client.publish(
            RETURN_TOPIC,
            QoS::AtMostOnce,
            false,
            init_reply,
        ).expect("could not publish init response");

        while let Ok(msg_bytes) = rx.recv() {
            let _ret = match sphinx_key_signer::handle(&root_handler, msg_bytes) {
                Ok(b) =>  client.publish(RETURN_TOPIC, QoS::AtMostOnce, false, b).expect("could not publish init response"),
                Err(e) => panic!("HANDLE FAILED {:?}", e),
            };
        }

    });

    Ok(())
}


pub fn make_test_event_thread(mqtt: Arc<Mutex<EspMqttClient<ConnState<MessageImpl, EspError>>>>, rx: mpsc::Receiver<Vec<u8>>) -> Result<()> {

    thread::spawn(move||{
        let mut client = mqtt.lock().unwrap();
        info!("About to subscribe to the mpsc channel");

        while let Ok(msg_bytes) = rx.recv() {
            let b = sphinx_key_signer::parse_ping_and_form_response(msg_bytes);
            log::info!("GOT A PING MESSAGE! returning pong now...");
            client.publish(RETURN_TOPIC, QoS::AtMostOnce, false, b).expect("could not publish init response");
        }

    });

    Ok(())
}
