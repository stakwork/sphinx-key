use lightning_signer::policy::filter::PolicyFilter;
use lightning_signer::policy::simple_validator::{
    make_simple_policy, SimplePolicy, SimpleValidatorFactory,
};
use lightning_signer::util::velocity::{VelocityControlIntervalType, VelocityControlSpec};
use sphinx_key_parser::control::{Interval, Policy};
use std::sync::Arc;
use vls_protocol_signer::handler::RootHandler;
use vls_protocol_signer::lightning_signer;
use vls_protocol_signer::lightning_signer::bitcoin::Network;

fn policy_interval(int: Interval) -> VelocityControlIntervalType {
    match int {
        Interval::Hourly => VelocityControlIntervalType::Hourly,
        Interval::Daily => VelocityControlIntervalType::Daily,
    }
}

pub fn set_policy(root_handler: &RootHandler, network: Network, po: Policy) -> anyhow::Result<()> {
    let policy = make_policy(network, &po);
    let validator_factory = Arc::new(SimpleValidatorFactory::new_with_policy(policy));
    root_handler.node.set_validator_factory(validator_factory);
    Ok(())
}

pub fn set_allowlist(root_handler: &RootHandler, allowlist: &Vec<String>) -> anyhow::Result<()> {
    if let Err(e) = root_handler.node.set_allowlist(allowlist) {
        return Err(anyhow::anyhow!("error setting allowlist {:?}", e));
    }
    Ok(())
}

pub fn get_allowlist(root_handler: &RootHandler) -> anyhow::Result<Vec<String>> {
    match root_handler.node.allowlist() {
        Ok(al) => Ok(al),
        Err(e) => Err(anyhow::anyhow!("error setting allowlist {:?}", e)),
    }
}

pub fn make_policy(network: Network, po: &Policy) -> SimplePolicy {
    let mut p = make_simple_policy(network);
    p.max_htlc_value_sat = po.htlc_limit;
    p.filter = PolicyFilter::new_permissive();
    let velocity_spec = VelocityControlSpec {
        limit: po.sat_limit,
        interval_type: policy_interval(po.interval),
    };
    p.global_velocity_control = velocity_spec;
    p
}
