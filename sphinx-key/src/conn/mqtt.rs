use crate::core::events::Event as CoreEvent;

use embedded_svc::mqtt::client::utils::ConnState;
use embedded_svc::mqtt::client::{Connection, MessageImpl, QoS, Event, Message as MqttMessage};
use embedded_svc::mqtt::client::utils::Connection as MqttConnection;
use esp_idf_svc::mqtt::client::*;
use anyhow::Result;
use log::*;
use std::thread;
use esp_idf_sys::{self};
use esp_idf_sys::EspError;
use esp_idf_hal::mutex::Condvar;
use std::sync::{mpsc};

pub const TOPIC: &str = "sphinx";
pub const RETURN_TOPIC: &str = "sphinx-return";
pub const USERNAME: &str = "sphinx-key";
pub const PASSWORD: &str = "sphinx-key-pass";
pub const QOS: QoS = QoS::AtMostOnce;

pub fn make_client(broker: &str, client_id: &str) -> Result<(
    EspMqttClient<ConnState<MessageImpl, EspError>>, 
    MqttConnection<Condvar, MessageImpl, EspError>,
)> {
    let conf = MqttClientConfiguration {
        client_id: Some(client_id),
        buffer_size: 2048,
        task_stack: 12288,
        username: Some(USERNAME),
        password: Some(PASSWORD),
        // FIXME - mqtts
        // crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
        ..Default::default()
    };

    let b = format!("mqtt://{}", broker);
    // let (mut client, mut connection) = EspMqttClient::new_with_conn(b, &conf)?;
    let cc = EspMqttClient::new_with_conn(b, &conf)?;

    info!("MQTT client started");

    Ok(cc)
}

pub fn start_listening(
    client: EspMqttClient<ConnState<MessageImpl, EspError>>,
    mut connection: MqttConnection<Condvar, MessageImpl, EspError>, 
    tx: mpsc::Sender<CoreEvent>,
) -> Result<EspMqttClient<ConnState<MessageImpl, EspError>>> {
    
    // must start pumping before subscribe or publish will not work
    thread::spawn(move || {
        info!("MQTT Listening for messages");
        loop {
            match connection.next() {
                Some(msg) => {
                    match msg {
                        Err(e) => match e.to_string().as_ref() {
                            "ESP_FAIL" => {
                                error!("ESP_FAIL msg!");
                            },
                            _ => error!("Unknown error: {}", e),
                        },
                        Ok(msg) => {
                            match msg {
                                Event::BeforeConnect => info!("RECEIVED BeforeConnect MESSAGE"),
                                Event::Connected(_flag) => {
                                    info!("RECEIVED Connected MESSAGE");
                                    tx.send(CoreEvent::Connected).expect("couldnt send Event::Connected");
                                },
                                Event::Disconnected => {
                                    warn!("RECEIVED Disconnected MESSAGE");
                                    tx.send(CoreEvent::Disconnected).expect("couldnt send Event::Disconnected");
                                },
                                Event::Subscribed(_mes_id) => info!("RECEIVED Subscribed MESSAGE"),
                                Event::Unsubscribed(_mes_id) => info!("RECEIVED Unsubscribed MESSAGE"),
                                Event::Published(_mes_id) => info!("RECEIVED Published MESSAGE"),
                                Event::Received(msg) => tx.send(CoreEvent::Message(msg.data().to_vec())).expect("couldnt send Event::Message"),
                                Event::Deleted(_mes_id) => info!("RECEIVED Deleted MESSAGE"),
                            }
                        },
                    }
                },
                None => break,
            }
        }
        //info!("MQTT connection loop exit");
    });

    // log::info!("SUBSCRIBE TO {}", TOPIC);
    // client.subscribe(TOPIC, QoS::AtMostOnce)?;

    Ok(client)
}
