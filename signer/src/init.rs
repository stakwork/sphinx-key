use crate::policy::Policy;

use lightning_signer::node::{Node, NodeConfig};
use lightning_signer::persist::Persist;
use lightning_signer::policy::simple_validator::SimpleValidatorFactory;
use lightning_signer::signer::derive::KeyDerivationStyle;
use std::sync::Arc;
use vls_protocol_signer::handler::RootHandler;
use vls_protocol_signer::lightning_signer;
use vls_protocol_signer::lightning_signer::bitcoin::Network;

pub fn new_root_handler_with_policy(
    network: Network,
    id: u64,
    seed: [u8; 32],
    persister: Arc<dyn Persist>,
    allowlist: Vec<String>,
    policy: Policy,
) -> RootHandler {
    let config = NodeConfig {
        network,
        key_derivation_style: KeyDerivationStyle::Native,
    };

    let nodes = persister.get_nodes();
    // let policy = make_simple_policy(network);
    // [permissive mode]
    // policy.filter = PolicyFilter::new_permissive();

    let validator_factory = Arc::new(SimpleValidatorFactory::new_with_policy(policy.into()));
    let node = if nodes.is_empty() {
        let node = Arc::new(Node::new(
            config,
            &seed,
            &persister,
            vec![],
            validator_factory,
        ));
        log::info!("New node {}", node.get_id());
        node.add_allowlist(&allowlist).expect("allowlist");
        persister.new_node(&node.get_id(), &config, &seed);
        persister.new_chain_tracker(&node.get_id(), &node.get_tracker());
        node
    } else {
        assert_eq!(nodes.len(), 1);
        let (node_id, entry) = nodes.into_iter().next().unwrap();
        log::info!("Restore node {}", node_id);
        Node::restore_node(&node_id, entry, persister, validator_factory)
    };

    RootHandler { id, node }
}
