use lightning_signer::signer::StartingTimeFactory;
use rand::{rngs::OsRng, RngCore};
use std::sync::Arc;
use vls_protocol_signer::lightning_signer;

/// A starting time factory which uses entropy from the RNG
pub(crate) struct RandomStartingTimeFactory {}

impl StartingTimeFactory for RandomStartingTimeFactory {
    fn starting_time(&self) -> (u64, u32) {
        (OsRng.next_u64(), OsRng.next_u32())
    }
}

impl RandomStartingTimeFactory {
    pub(crate) fn new() -> Arc<dyn StartingTimeFactory> {
        Arc::new(RandomStartingTimeFactory {})
    }
}
