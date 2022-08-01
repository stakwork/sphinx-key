use lightning_signer::policy::filter::PolicyFilter;
use lightning_signer::policy::simple_validator::SimplePolicy;
use lightning_signer::policy::simple_validator::SimpleValidatorFactory;
use serde::{Deserialize, Serialize};
use vls_protocol_signer::lightning_signer;
use vls_protocol_signer::lightning_signer::bitcoin::Network;

#[derive(Serialize, Deserialize)]
pub struct Policy {
    pub max_htlc_value_sat: u64,
}

pub fn make_factory(policy: Policy, network: Network) -> SimpleValidatorFactory {
    SimpleValidatorFactory::new_with_policy(make_policy(policy, network))
}

pub fn make_policy(p: Policy, network: Network) -> SimplePolicy {
    if network == Network::Bitcoin {
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
    } else {
        SimplePolicy {
            min_delay: 4,
            max_delay: 2016,                     // Match LDK maximum and default
            max_channel_size_sat: 1_000_000_001, // lnd itest: wumbu default + 1
            // lnd itest: async_bidirectional_payments (large amount of dust HTLCs) 1_600_000
            epsilon_sat: 10_000, // c-lightning
            max_htlcs: 1000,
            max_htlc_value_sat: p.max_htlc_value_sat,
            use_chain_state: false,
            min_feerate_per_kw: 253, // testnet/regtest observed
            max_feerate_per_kw: 100_000,
            require_invoices: false,
            enforce_balance: false,
            max_routing_fee_msat: 10000,
            dev_flags: None,
            filter: PolicyFilter::default(),
        }
    }
}
