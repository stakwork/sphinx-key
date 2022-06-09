use sphinx_key_parser::MsgDriver;
use vls_protocol::model::Secret;
use vls_protocol::{msgs, serde_bolt::WireString};
use vls_proxy::util::{read_allowlist, read_integration_test_seed};

pub fn make_init_msg() -> anyhow::Result<Vec<u8>> {
    let allowlist = read_allowlist()
        .into_iter()
        .map(|s| WireString(s.as_bytes().to_vec()))
        .collect::<Vec<_>>();
    let seed = read_integration_test_seed()
        .map(|s| Secret(s))
        .or(Some(Secret([1; 32])));
    // FIXME remove this
    log::info!("allowlist {:?} seed {:?}", allowlist, seed);
    let init = msgs::HsmdInit2 {
        derivation_style: 0,
        network_name: WireString("testnet".as_bytes().to_vec()),
        dev_seed: seed,
        dev_allowlist: allowlist,
    };
    let sequence = 0;
    let mut md = MsgDriver::new_empty();
    msgs::write_serial_request_header(&mut md, sequence, 0)?;
    msgs::write(&mut md, init)?;
    Ok(md.bytes())
    // msgs::read_serial_response_header(&mut serial, sequence)?;
    // let init_reply: msgs::HsmdInit2Reply = msgs::read_message(&mut serial)?;
    // log::info!("init reply {:?}", init_reply);
    // Ok(())
}
