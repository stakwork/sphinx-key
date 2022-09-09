use crate::core::events::Event as CoreEvent;

use anyhow::Result;
use embedded_svc::mqtt::client::utils::ConnState;
use embedded_svc::mqtt::client::utils::Connection as MqttConnection;
use embedded_svc::mqtt::client::{Connection, Event, Message as MqttMessage, MessageImpl, QoS};
use esp_idf_hal::mutex::Condvar;
use esp_idf_svc::mqtt::client::*;
use esp_idf_sys::EspError;
use esp_idf_sys::{self};
use log::*;
use std::sync::mpsc;
use std::thread;

pub const VLS_TOPIC: &str = "sphinx";
pub const CONTROL_TOPIC: &str = "sphinx-control";
pub const RETURN_TOPIC: &str = "sphinx-return";
pub const CONTROL_RETURN_TOPIC: &str = "sphinx-control-return";
pub const USERNAME: &str = "sphinx-key";
pub const PASSWORD: &str = "sphinx-key-pass";
pub const QOS: QoS = QoS::AtMostOnce;

pub fn make_client(
    broker: &str,
    client_id: &str,
) -> Result<(
    EspMqttClient<ConnState<MessageImpl, EspError>>,
    MqttConnection<Condvar, MessageImpl, EspError>,
)> {
    log::info!("make_client with id {}", client_id);
    let conf = MqttClientConfiguration {
        client_id: Some(client_id),
        buffer_size: 4096,
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
                Some(msg) => match msg {
                    Err(e) => match e.to_string().as_ref() {
                        "ESP_FAIL" => {
                            error!("ESP_FAIL msg!");
                        }
                        _ => error!("Unknown error: {}", e),
                    },
                    Ok(msg) => match msg {
                        Event::BeforeConnect => info!("RECEIVED BeforeConnect MESSAGE"),
                        Event::Connected(_flag) => {
                            info!("RECEIVED Connected MESSAGE");
                            tx.send(CoreEvent::Connected)
                                .expect("couldnt send Event::Connected");
                        }
                        Event::Disconnected => {
                            warn!("RECEIVED Disconnected MESSAGE");
                            tx.send(CoreEvent::Disconnected)
                                .expect("couldnt send Event::Disconnected");
                        }
                        Event::Subscribed(_mes_id) => info!("RECEIVED Subscribed MESSAGE"),
                        Event::Unsubscribed(_mes_id) => info!("RECEIVED Unsubscribed MESSAGE"),
                        Event::Published(_mes_id) => info!("RECEIVED Published MESSAGE"),
                        Event::Received(msg) => {
                            let topic_opt = msg.topic();
                            if let Some(topic) = topic_opt {
                                match topic {
                                    VLS_TOPIC => tx
                                        .send(CoreEvent::VlsMessage(msg.data().to_vec()))
                                        .expect("couldnt send Event::VlsMessage"),
                                    CONTROL_TOPIC => tx
                                        .send(CoreEvent::Control(msg.data().to_vec()))
                                        .expect("couldnt send Event::Control"),
                                    _ => log::warn!("unrecognized topic {}", topic),
                                };
                            } else {
                                log::warn!("empty topic in msg!!!");
                            }
                        }
                        Event::Deleted(_mes_id) => info!("RECEIVED Deleted MESSAGE"),
                    },
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
