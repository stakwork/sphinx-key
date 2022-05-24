
use esp_idf_svc::eventloop::*;
use embedded_svc::httpd::Result;
use esp_idf_sys::{self, c_types};
use log::*;

const MSG_SIZE: usize = 256;

#[derive(Copy, Clone, Debug)]
pub struct Message([u8; MSG_SIZE]);

impl Message {
    pub fn new(bytes: [u8; MSG_SIZE]) -> Self {
        Self(bytes)
    }
}

impl EspTypedEventSource for Message {
    fn source() -> *const c_types::c_char {
        b"DEMO-SERVICE\0".as_ptr() as *const _
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

pub fn make_eventloop() -> Result<(EspBackgroundEventLoop, EspBackgroundSubscription)> {
    use embedded_svc::event_bus::EventBus;

    info!("About to start a background event loop");
    let mut eventloop = EspBackgroundEventLoop::new(&Default::default())?;

    info!("About to subscribe to the background event loop");
    let subscription = eventloop.subscribe(|message: &Message| {
        info!("!!! Got message from the event loop"); //: {:?}", message.0);
    })?;
    // let subscription = eventloop.subscribe(cb)?;

    Ok((eventloop, subscription))
}