use crate::core::events::Event as CoreEvent;
use sphinx_signer::sphinx_glyph::topics;

use anyhow::Result;
use esp_idf_svc::mqtt::client::*;
use esp_idf_svc::sys::EspError;
use log::*;
pub(crate) use sphinx_signer::root::MsgBytes;
use std::sync::mpsc;
use std::thread;

pub const QOS: QoS = QoS::AtMostOnce;

pub fn make_client(
    broker: &str,
    signer_id: &[u8; 16],
    username: &str,
    password: &str,
    tx: mpsc::Sender<CoreEvent>,
) -> Result<EspMqttClient<'static, ConnState<MessageImpl, EspError>>> {
    let client_id = hex::encode(signer_id);
    log::info!("make_client with id {}", client_id);

    let mut conf = MqttClientConfiguration {
        client_id: Some(&client_id),
        out_buffer_size: 2 * 1024,
        username: Some(username),
        password: Some(password),
        ..Default::default()
    };

    conf.crt_bundle_attach = Some(esp_idf_svc::sys::esp_crt_bundle_attach);
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

    let builder = thread::Builder::new().stack_size(2048);
    builder.spawn(move || {
        info!("MQTT Listening for messages");
        let mut inflight = MsgBytes::new();
        let mut inflight_topic = "".to_string();
        while let Some(msg) = connection.next() {
            match msg {
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
                        let incoming_message: Option<(String, MsgBytes)> = match msg.details() {
                            Details::Complete => {
                                let mut buf = MsgBytes::new();
                                buf.write(msg.data());
                                msg.topic().map(|topic| (topic.to_string(), buf))
                            }
                            Details::InitialChunk(_chunk_info) => {
                                if let Some(topic) = msg.topic() {
                                    inflight_topic = topic.to_string();
                                    inflight.write(msg.data());
                                    None
                                } else {
                                    None
                                }
                            }
                            Details::SubsequentChunk(chunk_data) => {
                                inflight.write(msg.data());
                                if inflight.len() == chunk_data.total_data_size {
                                    let ret = Some((inflight_topic, inflight));
                                    inflight_topic = String::new();
                                    inflight = MsgBytes::new();
                                    ret
                                } else {
                                    None
                                }
                            }
                        };
                        drop(msg);
                        if let Some((topic, data)) = incoming_message {
                            if topic.ends_with(topics::VLS) {
                                tx.send(CoreEvent::VlsMessage(data))
                                    .expect("couldnt send Event::VlsMessage");
                            } else if topic.ends_with(topics::LSS_MSG)
                                || topic.ends_with(topics::INIT_1_MSG)
                                || topic.ends_with(topics::INIT_2_MSG)
                                || topic.ends_with(topics::LSS_CONFLICT)
                            {
                                log::debug!("received data len {}", data.len());
                                tx.send(CoreEvent::LssMessage(data.to_vec()))
                                    .expect("couldnt send Event::LssMessage");
                            } else if topic.ends_with(topics::CONTROL) {
                                tx.send(CoreEvent::Control(data.to_vec()))
                                    .expect("couldnt send Event::Control");
                            } else {
                                log::warn!("unrecognized topic {}", topic);
                            }
                        }
                    }
                    Event::Deleted(_mes_id) => info!("RECEIVED Deleted MESSAGE"),
                },
            } // match
        } // while let
          //info!("MQTT connection loop exit");
    })?; // spawn

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
