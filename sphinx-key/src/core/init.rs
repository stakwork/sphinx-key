use sphinx_key_signer::lightning_signer::bitcoin::Network;
use sphinx_key_signer::vls_protocol::model::Secret;
use sphinx_key_signer::vls_protocol::{msgs, serde_bolt::WireString};
use sphinx_key_signer::MsgDriver;

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
