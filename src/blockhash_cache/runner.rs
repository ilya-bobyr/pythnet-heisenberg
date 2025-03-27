//! Convenience wrapper for the `BlockhashCache`, removing a lot of the "noise" from the call sites
//! that need to use an instance of a `BlockhashCache`.

use std::time::Duration;

use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use tokio::{pin, select};
use tokio_util::sync::CancellationToken;

use super::BlockhashCache;

/// Prepares an asynchronous operation with an access to a [`BlockhashCache`] instance, that is kept
/// up to date.
pub fn with_blockhash(rpc_client: &RpcClient) -> RunWithBlockhashArgs<'_> {
    RunWithBlockhashArgs {
        rpc_client,
        shutdown: None,
    }
}

/// Holds configuration for an async task.  Provides a builder pattern interface.  Execution happens
/// via the [`run()`] call.
pub struct RunWithBlockhashArgs<'rpc_client> {
    rpc_client: &'rpc_client RpcClient,
    shutdown: Option<CancellationToken>,
}

impl<'rpc_client> RunWithBlockhashArgs<'rpc_client> {
    /// Execution will use the specified cancellation token, rather than creating a new one, in
    /// order to shutdown the blockhash update task.  If you do not want to have this token
    /// cancelled, use [`CancellationToken::child_token()`].
    pub fn shutdown_via(mut self, shutdown: CancellationToken) -> Self {
        self.shutdown = Some(shutdown);
        self
    }

    /// Runs the specified asynchronous operation with an access to a [`BlockhashCache`] instance,
    /// that is kept up to date.
    pub async fn run<'context, T, Op>(self, op: Op) -> T
    where
        Op: AsyncFnOnce(&BlockhashCache) -> T + 'rpc_client + 'context,
        'rpc_client: 'context,
    {
        let Self {
            rpc_client,
            shutdown,
        } = self;

        let shutdown = shutdown.unwrap_or_else(CancellationToken::new);

        let blockhash_cache = BlockhashCache::uninitialized();
        blockhash_cache.init(rpc_client).await;

        let blockhash_cache_refresh_task = blockhash_cache.run_refresh_loop(
            rpc_client,
            Duration::from_millis(400),
            shutdown.clone(),
        );
        pin!(blockhash_cache_refresh_task);

        let op_task = op(&blockhash_cache);
        pin!(op_task);

        let op_res = select! {
            op_res = &mut op_task => op_res,
            () = &mut blockhash_cache_refresh_task => {
                panic!("BlockhashCache should not stop until requested");
            }
        };

        shutdown.cancel();
        blockhash_cache_refresh_task.await;

        op_res
    }
}
