use crate::core::events::Message;
use embedded_svc::event_bus::Postbox;

use embedded_svc::event_bus::EventBus;
use embedded_svc::mqtt::client::utils::ConnState;
use embedded_svc::mqtt::client::{Client, Connection, MessageImpl, Publish, QoS};
use esp_idf_svc::mqtt::client::*;
use anyhow::Result;
use esp_idf_svc::eventloop::*;
use log::*;
use std::thread;
use esp_idf_sys::{self};
use esp_idf_sys::EspError;

pub fn mqtt_client(broker: &str, mut eventloop: EspBackgroundEventLoop) -> Result<EspMqttClient<ConnState<MessageImpl, EspError>>> {
    info!("About to start MQTT client");

    let conf = MqttClientConfiguration {
        client_id: Some("rust-esp32-std-demo-1"),
        // FIXME - mqtts
        // crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
        ..Default::default()
    };

    let b = format!("mqtt://{}", broker);
    println!("===> CONNECT TO {}", b);
    let (mut client, mut connection) = EspMqttClient::new_with_conn(b, &conf)?;

    info!("MQTT client started");

    // let subscription = eventloop.subscribe(|message: &Message| {
    //     log::info!("!!! Got message from the event loop"); //: {:?}", message.0);
    // })?;

    // Need to immediately start pumping the connection for messages, or else subscribe() and publish() below will not work
    // Note that when using the alternative constructor - `EspMqttClient::new` - you don't need to
    // spawn a new thread, as the messages will be pumped with a backpressure into the callback you provide.
    // Yet, you still need to efficiently process each message in the callback without blocking for too long.
    thread::spawn(move || {
        info!("MQTT Listening for messages");

        while let Some(msg) = connection.next() {
            match msg {
                Err(e) => info!("MQTT Message ERROR: {}", e),
                Ok(msg) => {
                    eventloop.post(&Message::new([0; 256]), None).unwrap();
                    info!("MQTT Message: {:?}", msg);
                },
            }
        }

        info!("MQTT connection loop exit");
    });

    client.subscribe("rust-esp32-std-demo", QoS::AtMostOnce)?;

    info!("Subscribed to all topics (rust-esp32-std-demo)");

    client.publish(
        "rust-esp32-std-demo",
        QoS::AtMostOnce,
        false,
        "Hello from rust-esp32-std-demo!".as_bytes(),
    )?;

    info!("Published a hello message to topic \"rust-esp32-std-demo\"");

    Ok(client)
}