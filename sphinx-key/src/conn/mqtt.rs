use crate::core::events::Message;

use embedded_svc::event_bus::Postbox;
use embedded_svc::mqtt::client::utils::ConnState;
use embedded_svc::mqtt::client::{Client, Connection, MessageImpl, Publish, QoS, Event, Message as MqttMessage};
use embedded_svc::mqtt::client::utils::Connection as MqttConnection;
use esp_idf_svc::mqtt::client::*;
use anyhow::Result;
use esp_idf_svc::eventloop::EspBackgroundEventLoop;
use log::*;
use std::thread;
use esp_idf_sys::{self};
use esp_idf_sys::EspError;
use esp_idf_hal::mutex::Condvar;
use std::sync::{Arc, Mutex};

pub const TOPIC: &str = "sphinx";
pub const RETURN_TOPIC: &str = "sphinx-return";
pub const CLIENT_ID: &str = "sphinx-1";
pub const USERNAME: &str = "sphinx-key";
pub const PASSWORD: &str = "sphinx-key-pass";

pub fn make_client(broker: &str) -> Result<(
    EspMqttClient<ConnState<MessageImpl, EspError>>, 
    MqttConnection<Condvar, MessageImpl, EspError>
)> {
    let conf = MqttClientConfiguration {
        client_id: Some(CLIENT_ID),
        buffer_size: 2048,
        task_stack: 12288,
        username: Some(USERNAME),
        password: Some(PASSWORD),
        // FIXME - mqtts
        // crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
        ..Default::default()
    };

    let b = format!("mqtt://{}", broker);
    println!("===> CONNECT TO {}", b);
    // let (mut client, mut connection) = EspMqttClient::new_with_conn(b, &conf)?;
    let cc = EspMqttClient::new_with_conn(b, &conf)?;
// 
    info!("MQTT client started");

    Ok(cc)
}

pub fn start_listening(
    mqtt: Arc<Mutex<EspMqttClient<ConnState<MessageImpl, EspError>>>>,
    mut connection: MqttConnection<Condvar, MessageImpl, EspError>, 
    mut eventloop: EspBackgroundEventLoop
) -> Result<()> {
    
    // must start pumping before subscribe or publish will work
    thread::spawn(move || {
        info!("MQTT Listening for messages");

        while let Some(msg) = connection.next() {
            match msg {
                Err(e) => info!("MQTT Message ERROR: {}", e),
                Ok(msg) => {
                    if let Event::Received(msg) = msg {
                        info!("MQTT MESSAGE RECEIVED!");
                        if let Ok(m) = Message::new_from_slice(&msg.data()) {
                            if let Err(e) = eventloop.post(&m, None) {
                                warn!("failed to post to eventloop {:?}", e);
                            }
                        }
                    }
                },
            }
        }
        info!("MQTT connection loop exit");
    });

    let mut client = mqtt.lock().unwrap();

    client.subscribe(TOPIC, QoS::AtMostOnce)?;

    client.publish(
        TOPIC,
        QoS::AtMostOnce,
        false,
        format!("Hello from {}!", CLIENT_ID).as_bytes(),
    )?;

    Ok(())
}
