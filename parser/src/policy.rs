use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum ControlMessage {
    Nonce(u64),
    QueryPolicy,
    UpdatePolicy(Policy),
}

#[derive(Serialize, Deserialize)]
pub enum ControlMessageResponse {
    Nonce(u64),
    CurrentPolicy(Policy),
    PolicyUpdated(Policy),
}

#[derive(Serialize, Deserialize)]
pub struct Policy {
    pub sats_per_day: u64,
}
