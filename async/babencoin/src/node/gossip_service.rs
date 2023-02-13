use crate::{
    block_forest::BlockForest,
    data::{BlockHash, TransactionHash, VerifiedBlock, VerifiedPeerMessage, VerifiedTransaction},
    node::mining_service::MiningInfo,
    node::peer_service::{PeerCommand, PeerCommandKind, PeerEvent, PeerEventKind, SessionId},
};

use anyhow::{anyhow, Context, Result};
use crossbeam::{
    channel::{self, Receiver, Sender},
    select,
};
use log::*;
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};

use crate::data::PeerMessage;
use crate::data::PeerMessage::Transaction;
use crate::data::VerifiedPeerMessage::{Block, Request};
use crossbeam::channel::RecvError;
use std::ops::Deref;
use std::sync::{Arc, LockResult, RwLock, RwLockReadGuard};
use std::{
    collections::{HashMap, HashSet},
    thread,
    time::Duration,
};

////////////////////////////////////////////////////////////////////////////////

#[derive(Default, Serialize, Deserialize)]
pub struct GossipServiceConfig {
    #[serde(with = "humantime_serde")]
    pub eager_requests_interval: Duration,
}

pub struct GossipService {
    config: GossipServiceConfig,
    event_receiver: Receiver<PeerEvent>,
    command_sender: Sender<PeerCommand>,
    block_receiver: Receiver<VerifiedBlock>,
    mining_info_sender: Sender<MiningInfo>,
    block_forest: Arc<RwLock<BlockForest>>,
    session_storage: Arc<RwLock<SessionStorage>>,
}

struct SessionStorage {
    session_to_blocks: HashMap<SessionId, HashSet<BlockHash>>,
    session_to_transactions: HashMap<SessionId, HashSet<TransactionHash>>,
}

impl Default for SessionStorage {
    fn default() -> Self {
        SessionStorage {
            session_to_blocks: HashMap::new(),
            session_to_transactions: HashMap::new(),
        }
    }
}

impl GossipService {
    pub fn new(
        config: GossipServiceConfig,
        event_receiver: Receiver<PeerEvent>,
        command_sender: Sender<PeerCommand>,
        block_receiver: Receiver<VerifiedBlock>,
        mining_info_sender: Sender<MiningInfo>,
    ) -> Self {
        Self {
            config,
            event_receiver,
            command_sender,
            block_receiver,
            mining_info_sender,
            block_forest: Arc::new(RwLock::new(BlockForest::new())),
            session_storage: Arc::new(RwLock::new(SessionStorage::default())),
        }
    }

    pub fn run(&mut self) {
        let event_receiver = self.event_receiver.clone();
        let block_receiver = self.block_receiver.clone();

        let mining_sender = self.mining_info_sender.clone();
        let command_sender = self.command_sender.clone();

        let block_forest = Arc::clone(&self.block_forest);
        let session_storage = Arc::clone(&self.session_storage);

        loop {
            select! {
                recv(event_receiver) -> msg => Self::handle_peer_event_message(
                    command_sender.clone(),
                    msg,
                    Arc::clone(&block_forest),
                    Arc::clone(&session_storage)).unwrap(),
                //todo: it is breaking test_simple
                /*recv(block_receiver) -> msg => Self::handle_new_block_event_message(
                    command_sender.clone(),
                    msg,
                    Arc::clone(&block_forest),
                    Arc::clone(&session_storage)).unwrap(),*/
            }
        }
    }

    fn handle_new_block_event_message(
        peer_command_sender: Sender<PeerCommand>,
        new_block_message: Result<VerifiedBlock, RecvError>,
        block_forest: Arc<RwLock<BlockForest>>,
        session_storage: Arc<RwLock<SessionStorage>>,
    ) -> Result<()> {
        let verified_block = new_block_message.expect("Failed to receive PeerEvent");
        let mut block_forest = block_forest.write().expect("Failed to capture write lock");
        let validation_res = block_forest.add_block(verified_block.clone());
        if validation_res.is_ok() {
            let mut session_storage = Self::ignore_poison(session_storage.write());
            for (session, mut set) in session_storage.session_to_blocks.iter_mut() {
                let peer_command = PeerCommand {
                    session_id: *session,
                    command_kind: PeerCommandKind::SendMessage(Block(Box::new(
                        verified_block.clone(),
                    ))),
                };
                peer_command_sender
                    .send(peer_command)
                    .expect("Failed to send new block to the channel");
                set.insert(*verified_block.hash());
            }
        }
        validation_res
    }

    fn handle_peer_event_message(
        peer_command_sender: Sender<PeerCommand>,
        peer_event_message: Result<PeerEvent, RecvError>,
        block_forest: Arc<RwLock<BlockForest>>,
        session_storage: Arc<RwLock<SessionStorage>>,
    ) -> Result<()> {
        let PeerEvent {
            session_id,
            event_kind,
        } = peer_event_message.expect("Failed to receive PeerEvent");
        debug!("Event came to gossip: {:?}", event_kind);
        match event_kind {
            PeerEventKind::Connected => GossipService::handle_new_connection(
                session_id,
                block_forest,
                peer_command_sender,
                session_storage,
            )?,
            PeerEventKind::Disconnected => {
                GossipService::handle_disconnect(session_id, session_storage, peer_command_sender)?
            }
            PeerEventKind::NewMessage(mes) => match mes {
                Block(block) => GossipService::handle_new_block(
                    session_id,
                    block_forest,
                    peer_command_sender,
                    session_storage,
                    block,
                )?,
                VerifiedPeerMessage::Transaction(tx) => GossipService::handle_new_transaction(
                    session_id,
                    block_forest,
                    peer_command_sender,
                    session_storage,
                    tx,
                )?,
                Request { block_hash } => GossipService::handle_block_request(
                    session_id,
                    block_forest,
                    peer_command_sender,
                    session_storage,
                    block_hash,
                )?,
            },
        };
        Ok(())
    }

    fn handle_new_connection(
        session_id: SessionId,
        block_forest: Arc<RwLock<BlockForest>>,
        peer_command_sender: Sender<PeerCommand>,
        session_storage: Arc<RwLock<SessionStorage>>,
    ) -> Result<()> {
        {
            let mut session_storage = Self::ignore_poison(session_storage.write());
            session_storage
                .session_to_blocks
                .insert(session_id, HashSet::new());
            session_storage
                .session_to_transactions
                .insert(session_id, HashSet::new());
        }

        let block_forest = Self::ignore_poison(block_forest.read());
        let head = block_forest.head();
        let pending = block_forest.pending_transactions();
        let head = PeerCommand {
            session_id,
            command_kind: PeerCommandKind::SendMessage(Block(Box::new(head.as_ref().clone()))),
        };
        println!("Head {:?}", head);
        peer_command_sender
            .send(head)
            .expect("Failed to send head block");
        for (_, tx) in pending {
            let transaction_message = VerifiedPeerMessage::Transaction(Box::new(tx.clone()));
            let command = PeerCommand {
                session_id,
                command_kind: PeerCommandKind::SendMessage(transaction_message),
            };
            peer_command_sender
                .send(command)
                .expect("Failed to send transaction message");
        }
        Ok(())
    }

    fn handle_disconnect(
        session_id: SessionId,
        session_storage: Arc<RwLock<SessionStorage>>,
        peer_command_sender: Sender<PeerCommand>,
    ) -> Result<()> {
        debug!("Gossip is sending disconnect!");
        let mut session_storage = Self::ignore_poison(session_storage.write());
        session_storage.session_to_blocks.remove(&session_id);
        session_storage.session_to_transactions.remove(&session_id);
        let peer_command = PeerCommand {
            session_id,
            command_kind: PeerCommandKind::Drop,
        };
        debug!("Gossip is sending disconnect: {:?}", peer_command);
        peer_command_sender
            .send(peer_command)
            .expect("Failed to send drop to the channel");
        Ok(())
    }

    fn handle_block_request(
        session_id: SessionId,
        block_forest: Arc<RwLock<BlockForest>>,
        peer_command_sender: Sender<PeerCommand>,
        session_storage: Arc<RwLock<SessionStorage>>,
        block_hash: BlockHash,
    ) -> Result<()> {
        println!("New block request!");
        let read_lock = block_forest
            .read()
            .expect("Failed to capture read lock on forest!");
        read_lock
            .find_block(&block_hash)
            .ok_or(anyhow!("Block was not found"))
            .and_then(|block| {
                println!("Found block {:?}", block);
                let peer_command = PeerCommand {
                    session_id,
                    command_kind: PeerCommandKind::SendMessage(Block(Box::new(
                        block.as_ref().clone(),
                    ))),
                };
                let send_res = peer_command_sender
                    .send(peer_command)
                    .map_err(anyhow::Error::msg);
                if send_res.is_ok() {
                    let mut session_storage = Self::ignore_poison(session_storage.write());
                    let block_set = session_storage
                        .session_to_blocks
                        .get_mut(&session_id)
                        .expect("Not found block set which must be present!");
                    block_set.insert(block_hash);
                    let transactions = block.as_ref().transactions();
                    let transaction_set = session_storage
                        .session_to_transactions
                        .get_mut(&session_id)
                        .expect("Not found transaction set which must be present!");
                    for trx in transactions {
                        transaction_set.insert(*trx.hash());
                    }
                }
                send_res
            })
    }

    fn handle_new_transaction(
        session_id: SessionId,
        block_forest: Arc<RwLock<BlockForest>>,
        peer_command_sender: Sender<PeerCommand>,
        session_storage: Arc<RwLock<SessionStorage>>,
        tx: Box<VerifiedTransaction>,
    ) -> Result<()> {
        let mut block_forest = block_forest
            .write()
            .expect("Failed to capture write lock on forest!");
        let validation_res = block_forest.add_transaction(*tx.clone());
        if validation_res.is_ok() {
            let mut session_storage = Self::ignore_poison(session_storage.write());
            let transaction_set = session_storage
                .session_to_transactions
                .get_mut(&session_id)
                .expect("Not found transaction set which must be present!");
            let trx_hash = tx.compute_hash();
            transaction_set.insert(trx_hash);
            for (session_id, transaction_set) in session_storage.session_to_transactions.iter_mut()
            {
                if !transaction_set.contains(&trx_hash) {
                    let peer_command = PeerCommand {
                        session_id: *session_id,
                        command_kind: PeerCommandKind::SendMessage(
                            VerifiedPeerMessage::Transaction(tx.clone()),
                        ),
                    };
                    peer_command_sender.send(peer_command)?;
                    transaction_set.insert(trx_hash);
                }
            }
            validation_res
        } else {
            Ok(())
        }
    }

    fn handle_new_block(
        session_id: SessionId,
        block_forest: Arc<RwLock<BlockForest>>,
        peer_command_sender: Sender<PeerCommand>,
        session_storage: Arc<RwLock<SessionStorage>>,
        block: Box<VerifiedBlock>,
    ) -> Result<()> {
        {
            let parent = block.prev_hash;
            let block_forest = block_forest
                .read()
                .expect("Failed to capture read lock on forest!");
            let parent_in_forest = block_forest.find_block(&parent);
            if parent_in_forest.is_none() {
                let peer_command = PeerCommand {
                    session_id,
                    command_kind: PeerCommandKind::SendMessage(Request { block_hash: parent }),
                };
                peer_command_sender
                    .send(peer_command)
                    .expect("Failed to request parent for lock");
            }
        }
        let mut block_forest = block_forest.write().expect("Failed to capture write lock");
        let validation_res = block_forest.add_block(*block.clone());
        if validation_res.is_ok() {
            let mut session_storage = Self::ignore_poison(session_storage.write());
            let block_set = session_storage
                .session_to_blocks
                .get_mut(&session_id)
                .expect("Not found block set which must be present!");
            let block_hash = block.hash();
            block_set.insert(*block_hash);
            for (session_id, block_set) in session_storage.session_to_blocks.iter_mut() {
                if !block_set.contains(block_hash) {
                    let peer_command = PeerCommand {
                        session_id: *session_id,
                        command_kind: PeerCommandKind::SendMessage(Block(block.clone())),
                    };
                    peer_command_sender.send(peer_command)?;
                    block_set.insert(*block_hash);
                }
            }
        }
        validation_res
    }

    // todo: what is lock poisoning and how to handle it?
    fn ignore_poison<T>(lock_res: LockResult<T>) -> T {
        match lock_res {
            Ok(x) => x,
            Err(e) => e.into_inner(),
        }
    }
}
