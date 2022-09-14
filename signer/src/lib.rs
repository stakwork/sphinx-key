mod derive;
mod policy;
mod randomstartingtime;

use lightning_signer::bitcoin::blockdata::constants::ChainHash;
use lightning_signer::node::NodeServices;
use lightning_signer::persist::Persist;
use lightning_signer::policy::simple_validator::SimpleValidatorFactory;
use lightning_signer::util::clock::StandardClock;
use randomstartingtime::RandomStartingTimeFactory;
use std::sync::Arc;
use vls_protocol::model::{PubKey, Secret};
use vls_protocol::msgs::{self, read_serial_request_header, write_serial_response_header, Message};
use vls_protocol::serde_bolt::WireString;
pub use vls_protocol_signer::handler::{Handler, RootHandler};
pub use vls_protocol_signer::lightning_signer;
use vls_protocol_signer::lightning_signer::bitcoin::Network;
use vls_protocol_signer::lightning_signer::wallet::Wallet;
pub use vls_protocol_signer::vls_protocol;

pub use derive::node_keys as derive_node_keys;
pub use policy::{get_allowlist, make_policy, set_allowlist, set_policy};
pub use sphinx_key_parser::{control, topics, MsgDriver};
pub use sphinx_key_persister::FsPersister;
pub struct InitResponse {
    pub root_handler: RootHandler,
    pub init_reply: Vec<u8>,
}

pub const ROOT_STORE: &str = "/sdcard/store";

pub fn init(
    bytes: Vec<u8>,
    network: Network,
    po: &control::Policy,
) -> anyhow::Result<InitResponse> {
    // let persister: Arc<dyn Persist> = Arc::new(DummyPersister);
    let mut md = MsgDriver::new(bytes);
    let (sequence, dbid) = read_serial_request_header(&mut md).expect("read init header");
    assert_eq!(dbid, 0);
    assert_eq!(sequence, 0);
    let init: msgs::HsmdInit2 = msgs::read_message(&mut md).expect("failed to read init message");
    log::info!("init {:?}", init);

    let seed = init.dev_seed.as_ref().map(|s| s.0).expect("no seed");
    let allowlist = init
        .dev_allowlist
        .iter()
        .map(|s| from_wire_string(s))
        .collect::<Vec<_>>();
    log::info!("allowlist {:?}", allowlist);
    let policy = make_policy(network, po);
    let validator_factory = Arc::new(SimpleValidatorFactory::new_with_policy(policy));
    let random_time_factory = RandomStartingTimeFactory::new();
    let persister: Arc<dyn Persist> = Arc::new(FsPersister::new(ROOT_STORE));
    let clock = Arc::new(StandardClock());
    let services = NodeServices {
        validator_factory,
        starting_time_factory: random_time_factory,
        persister,
        clock,
    };
    log::info!("create root handler now");
    let root_handler = RootHandler::new(network, 0, Some(seed), allowlist, services);
    log::info!("root_handler created");
    let init_reply = root_handler
        .handle(Message::HsmdInit2(init))
        .expect("handle init");
    let mut reply = MsgDriver::new_empty();
    write_serial_response_header(&mut reply, sequence).expect("write init header");
    msgs::write_vec(&mut reply, init_reply.as_vec()).expect("write init reply");
    Ok(InitResponse {
        root_handler,
        init_reply: reply.bytes(),
    })
}

pub fn handle(
    root_handler: &RootHandler,
    bytes: Vec<u8>,
    dummy_peer: PubKey,
    do_log: bool,
) -> anyhow::Result<Vec<u8>> {
    let mut md = MsgDriver::new(bytes);
    let (sequence, dbid) = read_serial_request_header(&mut md).expect("read request header");
    let mut message = msgs::read(&mut md).expect("message read failed");

    // Override the peerid when it is passed in certain messages
    match message {
        Message::NewChannel(ref mut m) => m.node_id = dummy_peer.clone(),
        Message::ClientHsmFd(ref mut m) => m.peer_id = dummy_peer.clone(),
        Message::GetChannelBasepoints(ref mut m) => m.node_id = dummy_peer.clone(),
        Message::SignCommitmentTx(ref mut m) => m.peer_id = dummy_peer.clone(),
        _ => {}
    };

    if let Message::HsmdInit(ref m) = message {
        if ChainHash::using_genesis_block(root_handler.node.network()).as_bytes()
            != &m.chain_params.0
        {
            log::error!("The network setting of CLN and VLS don't match!");
            panic!("The network setting of CLN and VLS don't match!");
        }
    }

    if do_log {
        log::info!("VLS msg: {:?}", message);
    }
    let reply = if dbid > 0 {
        let handler = root_handler.for_new_client(dbid, dummy_peer.clone(), dbid);
        handler.handle(message).expect("handle")
    } else {
        root_handler.handle(message).expect("handle")
    };
    if do_log {
        log::info!("VLS msg handled");
    }
    let mut out_md = MsgDriver::new_empty();
    write_serial_response_header(&mut out_md, sequence).expect("write reply header");
    msgs::write_vec(&mut out_md, reply.as_vec()).expect("write reply");
    Ok(out_md.bytes())
}

pub fn make_init_msg(network: Network, seed: [u8; 32]) -> anyhow::Result<Vec<u8>> {
    let allowlist = Vec::new();
    log::info!("allowlist {:?} seed {:?}", allowlist, seed);
    let init = msgs::HsmdInit2 {
        derivation_style: 0,
        network_name: WireString(network.to_string().as_bytes().to_vec()),
        dev_seed: Some(Secret(seed)),
        dev_allowlist: allowlist,
    };
    let sequence = 0;
    let mut md = MsgDriver::new_empty();
    msgs::write_serial_request_header(&mut md, sequence, 0)?;
    msgs::write(&mut md, init)?;
    Ok(md.bytes())
}

pub fn parse_ping_and_form_response(msg_bytes: Vec<u8>) -> Vec<u8> {
    let mut m = MsgDriver::new(msg_bytes);
    let (sequence, _dbid) = msgs::read_serial_request_header(&mut m).expect("read ping header");
    let ping: msgs::Ping = msgs::read_message(&mut m).expect("failed to read ping message");
    let mut md = MsgDriver::new_empty();
    msgs::write_serial_response_header(&mut md, sequence)
        .expect("failed to write_serial_request_header");
    let pong = msgs::Pong {
        id: ping.id,
        message: ping.message,
    };
    msgs::write(&mut md, pong).expect("failed to serial write");
    md.bytes()
}

fn from_wire_string(s: &WireString) -> String {
    String::from_utf8(s.0.to_vec()).expect("malformed string")
}
