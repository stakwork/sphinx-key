use fsdb::{Bucket, Fsdb};
use lightning_signer::persist::Persist;
use lightning_signer_server::persist::model::{ChannelEntry, NodeEntry};
use std::string::String;

use lightning_signer::bitcoin::secp256k1::PublicKey;
use lightning_signer::chain::tracker::ChainTracker;
use lightning_signer::channel::Channel;
use lightning_signer::channel::ChannelId;
use lightning_signer::channel::ChannelStub;
use lightning_signer::monitor::ChainMonitor;
use lightning_signer::node::NodeConfig;
use lightning_signer::policy::validator::EnforcementState;
use lightning_signer_server::persist::model::AllowlistItemEntry;
use lightning_signer_server::persist::model::ChainTrackerEntry;
use lightning_signer_server::persist::model::NodeChannelId;

use lightning_signer::persist::model::{
    ChannelEntry as CoreChannelEntry, NodeEntry as CoreNodeEntry,
};

const FAT32_MAXFILENAMESIZE: usize = 8;

pub struct FsPersister {
    nodes: Bucket<NodeEntry>,
    channels: Bucket<ChannelEntry>,
    allowlist: Bucket<AllowlistItemEntry>,
    chaintracker: Bucket<ChainTrackerEntry>,
    pubkeys: Bucket<PublicKey>,
}

impl FsPersister {
    pub fn new() -> Self {
        let db = Fsdb::new("home/ubuntu/sdcard").expect("could not create db");
        let mut nodes = db.bucket("nodes").expect("fail nodes");
        nodes.set_max_file_name(FAT32_MAXFILENAMESIZE);
        let mut channels = db.bucket("channel").expect("fail channel");
        channels.set_max_file_name(FAT32_MAXFILENAMESIZE);
        let mut allowlist = db.bucket("allowlis").expect("fail allowlis");
        allowlist.set_max_file_name(FAT32_MAXFILENAMESIZE);
        let mut chaintracker = db.bucket("chaintra").expect("fail chaintra");
        chaintracker.set_max_file_name(FAT32_MAXFILENAMESIZE);
        let mut pubkeys = db.bucket("pubkey").expect("fail pubkey");
        pubkeys.set_max_file_name(FAT32_MAXFILENAMESIZE);
        Self {
            nodes,
            channels,
            allowlist,
            chaintracker,
            pubkeys,
        }
    }
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
        let _ = self.channels.clear_within(&pk);
        let _ = self.nodes.remove(&pk);
        let _ = self.pubkeys.remove(&pk);
    }
    fn new_channel(&self, node_id: &PublicKey, stub: &ChannelStub) -> Result<(), ()> {
        let pk = hex::encode(node_id.serialize());
        let id = NodeChannelId::new(node_id, &stub.id0);
        let chan_id = hex::encode(id.channel_id().as_slice());
        let entry = ChannelEntry {
            channel_value_satoshis: 0,
            channel_setup: None,
            id: Some(id.channel_id()),
            enforcement_state: EnforcementState::new(0),
        };
        let _ = self.channels.put_within(&chan_id, entry, &pk);
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
        let pk = hex::encode(node_id.serialize());
        let _ = self.chaintracker.put(&pk, tracker.into());
        Ok(())
    }
    fn get_tracker(&self, node_id: &PublicKey) -> Result<ChainTracker<ChainMonitor>, ()> {
        let pk = hex::encode(node_id.serialize());
        let ret: ChainTrackerEntry = match self.chaintracker.get(&pk) {
            Ok(ct) => ct,
            Err(_) => return Err(()),
        };
        Ok(ret.into())
    }
    fn update_channel(&self, node_id: &PublicKey, channel: &Channel) -> Result<(), ()> {
        let pk = hex::encode(node_id.serialize());
        let id = NodeChannelId::new(node_id, &channel.id0);
        let chan_id = hex::encode(id.channel_id().as_slice());
        let entry = ChannelEntry {
            id: if channel.id.is_none() {
                Some(id.channel_id())
            } else {
                channel.id.clone()
            },
            channel_value_satoshis: channel.setup.channel_value_sat,
            channel_setup: Some(channel.setup.clone()),
            enforcement_state: channel.enforcement_state.clone(),
        };
        let _ = self.channels.put_within(&chan_id, entry, &pk);
        Ok(())
    }
    fn get_channel(
        &self,
        node_id: &PublicKey,
        channel_id: &ChannelId,
    ) -> Result<CoreChannelEntry, ()> {
        let pk = hex::encode(node_id.serialize());
        let id = NodeChannelId::new(node_id, channel_id);
        let chan_id = hex::encode(id.channel_id().as_slice());
        let ret: ChannelEntry = match self.channels.get_within(&chan_id, &pk) {
            Ok(ce) => ce,
            Err(_) => return Err(()),
        };
        Ok(ret.into())
    }
    fn get_node_channels(&self, node_id: &PublicKey) -> Vec<(ChannelId, CoreChannelEntry)> {
        let mut res = Vec::new();
        let pk = hex::encode(node_id.serialize());
        let list = match self.channels.list_within(&pk) {
            Ok(l) => l,
            Err(_) => return res,
        };
        for channel in list {
            if let Ok(entry) = self.channels.get_within(&channel, &pk) {
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
        let _ = self.channels.clear();
        let _ = self.allowlist.clear();
        let _ = self.chaintracker.clear();
        let _ = self.pubkeys.clear();
    }
}
