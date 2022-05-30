use crate::conn::mqtt::RETURN_TOPIC;

use esp_idf_svc::eventloop::*;
use embedded_svc::httpd::Result;
use esp_idf_sys::{self, c_types};
use embedded_svc::mqtt::client::utils::ConnState;
use embedded_svc::mqtt::client::{MessageImpl, Publish, QoS};
use esp_idf_svc::mqtt::client::*;
use esp_idf_sys::EspError;
use std::sync::{Arc, Mutex};
use log::*;
use std::cmp::min;

pub const MSG_SIZE: usize = 256;

#[derive(Copy, Clone, Debug)]
pub struct Message([u8; MSG_SIZE]);

impl Message {
    pub fn _new(bytes: [u8; MSG_SIZE]) -> Self {
        Self(bytes)
    }
    // the first byte is the length of the message
    pub fn new_from_slice(src: &[u8]) -> Result<Self> {
        if src.len() > MSG_SIZE - 1 {
            return Err(anyhow::anyhow!("message too long"));
        }
        let mut dest = [0; MSG_SIZE];
        dest[0] = src.len() as u8; // this would crash if MSG_SIZE>256
        for i in 0..min(src.len(), MSG_SIZE) {
            dest[i+1] = src[i];
        }
        Ok(Self(dest))
    }
    pub fn read_bytes(&self) -> Vec<u8> {
        let l = self.0[0] as usize;
        self.0[1..l+1].to_vec()
    }
    pub fn read_string(&self) -> String {
        String::from_utf8_lossy(&self.0).to_string()
    }
}

impl EspTypedEventSource for Message {
    fn source() -> *const c_types::c_char {
        b"SPHINX\0".as_ptr() as *const _
    }
}

impl EspTypedEventSerializer<Message> for Message {
    fn serialize<R>(
        event: &Message,
        f: impl for<'a> FnOnce(&'a EspEventPostData) -> R,
    ) -> R {
        f(&unsafe { EspEventPostData::new(Self::source(), Self::event_id(), event) })
    }
}

impl EspTypedEventDeserializer<Message> for Message {
    fn deserialize<R>(
        data: &EspEventFetchData,
        f: &mut impl for<'a> FnMut(&'a Message) -> R,
    ) -> R {
        f(unsafe { data.as_payload() })
    }
}

pub fn make_eventloop(client: Arc<Mutex<EspMqttClient<ConnState<MessageImpl, EspError>>>>) -> Result<(EspBackgroundEventLoop, EspBackgroundSubscription)> {
    use embedded_svc::event_bus::EventBus;

    info!("About to start a background event loop");
    let mut eventloop = EspBackgroundEventLoop::new(
        &BackgroundLoopConfiguration {
            task_stack_size: 8192,
            .. Default::default()
        },
    )?;

    info!("About to subscribe to the background event loop");
    let subscription = eventloop.subscribe(move |message: &Message| {
        info!("!!! Got message from the event loop"); //: {:?}", message.0);
        let msg_str = message.read_string();
        // let msg_str = String::from_utf8_lossy(&msg[..]);
        match client.lock() {
            Ok(mut m_) => if let Err(err) = m_.publish(
                RETURN_TOPIC,
                QoS::AtMostOnce,
                false,
                format!("The processed message: {}", msg_str).as_bytes(),
            ) {
                log::warn!("failed to mqtt publish! {:?}", err);
            },
            Err(_) => log::warn!("failed to lock Mutex<Client>")
        };
        
    })?;
    // let subscription = eventloop.subscribe(cb)?;

    Ok((eventloop, subscription))
}