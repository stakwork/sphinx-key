use sphinx_key_parser::MsgDriver;
use lightning_signer::persist::{DummyPersister, Persist};
use lightning_signer::Arc;
use vls_protocol::model::PubKey;
use vls_protocol::msgs;
use vls_protocol::serde_bolt::WireString;
use vls_protocol_signer::handler::{Handler, RootHandler};
use vls_protocol_signer::lightning_signer;
use vls_protocol_signer::vls_protocol;

pub fn parse_ping(msg_bytes: Vec<u8>) -> msgs::Ping {
    let mut m = MsgDriver::new(msg_bytes);
    let (sequence, dbid) = msgs::read_serial_request_header(&mut m).expect("read ping header");
    let ping: msgs::Ping =
        msgs::read_message(&mut m).expect("failed to read ping message");
    ping
}

pub fn say_hi() {
    let persister: Arc<dyn Persist> = Arc::new(DummyPersister);

    println!("Hello, world!");
}
