//! When we prepare and send multiple transactions it makes sense to cache the last blockhash, to
//! save on the RPC calls.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context as _, Result};
use log::warn;
use parking_lot::Mutex;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::hash::Hash;
use tokio::{select, time::sleep};
use tokio_util::sync::CancellationToken;

pub mod runner;

/// A convenient way to use a [`BlockhashCache`] in your code.  [`with_blockhash`] uses a builder
/// pattern to configure a [`BlockhashCache`] and then a [`RunWithBlockhashArgs::run()`] method is
/// used to invoke an async operation with a [`BlockhashCache`] reference available for consumption.
pub use runner::with_blockhash;

#[derive(Debug, Clone)]
pub struct BlockhashCache {
    last_hash: Arc<Mutex<Hash>>,
}

impl BlockhashCache {
    /// Creates a new [`BlockhashCache`].  Note that it contains a default [`Hash`], you want to
    /// call [`BlockhashCache::refresh()`] at least once before the first use.
    pub fn uninitialized() -> Self {
        Self {
            last_hash: Arc::default(),
        }
    }

    /// Repeatedly calls `self.refresh()` until we get a non-default value.
    pub async fn init(&self, rpc_client: &RpcClient) {
        loop {
            let res = self.refresh(rpc_client).await;
            if let Err(err) = res {
                warn!("Failed to get the latest blockhash: {err}");
            }

            // We start with not blockhash, expressed as `Hash::default()`.  We can not do anything
            // until we get at least one blockhash.
            if self.get() != Hash::default() {
                return;
            }
        }
    }

    pub async fn refresh(&self, rpc_client: &RpcClient) -> Result<()> {
        let blockhash = rpc_client
            .get_latest_blockhash()
            .await
            .context("get_latest_blockhash() failed")?;
        let mut last_hash = self.last_hash.lock();
        if *last_hash == blockhash {
            // There are two probable cases why you might be seeing this warning:
            // 1. You are refreshing the blockhash too frequently.  It does not make sense to
            //    refresh more frequently than once every slot.  And you probably want even lower
            //    rate to avoid refreshing within the same slot.
            // 2. The cluster is not making any progress, in which case, this warning could help
            //    debug the consensus issue.
            warn!("`get_latest_blockhash()` returned the same blockhash we've seen before.");
        } else {
            *last_hash = blockhash;
        }
        Ok(())
    }

    pub async fn run_refresh_loop(
        &self,
        rpc_client: &RpcClient,
        min_loop_duration: Duration,
        exit: CancellationToken,
    ) {
        while !exit.is_cancelled() {
            let loop_start = Instant::now();

            loop {
                let res = select! {
                    res = self.refresh(rpc_client) => res,
                    () = exit.cancelled() => break,
                };
                if let Err(err) = res {
                    warn!("Failed to get the latest blockhash: {err}");
                } else {
                    break;
                }
            }

            let loop_wait_time = min_loop_duration.saturating_sub(loop_start.elapsed());
            if !loop_wait_time.is_zero() {
                select! {
                    () = sleep(loop_wait_time) => (),
                    () = exit.cancelled() => break,
                }
            }
        }
    }

    pub fn get(&self) -> Hash {
        *self.last_hash.lock()
    }
}
