use lightning_signer::policy::filter::PolicyFilter;
use lightning_signer::policy::simple_validator::SimplePolicy;
use std::convert::From;
use vls_protocol_signer::lightning_signer;

pub struct Policy {
    pub max_htlc_value_sat: u64,
}

impl From<Policy> for SimplePolicy {
    fn from(p: Policy) -> Self {
        SimplePolicy {
            min_delay: 144,  // LDK min
            max_delay: 2016, // LDK max
            max_channel_size_sat: 1_000_000_001,
            epsilon_sat: 10_000,
            max_htlcs: 1000,
            max_htlc_value_sat: p.max_htlc_value_sat,
            use_chain_state: false,
            min_feerate_per_kw: 253,    // mainnet observed
            max_feerate_per_kw: 25_000, // equiv to 100 sat/vb
            require_invoices: false,
            enforce_balance: false,
            max_routing_fee_msat: 10000,
            dev_flags: None,
            filter: PolicyFilter::default(),
        }
    }
}
