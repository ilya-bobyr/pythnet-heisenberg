//! Convenience wrapper for a task that needs a `NodeAddressService` (and a `BlockhashCache`),
//! removing some "noise" from the call sites that use `NodeAddressService` instances.
//!
//! As all the code that uses the `NodeAddressService` will want to use a `BlockhashCache` as well,
//! it is included.

use std::{sync::Arc, time::Duration};

use anyhow::{Context as _, Result};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use tokio::{pin, select};
use tokio_util::sync::CancellationToken;

use crate::blockhash_cache::BlockhashCache;

use super::NodeAddressService;

pub fn with_node_address_service(
    rpc_client: Arc<RpcClient>,
    websocket_url: &str,
) -> RunWithNodeAddressServiceArgs<'_> {
    RunWithNodeAddressServiceArgs {
        rpc_client,
        websocket_url,
        shutdown: None,
    }
}

/// Holds configuration for an async task.  Provides a builder pattern interface.  Execution happens
/// via the [`run()`] call.
pub struct RunWithNodeAddressServiceArgs<'websocket_url> {
    rpc_client: Arc<RpcClient>,
    websocket_url: &'websocket_url str,
    shutdown: Option<CancellationToken>,
}

impl<'websocket_url> RunWithNodeAddressServiceArgs<'websocket_url> {
    /// Execution will use the specified cancellation token, rather than creating a new one, in
    /// order to shutdown the blockhash update task.  If you do not want to have this token
    /// cancelled, use [`CancellationToken::child_token()`].
    pub fn shutdown_via(mut self, shutdown: CancellationToken) -> Self {
        self.shutdown = Some(shutdown);
        self
    }

    /// Runs the specified asynchronous operation with an access to a [`BlockhashCache`] instance,
    /// that is kept up to date.
    pub async fn run<'context, T, Op>(self, op: Op) -> Result<T>
    where
        Op: AsyncFnOnce(&BlockhashCache, NodeAddressService) -> T + 'websocket_url + 'context,
        'websocket_url: 'context,
    {
        let Self {
            rpc_client,
            websocket_url,
            shutdown,
        } = self;

        let shutdown = shutdown.unwrap_or_else(CancellationToken::new);

        let blockhash_cache = BlockhashCache::uninitialized();
        blockhash_cache.init(&rpc_client).await;

        let blockhash_cache_refresh_task = blockhash_cache.run_refresh_loop(
            &rpc_client,
            Duration::from_millis(400),
            shutdown.clone(),
        );
        pin!(blockhash_cache_refresh_task);

        let (node_address_service, node_address_service_handle) =
            NodeAddressService::init(rpc_client.clone(), websocket_url, shutdown.clone())
                .await
                .context("NodeAddressService construction failed")?;

        let op_task = op(&blockhash_cache, node_address_service);
        pin!(op_task);

        let op_res = select! {
            op_res = &mut op_task => op_res,
            () = &mut blockhash_cache_refresh_task => {
                panic!("BlockhashCache should not stop until requested");
            }
        };

        shutdown.cancel();
        blockhash_cache_refresh_task.await;

        node_address_service_handle
            .await
            .context("NodeAddressService thread was poisoned")?
            .context("Waiting for the NodeAddressService task to stop")?;

        Ok(op_res)
    }
}
