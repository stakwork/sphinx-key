use embedded_svc::mqtt::client::utils::ConnState;
use embedded_svc::mqtt::client::{Client, Connection, MessageImpl, Publish, QoS, Event, Message as MqttMessage};
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
    mut client: EspMqttClient<ConnState<MessageImpl, EspError>>,
    mut connection: MqttConnection<Condvar, MessageImpl, EspError>, 
    tx: mpsc::Sender<Vec<u8>>,
) -> Result<EspMqttClient<ConnState<MessageImpl, EspError>>> {
    
    // must start pumping before subscribe or publish will work
    thread::spawn(move || {
        info!("MQTT Listening for messages");

        while let Some(msg) = connection.next() {
            match msg {
                Err(e) => info!("MQTT Message ERROR: {}", e),
                Ok(msg) => {
                    if let Event::Received(msg) = msg {
                        tx.send(msg.data().to_vec()).expect("could send to TX");
                    }
                },
            }
        }
        info!("MQTT connection loop exit");
    });

    // log::info!("lock mqtt mutex guard");
    // let mut client = mqtt.lock().unwrap();

    log::info!("SUBSCRIBE TO {}", TOPIC);
    client.subscribe(TOPIC, QoS::AtMostOnce)?;

    log::info!("PUBLISH {} to {}", "READY", RETURN_TOPIC);
    client.publish(
        RETURN_TOPIC,
        QoS::AtMostOnce,
        false,
        format!("READY").as_bytes(),
    )?;

    Ok(client)
}
