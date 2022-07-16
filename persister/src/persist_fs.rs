use lightning_signer::persist::Persist;
use lightning_signer_server::persist::model::{ChannelEntry, NodeEntry};
use std::fs;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::ptr;
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
use serde::Deserialize;
use serde::Serialize;

use lightning_signer::persist::model::{
    ChannelEntry as CoreChannelEntry, NodeEntry as CoreNodeEntry,
};

const FAT32_MAXFILENAMESIZE: usize = 8;

const NODE: &str = "/home/ubuntu/sdcard/nodes";
const CHAN: &str = "/home/ubuntu/sdcard/channel";
const ALLO: &str = "/home/ubuntu/sdcard/allowlis";
const CHAI: &str = "/home/ubuntu/sdcard/chaintra";
const PUBS: &str = "/home/ubuntu/sdcard/pubkey";

pub struct FsPersister {
    node_path: String,
    channel_path: String,
    allowlist_path: String,
    chaintracker_path: String,
    pubkey_path: String,
}

impl FsPersister {
    pub fn new() -> Self {
        let _ = fs::create_dir(NODE);
        let _ = fs::create_dir(CHAN);
        let _ = fs::create_dir(ALLO);
        let _ = fs::create_dir(CHAI);
        let _ = fs::create_dir(PUBS);

        let mut node_path = String::with_capacity(NODE.len() + 1 + FAT32_MAXFILENAMESIZE);
        node_path.push_str(NODE);
        node_path.push_str("/");
        let mut channel_path = String::with_capacity(CHAN.len() + 2 + 2 * FAT32_MAXFILENAMESIZE);
        channel_path.push_str(CHAN);
        channel_path.push_str("/");
        let mut allowlist_path = String::with_capacity(ALLO.len() + 1 + FAT32_MAXFILENAMESIZE);
        allowlist_path.push_str(ALLO);
        allowlist_path.push_str("/");
        let mut chaintracker_path = String::with_capacity(CHAI.len() + 1 + FAT32_MAXFILENAMESIZE);
        chaintracker_path.push_str(CHAI);
        chaintracker_path.push_str("/");
        let mut pubkey_path = String::with_capacity(PUBS.len() + 1 + FAT32_MAXFILENAMESIZE);
        pubkey_path.push_str(PUBS);
        pubkey_path.push_str("/");

        Self {
            node_path,
            channel_path,
            allowlist_path,
            chaintracker_path,
            pubkey_path,
        }
    }
}

fn write<T: Serialize>(path: String, entry: T) -> anyhow::Result<()> {
    let mut buf = [0u8; 10000];
    let used = postcard::to_slice(&entry, &mut buf)?;
    println!("WROTE: {:?} BYTES", used.len());
    let file = fs::File::create(path)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(used)?;
    writer.flush()?;
    drop(writer);
    Ok(())
}

fn read<'a, T: Deserialize<'a>>(path: String, buf: &'a mut Vec<u8>) -> T {
    let file = fs::File::open(path).expect("Cannot open file");
    let mut reader = BufReader::new(file);
    reader.read_to_end(buf).expect("Could not read");
    println!("READ: {:?} BYTES", buf.len());
    postcard::from_bytes(buf).unwrap()
}

impl Persist for FsPersister {
    fn new_node(&self, node_id: &PublicKey, config: &NodeConfig, seed: &[u8]) {
        let mut node_path = self.node_path.clone();
        let mut pubkey_path = self.pubkey_path.clone();
        let filename = &hex::encode(node_id.serialize())[..FAT32_MAXFILENAMESIZE];
        node_path.push_str(filename);
        pubkey_path.push_str(filename);
        let entry = NodeEntry {
            seed: seed.to_vec(),
            key_derivation_style: config.key_derivation_style as u8,
            network: config.network.to_string(),
        };
        if let Err(e) = write(node_path, entry) {
            println!("Write error: {:?}", e);
        }
        if let Err(e) = write(pubkey_path, node_id) {
            println!("Write error: {:?}", e);
        }
    }
    fn delete_node(&self, node_id: &PublicKey) {
        let mut channel_path = self.channel_path.clone();
        let key_a = &hex::encode(node_id.serialize())[..FAT32_MAXFILENAMESIZE];
        channel_path.push_str(key_a);
        fs::remove_dir_all(channel_path).unwrap();
        let mut node_path = self.node_path.clone();
        let mut pubkey_path = self.pubkey_path.clone();
        let filename = &hex::encode(node_id.serialize())[..FAT32_MAXFILENAMESIZE];
        node_path.push_str(filename);
        pubkey_path.push_str(filename);
        fs::remove_file(node_path).unwrap();
        fs::remove_file(pubkey_path).unwrap();
    }
    fn new_channel(&self, node_id: &PublicKey, stub: &ChannelStub) -> Result<(), ()> {
        let mut channel_path = self.channel_path.clone();
        let key_a = &hex::encode(node_id.serialize())[..FAT32_MAXFILENAMESIZE];
        channel_path.push_str(&key_a);
        fs::create_dir(channel_path.clone()).expect("Problem creating a directory");
        channel_path.push_str("/");

        let id = NodeChannelId::new(node_id, &stub.id0);
        channel_path.push_str(&hex::encode(
            &id.channel_id().as_slice()[..FAT32_MAXFILENAMESIZE],
        ));
        let channel_value_satoshis = 0;
        let entry = ChannelEntry {
            channel_value_satoshis,
            channel_setup: None,
            id: Some(id.channel_id()),
            enforcement_state: EnforcementState::new(0),
        };
        if let Err(e) = write(channel_path, entry) {
            println!("Write error: {:?}", e);
        }
        Ok(())
    }
    fn new_chain_tracker(&self, node_id: &PublicKey, tracker: &ChainTracker<ChainMonitor>) {
        let mut chaintracker_path = self.chaintracker_path.clone();
        chaintracker_path.push_str(&hex::encode(node_id.serialize())[..FAT32_MAXFILENAMESIZE]);
        let entry: ChainTrackerEntry = tracker.into();
        if let Err(e) = write(chaintracker_path, entry) {
            println!("Write error: {:?}", e);
        }
    }
    fn update_tracker(
        &self,
        node_id: &PublicKey,
        tracker: &ChainTracker<ChainMonitor>,
    ) -> Result<(), ()> {
        let mut chaintracker_path = self.chaintracker_path.clone();
        chaintracker_path.push_str(&hex::encode(node_id.serialize())[..FAT32_MAXFILENAMESIZE]);
        let entry: ChainTrackerEntry = tracker.into();
        if let Err(e) = write(chaintracker_path, entry) {
            println!("Write error: {:?}", e);
        }
        Ok(())
    }
    fn get_tracker(&self, node_id: &PublicKey) -> Result<ChainTracker<ChainMonitor>, ()> {
        let mut chaintracker_path = self.chaintracker_path.clone();
        chaintracker_path.push_str(&hex::encode(node_id.serialize())[..FAT32_MAXFILENAMESIZE]);
        let mut buf = Vec::new();
        let ret: ChainTrackerEntry = read(chaintracker_path, &mut buf);
        Ok(ret.into())
    }
    fn update_channel(&self, node_id: &PublicKey, channel: &Channel) -> Result<(), ()> {
        println!("UPDATING CHANNEL: {:?}", channel.id);
        let channel_value_satoshis = channel.setup.channel_value_sat;
        let mut channel_path = self.channel_path.clone();
        let key_a = &hex::encode(node_id.serialize())[..FAT32_MAXFILENAMESIZE];
        channel_path.push_str(&key_a);
        channel_path.push_str("/");

        let id = NodeChannelId::new(node_id, &channel.id0);
        channel_path.push_str(&hex::encode(
            &id.channel_id().as_slice()[..FAT32_MAXFILENAMESIZE],
        ));
        let entry = ChannelEntry {
            channel_value_satoshis,
            channel_setup: Some(channel.setup.clone()),
            //id: channel.id.clone(),
            id: if channel.id.is_none() {
                Some(id.channel_id())
            } else {
                channel.id.clone()
            },
            enforcement_state: channel.enforcement_state.clone(),
        };
        if let Err(e) = write(channel_path, entry) {
            println!("Write error: {:?}", e);
        }
        Ok(())
    }
    fn get_channel(
        &self,
        node_id: &PublicKey,
        channel_id: &ChannelId,
    ) -> Result<CoreChannelEntry, ()> {
        let mut channel_path = self.channel_path.clone();
        let key_a = &hex::encode(node_id.serialize())[..FAT32_MAXFILENAMESIZE];
        channel_path.push_str(&key_a);
        channel_path.push_str("/");
        let id = NodeChannelId::new(node_id, channel_id);
        channel_path.push_str(&hex::encode(
            &id.channel_id().as_slice()[..FAT32_MAXFILENAMESIZE],
        ));
        let mut buf = Vec::new();
        let ret: ChannelEntry = read(channel_path, &mut buf);
        Ok(ret.into())
    }
    fn get_node_channels(&self, node_id: &PublicKey) -> Vec<(ChannelId, CoreChannelEntry)> {
        let mut res = Vec::new();
        let mut channel_path = self.channel_path.clone();
        let key_a = &hex::encode(node_id.serialize())[..FAT32_MAXFILENAMESIZE];
        channel_path.push_str(&key_a);
        if !Path::new(&channel_path).exists() {
            return res;
        }
        for channel in fs::read_dir(channel_path).unwrap() {
            let channel = channel.unwrap();
            let mut buf = Vec::new();
            let entry: ChannelEntry = read(channel.path().to_str().unwrap().to_string(), &mut buf);
            let id = entry.id.clone().unwrap();
            res.push((id, entry.into()))
        }
        res
    }
    fn update_node_allowlist(&self, node_id: &PublicKey, allowlist: Vec<String>) -> Result<(), ()> {
        let mut allowlist_path = self.allowlist_path.clone();
        allowlist_path.push_str(&hex::encode(node_id.serialize())[..FAT32_MAXFILENAMESIZE]);
        let entry = AllowlistItemEntry { allowlist };
        if let Err(e) = write(allowlist_path, entry) {
            println!("Write error: {:?}", e);
        }
        Ok(())
    }
    fn get_node_allowlist(&self, node_id: &PublicKey) -> Vec<String> {
        let mut allowlist_path = self.allowlist_path.clone();
        allowlist_path.push_str(&hex::encode(node_id.serialize())[..FAT32_MAXFILENAMESIZE]);
        let mut buf = Vec::new();
        let entry: AllowlistItemEntry = read(allowlist_path, &mut buf);
        entry.allowlist
    }
    fn get_nodes(&self) -> Vec<(PublicKey, CoreNodeEntry)> {
        let mut res = Vec::new();
        let pubkey_path = self.pubkey_path.clone();
        let node_path = self.node_path.clone();
        for (pubkey, node) in fs::read_dir(pubkey_path)
            .unwrap()
            .zip(fs::read_dir(node_path).unwrap())
        {
            let node = node.unwrap();
            let pubkey = pubkey.unwrap();
            let mut buf_a = Vec::new();
            let mut buf_b = Vec::new();
            let pubkey: PublicKey = read(pubkey.path().to_str().unwrap().to_string(), &mut buf_a);
            let entry: NodeEntry = read(node.path().to_str().unwrap().to_string(), &mut buf_b);
            res.push((pubkey, entry.into()))
        }
        res
    }
    fn clear_database(&self) {
        fs::remove_dir_all(NODE).unwrap();
        fs::remove_dir_all(CHAN).unwrap();
        fs::remove_dir_all(ALLO).unwrap();
        fs::remove_dir_all(CHAI).unwrap();
        fs::remove_dir_all(PUBS).unwrap();
    }
}
