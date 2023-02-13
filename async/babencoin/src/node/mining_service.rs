use std::{
    iter,
    sync::{Arc, RwLock},
    thread,
};

use crate::{
    data::{
        Block, BlockAttributes, BlockHash, Transaction, VerifiedBlock, VerifiedTransaction,
        WalletId, MAX_REWARD,
    },
    util::{deserialize_wallet_id, serialize_wallet_id},
};

use anyhow::{Context, Result};
use chrono::Utc;
use crossbeam::channel::{Receiver, Sender};
use crossbeam::{channel, select};
use log::*;
use rand::{thread_rng, Rng};
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use rayon::{ThreadPool, ThreadPoolBuilder};
use serde::{Deserialize, Serialize};

////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize)]
pub struct MiningServiceConfig {
    pub thread_count: usize,
    pub max_tx_per_block: usize,

    #[serde(
        serialize_with = "serialize_wallet_id",
        deserialize_with = "deserialize_wallet_id"
    )]
    pub public_key: WalletId,
}

impl Default for MiningServiceConfig {
    fn default() -> Self {
        Self {
            thread_count: 0,
            max_tx_per_block: 0,
            public_key: WalletId::of_genesis(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
pub struct MiningInfo {
    pub block_index: u64,
    pub prev_hash: BlockHash,
    pub max_hash: BlockHash,
    pub transactions: Vec<VerifiedTransaction>,
}

pub struct MiningService {
    config: MiningServiceConfig,
    info_receiver: Receiver<MiningInfo>,
    block_sender: Sender<VerifiedBlock>,
    // TODO: your code here.
}

impl MiningService {
    pub fn new(
        config: MiningServiceConfig,
        info_receiver: Receiver<MiningInfo>,
        block_sender: Sender<VerifiedBlock>,
    ) -> Self {
        Self {
            config,
            info_receiver,
            block_sender,
        }
    }

    pub fn run(&mut self) {
        //todo: it is not as it should be, but it is intended to be so on the first iteration
        //  parallelism should be on a nounce generation level
        let pool = ThreadPoolBuilder::new()
            .num_threads(self.config.thread_count)
            .build()
            .unwrap();

        loop {
            let mining_info_receiver = self.info_receiver.clone();
            let block_sender = self.block_sender.clone();
            let issuer_wallet = self.config.public_key.clone();
            let max_tx = self.config.max_tx_per_block;

            let MiningInfo {
                block_index,
                prev_hash,
                max_hash,
                transactions,
            } = mining_info_receiver
                .recv()
                .expect("Error receiving MiningInfo");
            let transactions = transactions
                .into_iter()
                .take(max_tx)
                .map(|x| x.into())
                .collect::<Vec<_>>();

            let block_to_send = pool.install(|| {
                (0..u64::MAX)
                    .into_par_iter()
                    .map(move |nonce| {
                        let mut random = thread_rng();
                        let attrs = BlockAttributes {
                            index: block_index,
                            reward: random.gen::<u64>(),
                            nonce,
                            timestamp: Utc::now(),
                            issuer: issuer_wallet.clone(),
                            max_hash,
                            prev_hash,
                        };
                        Block {
                            attrs,
                            transactions: transactions.clone(),
                        }
                    })
                    .find_any(|b| b.clone().verified().is_ok())
                    .unwrap()
            });

            // todo: make beautiful
            block_sender
                .send(block_to_send.verified().expect("Failed to validate block"))
                .expect("Failed to send mined block");
        }
    }
}
