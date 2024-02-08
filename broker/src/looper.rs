use crate::conn::{ChannelRequest, LssReq};
use crate::handle::handle_message;
use crate::secp256k1::PublicKey;
use log::*;
use lru::LruCache;
use rocket::tokio::sync::mpsc;
use sphinx_signer::lightning_signer::bitcoin::hashes::{sha256::Hash as Sha256Hash, Hash};
use std::num::NonZeroUsize;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;
use vls_protocol::{msgs, msgs::Message, Error, Result};
use vls_proxy::client::Client;

const PREAPPROVE_CACHE_TTL: Duration = Duration::from_secs(60);
const PREAPPROVE_CACHE_SIZE: usize = 6;

struct PreapprovalCacheEntry {
    tstamp: SystemTime,
    reply_bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct ClientId {
    pub peer_id: PublicKey,
    pub dbid: u64,
}

/// Implement the hsmd UNIX fd protocol.
pub struct SignerLoop<C: 'static + Client> {
    client: C,
    log_prefix: String,
    vls_tx: mpsc::Sender<ChannelRequest>,
    lss_tx: mpsc::Sender<LssReq>,
    client_id: Option<ClientId>,
    preapproval_cache: LruCache<Sha256Hash, PreapprovalCacheEntry>,
}

impl<C: 'static + Client> SignerLoop<C> {
    /// Create a loop for the root (lightningd) connection, but doesn't start it yet
    pub fn new(
        client: C,
        vls_tx: mpsc::Sender<ChannelRequest>,
        lss_tx: mpsc::Sender<LssReq>,
    ) -> Self {
        let log_prefix = format!("{}/{}", std::process::id(), client.id());
        let preapproval_cache = LruCache::new(NonZeroUsize::new(PREAPPROVE_CACHE_SIZE).unwrap());
        Self {
            client,
            log_prefix,
            vls_tx,
            lss_tx,
            client_id: None,
            preapproval_cache,
        }
    }

    // Create a loop for a non-root connection
    fn new_for_client(
        client: C,
        lss_tx: mpsc::Sender<LssReq>,
        vls_tx: mpsc::Sender<ChannelRequest>,
        client_id: ClientId,
    ) -> Self {
        let log_prefix = format!("{}/{}", std::process::id(), client.id());
        let preapproval_cache = LruCache::new(NonZeroUsize::new(PREAPPROVE_CACHE_SIZE).unwrap());
        Self {
            client,
            log_prefix,
            vls_tx,
            lss_tx,
            client_id: Some(client_id),
            preapproval_cache,
        }
    }

    /// Start the read loop
    pub fn start(&mut self) {
        info!("loop {}: start", self.log_prefix);
        match self.do_loop() {
            Ok(()) => info!("loop {}: done", self.log_prefix),
            Err(Error::Eof) => info!("loop {}: ending", self.log_prefix),
            Err(e) => error!("loop {}: error {:?}", self.log_prefix, e),
        }
    }

    fn do_loop(&mut self) -> Result<()> {
        loop {
            let raw_msg = self.client.read_raw()?;
            // debug!("loop {}: got raw", self.log_prefix);
            let msg = msgs::from_vec(raw_msg.clone())?;
            info!("loop {}: got {}", self.log_prefix, vls_cmd(&msg));
            match msg {
                Message::ClientHsmFd(m) => {
                    self.client.write(msgs::ClientHsmFdReply {}).unwrap();
                    let new_client = self.client.new_client();
                    info!("new client {} -> {}", self.log_prefix, new_client.id());
                    let peer_id = PublicKey::from_slice(&m.peer_id.0).expect("client pubkey"); // we don't expect a bad key from lightningd parent
                    let client_id = ClientId {
                        peer_id,
                        dbid: m.dbid,
                    };
                    let mut new_loop = SignerLoop::new_for_client(
                        new_client,
                        self.lss_tx.clone(),
                        self.vls_tx.clone(),
                        client_id,
                    );
                    thread::spawn(move || new_loop.start());
                }
                Message::Memleak(_) => {
                    // info!("Memleak");
                    let reply = msgs::MemleakReply { result: false };
                    self.client.write(reply)?;
                }
                msg => {
                    if let Message::HsmdInit(ref _m) = msg {
                        panic!("HsmdInit should have been handled already!");
                    }
                    // check if we got the same preapprove message less than PREAPPROVE_CACHE_TTL seconds ago
                    if let Message::PreapproveInvoice(_) | Message::PreapproveKeysend(_) = msg {
                        let now = SystemTime::now();
                        let req_hash = Sha256Hash::hash(&raw_msg);
                        if let Some(entry) = self.preapproval_cache.get(&req_hash) {
                            let age = now.duration_since(entry.tstamp).expect("age");
                            if age < PREAPPROVE_CACHE_TTL {
                                debug!("{} found in preapproval cache", self.log_prefix);
                                let reply = entry.reply_bytes.clone();
                                self.client.write_vec(reply)?;
                                continue;
                            }
                        }
                    }

                    let reply_bytes = handle_message(
                        &self.client_id,
                        raw_msg.clone(),
                        &self.vls_tx,
                        &self.lss_tx,
                    );

                    // post signer response processing
                    let reply = msgs::from_vec(reply_bytes.clone()).expect("parse reply failed");
                    match reply {
                        // did we just preapprove a keysend ? if so add it to the cache
                        Message::PreapproveKeysendReply(pkr) => {
                            if pkr.result {
                                debug!("{} adding keysend to preapproval cache", self.log_prefix);
                                let now = SystemTime::now();
                                let req_hash = Sha256Hash::hash(&raw_msg);
                                self.preapproval_cache.put(
                                    req_hash,
                                    PreapprovalCacheEntry {
                                        tstamp: now,
                                        reply_bytes: reply_bytes.clone(),
                                    },
                                );
                            }
                        }
                        // did we just preapprove an invoice ? if so add it to the cache
                        Message::PreapproveInvoiceReply(pir) => {
                            if pir.result {
                                debug!("{} adding invoice to preapproval cache", self.log_prefix);
                                let now = SystemTime::now();
                                let req_hash = Sha256Hash::hash(&raw_msg);
                                self.preapproval_cache.put(
                                    req_hash,
                                    PreapprovalCacheEntry {
                                        tstamp: now,
                                        reply_bytes: reply_bytes.clone(),
                                    },
                                );
                            }
                        }
                        _ => {} // for future messages needing post signer response processing
                    }
                    // write the reply to CLN
                    self.client.write_vec(reply_bytes)?;
                }
            }
        }
    }
}

fn vls_cmd(msg: &Message) -> String {
    let m = match msg {
        Message::Ping(_) => "Ping",
        Message::Pong(_) => "Pong",
        Message::HsmdInit(_) => "HsmdInit",
        // HsmdInitReplyV1(HsmdInitReplyV1),
        #[allow(deprecated)]
        Message::HsmdInitReplyV2(_) => "HsmdInitReplyV2",
        Message::HsmdInitReplyV4(_) => "HsmdInitReplyV4",
        Message::HsmdInit2(_) => "HsmdInit2",
        Message::HsmdInit2Reply(_) => "HsmdInit2Reply",
        Message::ClientHsmFd(_) => "ClientHsmFd",
        Message::ClientHsmFdReply(_) => "ClientHsmFdReply",
        Message::SignInvoice(_) => "SignInvoice",
        Message::SignInvoiceReply(_) => "SignInvoiceReply",
        Message::SignWithdrawal(_) => "SignWithdrawal",
        Message::SignWithdrawalReply(_) => "SignWithdrawalReply",
        Message::Ecdh(_) => "Ecdh",
        Message::EcdhReply(_) => "EcdhReply",
        Message::Memleak(_) => "Memleak",
        Message::MemleakReply(_) => "MemleakReply",
        Message::CheckFutureSecret(_) => "CheckFutureSecret",
        Message::CheckFutureSecretReply(_) => "CheckFutureSecretReply",
        Message::SignBolt12(_) => "SignBolt12",
        Message::SignBolt12Reply(_) => "SignBolt12Reply",
        Message::PreapproveInvoice(_) => "PreapproveInvoice",
        Message::PreapproveInvoiceReply(_) => "PreapproveInvoiceReply",
        Message::PreapproveKeysend(_) => "PreapproveKeysend",
        Message::PreapproveKeysendReply(_) => "PreapproveKeysendReply",
        Message::DeriveSecret(_) => "DeriveSecret",
        Message::DeriveSecretReply(_) => "DeriveSecretReply",
        Message::CheckPubKey(_) => "CheckPubKey",
        Message::CheckPubKeyReply(_) => "CheckPubKeyReply",
        Message::SignMessage(_) => "SignMessage",
        Message::SignMessageReply(_) => "SignMessageReply",
        Message::SignChannelUpdate(_) => "SignChannelUpdate",
        Message::SignChannelUpdateReply(_) => "SignChannelUpdateReply",
        Message::SignChannelAnnouncement(_) => "SignChannelAnnouncement",
        Message::SignChannelAnnouncementReply(_) => "SignChannelAnnouncementReply",
        Message::SignNodeAnnouncement(_) => "SignNodeAnnouncement",
        Message::SignNodeAnnouncementReply(_) => "SignNodeAnnouncementReply",
        Message::GetPerCommitmentPoint(_) => "GetPerCommitmentPoint",
        Message::GetPerCommitmentPointReply(_) => "GetPerCommitmentPointReply",
        Message::GetPerCommitmentPoint2(_) => "GetPerCommitmentPoint2",
        Message::GetPerCommitmentPoint2Reply(_) => "GetPerCommitmentPoint2Reply",
        Message::SetupChannel(_) => "SetupChannel",
        Message::SetupChannelReply(_) => "SetupChannelReply",
        Message::ValidateCommitmentTx(_) => "ValidateCommitmentTx",
        Message::ValidateCommitmentTx2(_) => "ValidateCommitmentTx2",
        Message::ValidateCommitmentTxReply(_) => "ValidateCommitmentTxReply",
        Message::ValidateRevocation(_) => "ValidateRevocation",
        Message::ValidateRevocationReply(_) => "ValidateRevocationReply",
        Message::SignRemoteCommitmentTx(_) => "SignRemoteCommitmentTx",
        Message::SignRemoteCommitmentTx2(_) => "SignRemoteCommitmentTx2",
        Message::SignCommitmentTxWithHtlcsReply(_) => "SignCommitmentTxWithHtlcsReply",
        Message::SignDelayedPaymentToUs(_) => "SignDelayedPaymentToUs",
        Message::SignAnyDelayedPaymentToUs(_) => "SignAnyDelayedPaymentToUs",
        Message::SignRemoteHtlcToUs(_) => "SignRemoteHtlcToUs",
        Message::SignAnyRemoteHtlcToUs(_) => "SignAnyRemoteHtlcToUs",
        Message::SignLocalHtlcTx(_) => "SignLocalHtlcTx",
        Message::SignAnyLocalHtlcTx(_) => "SignAnyLocalHtlcTx",
        Message::SignCommitmentTx(_) => "SignCommitmentTx",
        Message::SignLocalCommitmentTx2(_) => "SignLocalCommitmentTx2",
        Message::SignGossipMessage(_) => "SignGossipMessage",
        Message::SignMutualCloseTx(_) => "SignMutualCloseTx",
        Message::SignMutualCloseTx2(_) => "SignMutualCloseTx2",
        Message::SignTxReply(_) => "SignTxReply",
        Message::SignCommitmentTxReply(_) => "SignCommitmentTxReply",
        Message::GetChannelBasepoints(_) => "GetChannelBasepoints",
        Message::GetChannelBasepointsReply(_) => "GetChannelBasepointsReply",
        Message::NewChannel(_) => "NewChannel",
        Message::NewChannelReply(_) => "NewChannelReply",
        Message::SignRemoteHtlcTx(_) => "SignRemoteHtlcTx",
        Message::SignPenaltyToUs(_) => "SignPenaltyToUs",
        Message::SignAnyPenaltyToUs(_) => "SignAnyPenaltyToUs",
        Message::TipInfo(_) => "TipInfo",
        Message::TipInfoReply(_) => "TipInfoReply",
        Message::ForwardWatches(_) => "ForwardWatches",
        Message::ForwardWatchesReply(_) => "ForwardWatchesReply",
        Message::ReverseWatches(_) => "ReverseWatches",
        Message::ReverseWatchesReply(_) => "ReverseWatchesReply",
        Message::AddBlock(_) => "AddBlock",
        Message::AddBlockReply(_) => "AddBlockReply",
        Message::RemoveBlock(_) => "RemoveBlock",
        Message::RemoveBlockReply(_) => "RemoveBlockReply",
        Message::GetHeartbeat(_) => "GetHeartbeat",
        Message::GetHeartbeatReply(_) => "GetHeartbeatReply",
        Message::NodeInfo(_) => "NodeInfo",
        Message::NodeInfoReply(_) => "NodeInfoReply",
        Message::Unknown(_) => "Unknown",
        Message::SignAnchorspend(_) => "SignAnchorspend",
        Message::SignAnchorspendReply(_) => "SignAnchorspendReply",
        Message::SignSpliceTx(_) => "SignAnchorspendReply",
        Message::SignHtlcTxMingle(_) => "SignHtlcTxMingle",
        Message::SignHtlcTxMingleReply(_) => "SignHtlcTxMingleReply",
        Message::BlockChunk(_) => "BlockChunk",
        Message::BlockChunkReply(_) => "BlockChunkReply",
        Message::SignerError(_) => "SignerError",
        Message::CheckOutpoint(_) => "CheckOutpoint",
        Message::CheckOutpointReply(_) => "CheckOutpointReply",
        Message::LockOutpoint(_) => "LockOutpoint",
        Message::LockOutpointReply(_) => "LockOutpointReply",
        Message::ForgetChannel(_) => "ForgetChannel",
        Message::ForgetChannelReply(_) => "ForgetChannelReply",
        Message::RevokeCommitmentTx(_) => "RevokeCommitmentTx",
        Message::RevokeCommitmentTxReply(_) => "RevokeCommitmentTxReply",
    };
    m.to_string()
}
