//! By convention, structs ending with `Def` are serde local types
//! describing how to serialize a remote type via `serde(remote)`.
//! Structs ending with `Entry` are local types that require a manual
//! transformation from the remote type - implemented via `From` / `Into`.

use std::borrow::Cow;
use std::collections::BTreeSet as Set;

use lightning::ln::chan_utils::ChannelPublicKeys;
use lightning::ln::PaymentHash;
use lightning::util::ser::Writer;
use lightning_signer::bitcoin::hashes::Hash;
use lightning_signer::bitcoin::secp256k1::PublicKey;
use lightning_signer::bitcoin::{OutPoint, Script, Txid};
use lightning_signer::chain::tracker::ListenSlot;
use lightning_signer::lightning;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::serde_as;
use serde_with::{DeserializeAs, SerializeAs};

use lightning_signer::channel::{ChannelId, ChannelSetup, CommitmentType};
use lightning_signer::monitor::State as ChainMonitorState;
use lightning_signer::policy::validator::EnforcementState;
use lightning_signer::tx::tx::{CommitmentInfo2, HTLCInfo2};

#[derive(Copy, Clone, Debug, Default)]
pub struct PublicKeyHandler;

impl SerializeAs<PublicKey> for PublicKeyHandler {
    fn serialize_as<S>(source: &PublicKey, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(hex::encode(source.serialize().to_vec()).as_str())
    }
}

impl<'de> DeserializeAs<'de, PublicKey> for PublicKeyHandler {
    fn deserialize_as<D>(deserializer: D) -> Result<PublicKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let res = <Cow<'de, str> as Deserialize<'de>>::deserialize(deserializer).unwrap();
        let key = PublicKey::from_slice(hex::decode(&*res).unwrap().as_slice()).unwrap();
        Ok(key)
    }
}

pub struct ChannelIdHandler;

impl SerializeAs<ChannelId> for ChannelIdHandler {
    fn serialize_as<S>(source: &ChannelId, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(hex::encode(source.as_slice()).as_str())
    }
}

impl<'de> DeserializeAs<'de, ChannelId> for ChannelIdHandler {
    fn deserialize_as<D>(deserializer: D) -> Result<ChannelId, D::Error>
    where
        D: Deserializer<'de>,
    {
        let res = <Cow<'de, str> as Deserialize<'de>>::deserialize(deserializer).unwrap();
        let key = ChannelId::new(&hex::decode(&*res).unwrap());
        Ok(key)
    }
}

#[serde_as]
#[derive(Serialize, Deserialize)]
#[serde(remote = "ChannelPublicKeys")]
pub struct ChannelPublicKeysDef {
    #[serde_as(as = "PublicKeyHandler")]
    pub funding_pubkey: PublicKey,
    #[serde_as(as = "PublicKeyHandler")]
    pub revocation_basepoint: PublicKey,
    #[serde_as(as = "PublicKeyHandler")]
    pub payment_point: PublicKey,
    #[serde_as(as = "PublicKeyHandler")]
    pub delayed_payment_basepoint: PublicKey,
    #[serde_as(as = "PublicKeyHandler")]
    pub htlc_basepoint: PublicKey,
}

#[derive(Deserialize)]
struct ChannelPublicKeysHelper(#[serde(with = "ChannelPublicKeysDef")] ChannelPublicKeys);

impl SerializeAs<ChannelPublicKeys> for ChannelPublicKeysDef {
    fn serialize_as<S>(value: &ChannelPublicKeys, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ChannelPublicKeysDef::serialize(value, serializer)
    }
}

impl<'de> DeserializeAs<'de, ChannelPublicKeys> for ChannelPublicKeysDef {
    fn deserialize_as<D>(
        deserializer: D,
    ) -> Result<ChannelPublicKeys, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        ChannelPublicKeysHelper::deserialize(deserializer).map(|h| h.0)
    }
}

pub struct VecWriter(pub Vec<u8>);
impl Writer for VecWriter {
    fn write_all(&mut self, buf: &[u8]) -> Result<(), ::std::io::Error> {
        self.0.extend_from_slice(buf);
        Ok(())
    }
}

struct TxidDef;

impl SerializeAs<Txid> for TxidDef {
    fn serialize_as<S>(value: &Txid, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(hex::encode(value.to_vec()).as_str())
    }
}

impl<'de> DeserializeAs<'de, Txid> for TxidDef {
    fn deserialize_as<D>(deserializer: D) -> Result<Txid, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let res = <Cow<'de, str> as Deserialize<'de>>::deserialize(deserializer).unwrap();
        let txid = Txid::from_slice(hex::decode(&*res).unwrap().as_slice()).unwrap();
        Ok(txid)
    }
}

#[serde_as]
#[derive(Serialize, Deserialize)]
#[serde(remote = "OutPoint")]
pub struct OutPointDef {
    #[serde_as(as = "TxidDef")]
    pub txid: Txid,
    pub vout: u32,
}

#[derive(Deserialize)]
struct OutPointHelper(#[serde(with = "OutPointDef")] OutPoint);

impl SerializeAs<OutPoint> for OutPointDef {
    fn serialize_as<S>(value: &OutPoint, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        OutPointDef::serialize(value, serializer)
    }
}

impl<'de> DeserializeAs<'de, OutPoint> for OutPointDef {
    fn deserialize_as<D>(deserializer: D) -> Result<OutPoint, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        OutPointHelper::deserialize(deserializer).map(|h| h.0)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "CommitmentType")]
pub enum CommitmentTypeDef {
    Legacy,
    StaticRemoteKey,
    Anchors,
}

#[derive(Deserialize)]
struct CommitmentTypeHelper(#[serde(with = "CommitmentTypeDef")] CommitmentType);

impl SerializeAs<CommitmentType> for CommitmentTypeDef {
    fn serialize_as<S>(value: &CommitmentType, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        CommitmentTypeDef::serialize(value, serializer)
    }
}

impl<'de> DeserializeAs<'de, CommitmentType> for CommitmentTypeDef {
    fn deserialize_as<D>(deserializer: D) -> Result<CommitmentType, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        CommitmentTypeHelper::deserialize(deserializer).map(|h| h.0)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Script")]
pub struct ScriptDef(#[serde(getter = "Script::to_bytes")] Vec<u8>);

impl From<ScriptDef> for Script {
    fn from(s: ScriptDef) -> Self {
        Script::from(s.0)
    }
}

#[derive(Deserialize)]
struct ScriptHelper(#[serde(with = "ScriptDef")] Script);

impl SerializeAs<Script> for ScriptDef {
    fn serialize_as<S>(value: &Script, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ScriptDef::serialize(value, serializer)
    }
}

impl<'de> DeserializeAs<'de, Script> for ScriptDef {
    fn deserialize_as<D>(deserializer: D) -> Result<Script, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        ScriptHelper::deserialize(deserializer).map(|h| h.0)
    }
}

#[serde_as]
#[derive(Serialize, Deserialize)]
#[serde(remote = "ChannelSetup")]
pub struct ChannelSetupDef {
    pub is_outbound: bool,
    pub channel_value_sat: u64,
    pub push_value_msat: u64,
    #[serde_as(as = "OutPointDef")]
    pub funding_outpoint: OutPoint,
    pub holder_selected_contest_delay: u16,
    #[serde_as(as = "Option<ScriptDef>")]
    pub holder_shutdown_script: Option<Script>,
    #[serde(with = "ChannelPublicKeysDef")]
    pub counterparty_points: ChannelPublicKeys,
    pub counterparty_selected_contest_delay: u16,
    #[serde_as(as = "Option<ScriptDef>")]
    pub counterparty_shutdown_script: Option<Script>,
    #[serde_as(as = "CommitmentTypeDef")]
    pub commitment_type: CommitmentType,
}

#[derive(Deserialize)]
struct ChannelSetupHelper(#[serde(with = "ChannelSetupDef")] ChannelSetup);

impl SerializeAs<ChannelSetup> for ChannelSetupDef {
    fn serialize_as<S>(value: &ChannelSetup, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ChannelSetupDef::serialize(value, serializer)
    }
}

impl<'de> DeserializeAs<'de, ChannelSetup> for ChannelSetupDef {
    fn deserialize_as<D>(deserializer: D) -> Result<ChannelSetup, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        ChannelSetupHelper::deserialize(deserializer).map(|h| h.0)
    }
}

#[serde_as]
#[derive(Serialize, Deserialize)]
#[serde(remote = "PaymentHash")]
pub struct PaymentHashDef(pub [u8; 32]);

#[derive(Deserialize)]
struct PaymentHashHelper(#[serde(with = "PaymentHashDef")] PaymentHash);

impl SerializeAs<PaymentHash> for PaymentHashDef {
    fn serialize_as<S>(value: &PaymentHash, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        PaymentHashDef::serialize(value, serializer)
    }
}

impl<'de> DeserializeAs<'de, PaymentHash> for PaymentHashDef {
    fn deserialize_as<D>(deserializer: D) -> Result<PaymentHash, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        PaymentHashHelper::deserialize(deserializer).map(|h| h.0)
    }
}

#[serde_as]
#[derive(Serialize, Deserialize)]
#[serde(remote = "HTLCInfo2")]
pub struct HTLCInfo2Def {
    pub value_sat: u64,
    #[serde_as(as = "PaymentHashDef")]
    pub payment_hash: PaymentHash,
    pub cltv_expiry: u32,
}

#[derive(Deserialize)]
struct HTLCInfo2Helper(#[serde(with = "HTLCInfo2Def")] HTLCInfo2);

impl SerializeAs<HTLCInfo2> for HTLCInfo2Def {
    fn serialize_as<S>(value: &HTLCInfo2, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        HTLCInfo2Def::serialize(value, serializer)
    }
}

impl<'de> DeserializeAs<'de, HTLCInfo2> for HTLCInfo2Def {
    fn deserialize_as<D>(deserializer: D) -> Result<HTLCInfo2, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        HTLCInfo2Helper::deserialize(deserializer).map(|h| h.0)
    }
}

#[serde_as]
#[derive(Serialize, Deserialize)]
#[serde(remote = "CommitmentInfo2")]
pub struct CommitmentInfo2Def {
    pub is_counterparty_broadcaster: bool,
    pub to_countersigner_pubkey: PublicKey,
    pub to_countersigner_value_sat: u64,
    pub revocation_pubkey: PublicKey,
    pub to_broadcaster_delayed_pubkey: PublicKey,
    pub to_broadcaster_value_sat: u64,
    pub to_self_delay: u16,
    #[serde_as(as = "Vec<HTLCInfo2Def>")]
    pub offered_htlcs: Vec<HTLCInfo2>,
    #[serde_as(as = "Vec<HTLCInfo2Def>")]
    pub received_htlcs: Vec<HTLCInfo2>,
    #[serde(default)] // TODO remove default once everybody upgraded
    pub feerate_per_kw: u32,
}

#[derive(Deserialize)]
struct CommitmentInfo2Helper(#[serde(with = "CommitmentInfo2Def")] CommitmentInfo2);

impl SerializeAs<CommitmentInfo2> for CommitmentInfo2Def {
    fn serialize_as<S>(value: &CommitmentInfo2, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        CommitmentInfo2Def::serialize(value, serializer)
    }
}

impl<'de> DeserializeAs<'de, CommitmentInfo2> for CommitmentInfo2Def {
    fn deserialize_as<D>(
        deserializer: D,
    ) -> Result<CommitmentInfo2, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        CommitmentInfo2Helper::deserialize(deserializer).map(|h| h.0)
    }
}

#[serde_as]
#[derive(Serialize, Deserialize)]
#[serde(remote = "EnforcementState")]
pub struct EnforcementStateDef {
    pub next_holder_commit_num: u64,
    pub next_counterparty_commit_num: u64,
    pub next_counterparty_revoke_num: u64,
    pub current_counterparty_point: Option<PublicKey>,
    pub previous_counterparty_point: Option<PublicKey>,
    #[serde_as(as = "Option<CommitmentInfo2Def>")]
    pub current_holder_commit_info: Option<CommitmentInfo2>,
    #[serde_as(as = "Option<CommitmentInfo2Def>")]
    pub current_counterparty_commit_info: Option<CommitmentInfo2>,
    #[serde_as(as = "Option<CommitmentInfo2Def>")]
    pub previous_counterparty_commit_info: Option<CommitmentInfo2>,
    pub channel_closed: bool,
    #[serde(default)] // TODO remove default once everyone upgrades
    pub initial_holder_value: u64,
}

#[derive(Deserialize)]
struct EnforcementStateHelper(#[serde(with = "EnforcementStateDef")] EnforcementState);

impl SerializeAs<EnforcementState> for EnforcementStateDef {
    fn serialize_as<S>(value: &EnforcementState, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        EnforcementStateDef::serialize(value, serializer)
    }
}

impl<'de> DeserializeAs<'de, EnforcementState> for EnforcementStateDef {
    fn deserialize_as<D>(
        deserializer: D,
    ) -> Result<EnforcementState, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        EnforcementStateHelper::deserialize(deserializer).map(|h| h.0)
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
#[serde(remote = "ListenSlot")]
pub struct ListenSlotDef {
    #[serde_as(as = "Set<TxidDef>")]
    pub txid_watches: Set<Txid>,
    #[serde_as(as = "Set<OutPointDef>")]
    watches: Set<OutPoint>,
    #[serde_as(as = "Set<OutPointDef>")]
    seen: Set<OutPoint>,
}

#[derive(Deserialize)]
struct ListenSlotHelper(#[serde(with = "ListenSlotDef")] ListenSlot);

impl SerializeAs<ListenSlot> for ListenSlotDef {
    fn serialize_as<S>(value: &ListenSlot, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ListenSlotDef::serialize(value, serializer)
    }
}

impl<'de> DeserializeAs<'de, ListenSlot> for ListenSlotDef {
    fn deserialize_as<D>(deserializer: D) -> Result<ListenSlot, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        ListenSlotHelper::deserialize(deserializer).map(|h| h.0)
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
#[serde(remote = "ChainMonitorState")]
pub struct ChainMonitorStateDef {
    height: u32,
    funding_txids: Vec<Txid>,
    funding_vouts: Vec<u32>,
    funding_inputs: Set<OutPoint>,
    funding_height: Option<u32>,
    funding_outpoint: Option<OutPoint>,
    funding_double_spent_height: Option<u32>,
    closing_height: Option<u32>,
}

#[derive(Deserialize)]
struct ChainMonitorStateHelper(#[serde(with = "ChainMonitorStateDef")] ChainMonitorState);

impl SerializeAs<ChainMonitorState> for ChainMonitorStateDef {
    fn serialize_as<S>(value: &ChainMonitorState, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ChainMonitorStateDef::serialize(value, serializer)
    }
}

impl<'de> DeserializeAs<'de, ChainMonitorState> for ChainMonitorStateDef {
    fn deserialize_as<D>(
        deserializer: D,
    ) -> Result<ChainMonitorState, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        ChainMonitorStateHelper::deserialize(deserializer).map(|h| h.0)
    }
}
