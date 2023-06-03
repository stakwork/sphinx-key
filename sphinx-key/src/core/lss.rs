use anyhow::{anyhow, Result};
use lss_connector::{secp256k1::PublicKey, LssSigner, Msg as LssMsg, Response as LssRes};
use sphinx_signer::{self, RootHandler, RootHandlerBuilder};

pub fn init_lss() -> Result<(RootHandler, LssSigner)> {
    let init = LssMsg::from_slice(&[0])?.as_init()?;
    Err(anyhow!("test"))
}
