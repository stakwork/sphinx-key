use crate::core::events::Event as CoreEvent;
use sphinx_signer::sphinx_glyph::topics;

use anyhow::Result;
use embedded_svc::mqtt::client::Details;
use embedded_svc::mqtt::client::{Connection, Event, Message as MqttMessage, MessageImpl, QoS};
use embedded_svc::utils::mqtt::client::ConnState;
// use embedded_svc::utils::mqtt::client::Connection as MqttConnection;
// use embedded_svc::utils::mutex::Condvar;
use esp_idf_svc::mqtt::client::*;
use esp_idf_sys::EspError;
use esp_idf_sys::{self};
use log::*;
use std::sync::mpsc;
use std::thread;

pub const QOS: QoS = QoS::AtMostOnce;

pub fn make_client(
    broker: &str,
    client_id: &str,
    username: &str,
    password: &str,
    tx: mpsc::Sender<CoreEvent>,
) -> Result<EspMqttClient<ConnState<MessageImpl, EspError>>> {
    log::info!("make_client with id {}", client_id);
    let conf = MqttClientConfiguration {
        client_id: Some(client_id),
        buffer_size: 4096,
        task_stack: 12288,
        username: Some(username),
        password: Some(password),
        crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
        ..Default::default()
    };

    let mut mqtturl = broker.to_string();
    if !(mqtturl.starts_with("mqtt://") || mqtturl.starts_with("mqtts://")) {
        let scheme = if mqtturl.contains("8883") {
            "mqtts"
        } else {
            "mqtt"
        };
        mqtturl = format!("{}://{}", scheme, mqtturl);
    }
    info!("=> connect to MQTT at {}", mqtturl);
    let (client, mut connection) = EspMqttClient::new_with_conn(&mqtturl, &conf)?;
    // let cc = EspMqttClient::new_with_conn(b, &conf)?;

    info!("MQTT client started");

    thread::spawn(move || {
        info!("MQTT Listening for messages");
        let mut inflight = Vec::new();
        let mut inflight_topic = "".to_string();
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
                            let incoming_message: Option<(String, Vec<u8>)> = match msg.details() {
                                Details::Complete => {
                                    if let Some(topic) = msg.topic() {
                                        Some((topic.to_string(), msg.data().to_vec()))
                                    } else {
                                        None
                                    }
                                }
                                Details::InitialChunk(chunk_info) => {
                                    if let Some(topic) = msg.topic() {
                                        inflight_topic = topic.to_string();
                                        inflight.extend(msg.data().iter());
                                        None
                                    } else {
                                        None
                                    }
                                }
                                Details::SubsequentChunk(chunk_data) => {
                                    inflight.extend(msg.data().iter());
                                    if inflight.len() == chunk_data.total_data_size {
                                        let ret = Some((inflight_topic, inflight));
                                        inflight_topic = "".to_string();
                                        inflight = Vec::new();
                                        ret
                                    } else {
                                        None
                                    }
                                }
                            };

                            if let Some((topic, data)) = incoming_message {
                                if topic.ends_with(topics::VLS) {
                                    tx.send(CoreEvent::VlsMessage(data))
                                        .expect("couldnt send Event::VlsMessage");
                                } else if topic.ends_with(topics::LSS_MSG) {
                                    log::debug!("received data len {}", data.len());
                                    tx.send(CoreEvent::LssMessage(data))
                                        .expect("couldnt send Event::LssMessage");
                                } else if topic.ends_with(topics::CONTROL) {
                                    tx.send(CoreEvent::Control(data))
                                        .expect("couldnt send Event::Control");
                                } else {
                                    log::warn!("unrecognized topic {}", topic);
                                }
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

    Ok(client)
}

// pub fn start_listening(
//     client: EspMqttClient<ConnState<MessageImpl, EspError>>,
//     mut connection: MqttConnection<Condvar, MessageImpl, EspError>,
//     tx: mpsc::Sender<CoreEvent>,
// ) -> Result<EspMqttClient<ConnState<MessageImpl, EspError>>> {
//     // must start pumping before subscribe or publish will not work
//     thread::spawn(move || {
//         info!("MQTT Listening for messages");
//         loop {
//             match connection.next() {
//                 Some(msg) => match msg {
//                     Err(e) => match e.to_string().as_ref() {
//                         "ESP_FAIL" => {
//                             error!("ESP_FAIL msg!");
//                         }
//                         _ => error!("Unknown error: {}", e),
//                     },
//                     Ok(msg) => match msg {
//                         Event::BeforeConnect => info!("RECEIVED BeforeConnect MESSAGE"),
//                         Event::Connected(_flag) => {
//                             info!("RECEIVED Connected MESSAGE");
//                             tx.send(CoreEvent::Connected)
//                                 .expect("couldnt send Event::Connected");
//                         }
//                         Event::Disconnected => {
//                             warn!("RECEIVED Disconnected MESSAGE");
//                             tx.send(CoreEvent::Disconnected)
//                                 .expect("couldnt send Event::Disconnected");
//                         }
//                         Event::Subscribed(_mes_id) => info!("RECEIVED Subscribed MESSAGE"),
//                         Event::Unsubscribed(_mes_id) => info!("RECEIVED Unsubscribed MESSAGE"),
//                         Event::Published(_mes_id) => info!("RECEIVED Published MESSAGE"),
//                         Event::Received(msg) => {
//                             let topic_opt = msg.topic();
//                             if let Some(topic) = topic_opt {
//                                 match topic {
//                                     topics::VLS => tx
//                                         .send(CoreEvent::VlsMessage(msg.data().to_vec()))
//                                         .expect("couldnt send Event::VlsMessage"),
//                                     topics::CONTROL => tx
//                                         .send(CoreEvent::Control(msg.data().to_vec()))
//                                         .expect("couldnt send Event::Control"),
//                                     _ => log::warn!("unrecognized topic {}", topic),
//                                 };
//                             } else {
//                                 log::warn!("empty topic in msg!!!");
//                             }
//                         }
//                         Event::Deleted(_mes_id) => info!("RECEIVED Deleted MESSAGE"),
//                     },
//                 },
//                 None => break,
//             }
//         }
//         //info!("MQTT connection loop exit");
//     });

//     Ok(client)
// }
