use fsdb::{Bucket, DoubleBucket, Fsdb};
use lightning_signer::bitcoin::secp256k1::PublicKey;
use lightning_signer::chain::tracker::ChainTracker;
use lightning_signer::channel::{Channel, ChannelId, ChannelStub};
use lightning_signer::monitor::ChainMonitor;
use lightning_signer::node::NodeConfig;
use lightning_signer::persist::Persist;
use lightning_signer::policy::validator::EnforcementState;
use lightning_signer_server::persist::model::{
    AllowlistItemEntry, ChainTrackerEntry, ChannelEntry, NodeEntry,
};
use std::string::String;

use lightning_signer::persist::model::{
    ChannelEntry as CoreChannelEntry, NodeEntry as CoreNodeEntry,
};

const FAT32_MAXFILENAMESIZE: usize = 8;

pub struct FsPersister {
    nodes: Bucket<NodeEntry>,
    channels: DoubleBucket<ChannelEntry>,
    allowlist: Bucket<AllowlistItemEntry>,
    chaintracker: Bucket<ChainTrackerEntry>,
    pubkeys: Bucket<PublicKey>,
}

impl FsPersister {
    pub fn new(dir: &str) -> Self {
        let db = Fsdb::new(dir).expect("could not create db");
        let max = Some(FAT32_MAXFILENAMESIZE);
        Self {
            nodes: db.bucket("nodes", max).expect("fail nodes"),
            channels: db.double_bucket("channel", max).expect("fail channel"),
            allowlist: db.bucket("allowlis", max).expect("fail allowlis"),
            chaintracker: db.bucket("chaintra", max).expect("fail chaintra"),
            pubkeys: db.bucket("pubkey", max).expect("fail pubkey"),
        }
    }
}

fn get_channel_key(channel_id: &[u8]) -> &[u8] {
    let length = channel_id.len();
    channel_id.get(length - 11..length - 7).unwrap()
}

impl Persist for FsPersister {
    fn new_node(&self, node_id: &PublicKey, config: &NodeConfig, seed: &[u8]) {
        let pk = hex::encode(node_id.serialize());
        let entry = NodeEntry {
            seed: seed.to_vec(),
            key_derivation_style: config.key_derivation_style as u8,
            network: config.network.to_string(),
        };
        let _ = self.nodes.put(&pk, entry);
        let _ = self.pubkeys.put(&pk, node_id.clone());
    }
    fn delete_node(&self, node_id: &PublicKey) {
        let pk = hex::encode(node_id.serialize());
        // clear all channel entries within "pk" sub-bucket
        let _ = self.channels.clear(&pk);
        let _ = self.nodes.remove(&pk);
        let _ = self.pubkeys.remove(&pk);
    }
    fn new_channel(&self, node_id: &PublicKey, stub: &ChannelStub) -> Result<(), ()> {
        let pk = hex::encode(node_id.serialize());
        let chan_id = hex::encode(get_channel_key(stub.id0.as_slice()));
        // this breaks things...
        // if let Ok(_) = self.channels.get(&pk, &chan_id) {
        //     log::error!("persister: failed to create new_channel: already exists");
        //     // return Err(()); // already exists
        // }
        let entry = ChannelEntry {
            id: Some(stub.id0.clone()),
            channel_value_satoshis: 0,
            channel_setup: None,
            enforcement_state: EnforcementState::new(0),
        };
        let _ = self.channels.put(&pk, &chan_id, entry);
        Ok(())
    }
    fn new_chain_tracker(&self, node_id: &PublicKey, tracker: &ChainTracker<ChainMonitor>) {
        let pk = hex::encode(node_id.serialize());
        let _ = self.chaintracker.put(&pk, tracker.into());
    }
    fn update_tracker(
        &self,
        node_id: &PublicKey,
        tracker: &ChainTracker<ChainMonitor>,
    ) -> Result<(), ()> {
        log::info!("=> update_tracker");
        let pk = hex::encode(node_id.serialize());
        let _ = self.chaintracker.put(&pk, tracker.into());
        log::info!("=> update_tracker complete");
        Ok(())
    }
    fn get_tracker(&self, node_id: &PublicKey) -> Result<ChainTracker<ChainMonitor>, ()> {
        let pk = hex::encode(node_id.serialize());
        let ret: ChainTrackerEntry = match self.chaintracker.get(&pk) {
            Ok(ct) => ct,
            Err(_) => {
                log::error!("persister: failed to get_tracker");
                return Err(());
            }
        };
        Ok(ret.into())
    }
    fn update_channel(&self, node_id: &PublicKey, channel: &Channel) -> Result<(), ()> {
        log::info!("=> update_channel");
        let pk = hex::encode(node_id.serialize());
        let chan_id = hex::encode(get_channel_key(channel.id0.as_slice()));
        // this breaks things...
        // if let Err(_) = self.channels.get(&pk, &chan_id) {
        //     log::error!("persister: failed to update_channel");
        //     // return Err(()); // not found
        // }
        let entry = ChannelEntry {
            id: Some(channel.id0.clone()),
            channel_value_satoshis: channel.setup.channel_value_sat,
            channel_setup: Some(channel.setup.clone()),
            enforcement_state: channel.enforcement_state.clone(),
        };
        let _ = self.channels.put(&pk, &chan_id, entry);
        log::info!("=> update_channel complete!");
        Ok(())
    }
    fn get_channel(
        &self,
        node_id: &PublicKey,
        channel_id: &ChannelId,
    ) -> Result<CoreChannelEntry, ()> {
        let pk = hex::encode(node_id.serialize());
        let chan_id = hex::encode(get_channel_key(channel_id.as_slice()));
        let ret: ChannelEntry = match self.channels.get(&pk, &chan_id) {
            Ok(ce) => ce,
            Err(_) => {
                log::error!("persister: failed to get_channel");
                return Err(());
            }
        };
        Ok(ret.into())
    }
    fn get_node_channels(&self, node_id: &PublicKey) -> Vec<(ChannelId, CoreChannelEntry)> {
        let mut res = Vec::new();
        let pk = hex::encode(node_id.serialize());
        let list = match self.channels.list(&pk) {
            Ok(l) => l,
            Err(_) => return res,
        };
        for channel in list {
            if let Ok(entry) = self.channels.get(&pk, &channel) {
                let id = entry.id.clone().unwrap();
                res.push((id, entry.into()));
            };
        }
        res
    }
    fn update_node_allowlist(&self, node_id: &PublicKey, allowlist: Vec<String>) -> Result<(), ()> {
        let pk = hex::encode(node_id.serialize());
        let entry = AllowlistItemEntry { allowlist };
        let _ = self.allowlist.put(&pk, entry);
        Ok(())
    }
    fn get_node_allowlist(&self, node_id: &PublicKey) -> Vec<String> {
        let pk = hex::encode(node_id.serialize());
        let entry: AllowlistItemEntry = match self.allowlist.get(&pk) {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };
        entry.allowlist
    }
    fn get_nodes(&self) -> Vec<(PublicKey, CoreNodeEntry)> {
        let mut res = Vec::new();
        let list = match self.nodes.list() {
            Ok(ns) => ns,
            Err(_) => return res,
        };
        log::info!("NODE LIST LEN {}", list.len());
        for pk in list {
            if let Ok(pubkey) = self.pubkeys.get(&pk) {
                if let Ok(node) = self.nodes.get(&pk) {
                    res.push((pubkey, node.into()));
                }
            }
        }
        res
    }
    fn clear_database(&self) {
        let _ = self.nodes.clear();
        let _ = self.channels.clear_all();
        let _ = self.allowlist.clear();
        let _ = self.chaintracker.clear();
        let _ = self.pubkeys.clear();
    }
}
