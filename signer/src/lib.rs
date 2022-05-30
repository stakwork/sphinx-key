pub mod msg;

use lightning_signer::persist::{DummyPersister, Persist};
use lightning_signer::Arc;
use vls_protocol::model::PubKey;
use vls_protocol::msgs::{self, read_serial_request_header, write_serial_response_header, Message};
use vls_protocol::serde_bolt::WireString;
use vls_protocol_signer::handler::{Handler, RootHandler};
use vls_protocol_signer::lightning_signer;
use vls_protocol_signer::vls_protocol;

pub fn say_hi() {
    let persister: Arc<dyn Persist> = Arc::new(DummyPersister);

    println!("Hello, world!");
}
