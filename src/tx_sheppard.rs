//! There are many cases when we want to execute multiple transactions in parallel and wait for all
//! of them to complete.
//!
//! [`TxSheppard`] is a solution to this problem, including a retry of the transaction execution, up
//! to the specified number of times.
//!
//! It also shows progress on the terminal, providing for a nice UI.

use std::{cmp, collections::HashSet, time::Duration};

use anyhow::Result;
use futures::{StreamExt as _, future::BoxFuture, stream::FuturesUnordered};
use indicatif::{ProgressBar, ProgressStyle};
use itertools::izip;
use log::warn;
use serde_json::json;
use solana_program::vote::state::MAX_LOCKOUT_HISTORY;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::{
    client_error::Error as RpcClientError, request::RpcRequest, response::Response as RpcResponse,
};
use solana_sdk::{
    signature::Signature,
    transaction::{Transaction, TransactionError},
};
use solana_transaction_status::TransactionStatus;
use tokio::{
    pin, select,
    time::{self, Instant, sleep},
};
use tokio_util::sync::CancellationToken;

use crate::blockhash_cache::BlockhashCache;

pub fn with_sheppard(rpc_client: &RpcClient) -> RunWithTxSheppardArgs<'_> {
    RunWithTxSheppardArgs {
        rpc_client,
        shutdown: None,
        rpc_failure_retry_delay: None,
        status_failure_retry_delay: None,
        retry_count: None,
    }
}

pub struct RunWithTxSheppardArgs<'rpc_client> {
    rpc_client: &'rpc_client RpcClient,
    shutdown: Option<CancellationToken>,
    rpc_failure_retry_delay: Option<Duration>,
    status_failure_retry_delay: Option<Duration>,
    retry_count: Option<usize>,
}

impl<'rpc_client> RunWithTxSheppardArgs<'rpc_client> {
    #[allow(unused)]
    pub fn shutdown_via(mut self, shutdown: CancellationToken) -> Self {
        self.shutdown = Some(shutdown);
        self
    }

    #[allow(unused)]
    pub fn rpc_failure_retry_delay(mut self, delay: Duration) -> Self {
        self.rpc_failure_retry_delay = Some(delay);
        self
    }

    #[allow(unused)]
    pub fn status_failure_retry_delay(mut self, delay: Duration) -> Self {
        self.status_failure_retry_delay = Some(delay);
        self
    }

    #[allow(unused)]
    pub fn retry_count(mut self, count: usize) -> Self {
        self.retry_count = Some(count);
        self
    }

    pub async fn run<'context, TxBuilder>(
        self,
        tx_builders: impl Iterator<Item = TxBuilder> + Clone + 'context,
    ) -> Result<()>
    where
        'rpc_client: 'context,
        TxBuilder: Fn(/* blockhash_cache: */ &BlockhashCache) -> Transaction + 'context,
    {
        let Self {
            rpc_client,
            shutdown,
            rpc_failure_retry_delay,
            status_failure_retry_delay,
            retry_count,
        } = self;

        let shutdown = shutdown.unwrap_or_else(CancellationToken::new);
        let rpc_failure_retry_delay =
            rpc_failure_retry_delay.unwrap_or_else(|| Duration::from_millis(400));
        let status_failure_retry_delay =
            status_failure_retry_delay.unwrap_or_else(|| Duration::from_millis(3 * 400));
        let retry_count = retry_count.unwrap_or(3);

        run_impl(
            rpc_client,
            shutdown,
            rpc_failure_retry_delay,
            status_failure_retry_delay,
            retry_count,
            tx_builders,
        )
        .await
    }
}

async fn run_impl<'rpc_client, 'context, TxBuilder>(
    rpc_client: &'rpc_client RpcClient,
    shutdown: CancellationToken,
    rpc_failure_retry_delay: Duration,
    status_failure_retry_delay: Duration,
    retry_count: usize,
    tx_builders: impl Iterator<Item = TxBuilder> + 'context,
) -> Result<()>
where
    'rpc_client: 'context,
    TxBuilder: Fn(/* blockhash_cache: */ &BlockhashCache) -> Transaction + 'context,
{
    let tx_builders = tx_builders.collect::<Vec<_>>();

    let blockhash_cache = BlockhashCache::uninitialized();
    blockhash_cache.init(rpc_client).await;
    let blockhash_cache = &blockhash_cache;

    let blockhash_cache_refresh_task =
        blockhash_cache.run_refresh_loop(rpc_client, Duration::from_millis(400), shutdown.clone());
    pin!(blockhash_cache_refresh_task);

    let tx_builder_count = tx_builders.len();

    let mut execution_status =
        vec![TargetExecutionStatus::Sending { retry_count }; tx_builder_count];

    let mut sending_txs = izip!(0usize.., tx_builders.iter())
        .map(|(idx, builder)| {
            send_one_tx(rpc_client, blockhash_cache, Duration::ZERO, idx, builder)
        })
        .collect::<FuturesUnordered<_>>();

    let mut last_status_check = Instant::now();
    let mut in_status_check = HashSet::new();

    let mut succeeded_count = 0;
    let mut failed_count = 0;

    let progress_bar = ProgressBar::new(42);
    progress_bar.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .expect("ProgressStyle::template direct input to be correct"),
    );
    // Update the progress bar twice a second.
    let mut progrss_update_timer = time::interval(Duration::from_millis(500));

    let mut status_task = start_status_check(
        rpc_client,
        &mut last_status_check,
        &execution_status,
        &in_status_check,
    );

    while !sending_txs.is_empty() || !in_status_check.is_empty() {
        select! {
            next_send_res = sending_txs.next(), if !sending_txs.is_empty() => match next_send_res {
                None => (),
                Some(send_res) => apply_send_result(
                    rpc_client,
                    blockhash_cache,
                    &tx_builders,
                    &mut execution_status,
                    &mut sending_txs,
                    &mut in_status_check,
                    rpc_failure_retry_delay,
                    send_res,
                ),
            },
            status_results = &mut status_task => {
                match status_results {
                    Ok(status_results) => apply_status_result(
                        rpc_client,
                        blockhash_cache,
                        &tx_builders,
                        &mut execution_status,
                        &mut sending_txs,
                        &mut in_status_check,
                        &mut succeeded_count,
                        &mut failed_count,
                        status_failure_retry_delay,
                        status_results,
                    ),
                    Err(error) => {
                        warn!("RPC request for the transaction status failed: {error}");
                    }
                };
                status_task = start_status_check(
                    rpc_client,
                    &mut last_status_check,
                    &execution_status,
                    &in_status_check,
                );
            }
            _instant = progrss_update_timer.tick() => update_progress_bar(
                &progress_bar,
                sending_txs.len(),
                &execution_status,
                &in_status_check,
                succeeded_count,
                failed_count,
            ),
            () = &mut blockhash_cache_refresh_task => {
                panic!("BlockhashCache should not stop until requested");
            }
        };
    }

    // While we remove the progress bar next, if the console has any intermediate messages, the
    // very last message might still be visible.  So we want to show the final state.
    update_progress_bar(
        &progress_bar,
        sending_txs.len(),
        &execution_status,
        &in_status_check,
        succeeded_count,
        failed_count,
    );
    progress_bar.finish_and_clear();

    shutdown.cancel();
    blockhash_cache_refresh_task.await;

    if failed_count > 0 {
        for status in execution_status {
            let TargetExecutionStatus::Failed(error) = status else {
                continue;
            };
            println!("Transaction failed: {error}");
        }
    }

    Ok(())
}

fn send_one_tx<'rpc_client, 'context, TxBuilder>(
    rpc_client: &'rpc_client RpcClient,
    blockhash_cache: &BlockhashCache,
    delay: Duration,
    idx: usize,
    builder: TxBuilder,
) -> BoxFuture<'context, TxSendResult>
where
    'rpc_client: 'context,
    TxBuilder: Fn(/* blockhash_cache: */ &BlockhashCache) -> Transaction,
{
    let tx = builder(blockhash_cache);
    Box::pin(async move {
        if !delay.is_zero() {
            sleep(delay).await;
        }

        let res = rpc_client.send_transaction(&tx).await;
        TxSendResult::from_result(idx, res)
    })
}

#[allow(clippy::too_many_arguments)]
fn apply_send_result<'rpc_client, 'context, TxBuilder>(
    rpc_client: &'rpc_client RpcClient,
    blockhash_cache: &BlockhashCache,
    tx_builders: &[TxBuilder],
    execution_status: &mut [TargetExecutionStatus],
    sending_txs: &mut FuturesUnordered<BoxFuture<'context, TxSendResult>>,
    in_status_check: &mut HashSet<usize>,
    retry_delay: Duration,
    send_result: TxSendResult,
) where
    'rpc_client: 'context,
    TxBuilder: Fn(/* blockhash_cache: */ &BlockhashCache) -> Transaction,
{
    match send_result {
        TxSendResult::Success { idx, signature } => {
            execution_status[idx].send_success(signature);
            in_status_check.insert(idx);
        }
        TxSendResult::Fail { idx, error } => {
            let retry = execution_status[idx].send_failed(error);
            if retry {
                sending_txs.push(send_one_tx(
                    rpc_client,
                    blockhash_cache,
                    retry_delay,
                    idx,
                    &tx_builders[idx],
                ));
            }
        }
    }
}

fn start_status_check<'rpc_client>(
    rpc_client: &'rpc_client RpcClient,
    last_status_check: &mut Instant,
    execution_status: &[TargetExecutionStatus],
    in_status_check: &HashSet<usize>,
) -> BoxFuture<'rpc_client, Result<Vec<TxStatusResult>, RpcClientError>> {
    let now = Instant::now();
    let iteration_time = now.duration_since(*last_status_check);
    // Update the status as frequently as we update the UI.
    let delay = Duration::from_millis(500).saturating_sub(iteration_time);
    *last_status_check = now + delay;

    let (indices, signatures): (Vec<usize>, Vec<String>) = in_status_check
        .iter()
        .copied()
        .map(|idx| {
            (
                idx,
                execution_status[idx]
                    .signature_for_status_check()
                    .to_string(),
            )
        })
        .unzip();

    Box::pin(async move {
        if !delay.is_zero() {
            sleep(delay).await;
        }

        if indices.is_empty() {
            return Ok(vec![]);
        }

        let results: RpcResponse<Vec<Option<TransactionStatus>>> = rpc_client
            .send(RpcRequest::GetSignatureStatuses, json!([signatures]))
            .await?;
        let results = results.value;

        let res = izip!(indices.into_iter(), results.into_iter())
            .map(|(idx, result)| {
                let Some(tx_status) = result else {
                    return TxStatusResult::Absent { idx };
                };

                match tx_status.confirmations {
                    None => match tx_status.err {
                        None => TxStatusResult::Success { idx },
                        Some(error) => TxStatusResult::Fail { idx, error },
                    },
                    Some(confirmations) => {
                        let confirmations = u8::try_from(confirmations).unwrap_or(u8::MAX);
                        TxStatusResult::Pending { idx, confirmations }
                    }
                }
            })
            .collect::<Vec<_>>();

        Ok(res)
    })
}

#[allow(clippy::too_many_arguments)]
fn apply_status_result<'rpc_client, 'context, TxBuilder>(
    rpc_client: &'rpc_client RpcClient,
    blockhash_cache: &BlockhashCache,
    tx_builders: &[TxBuilder],
    execution_status: &mut [TargetExecutionStatus],
    sending_txs: &mut FuturesUnordered<BoxFuture<'context, TxSendResult>>,
    in_status_check: &mut HashSet<usize>,
    succeeded_count: &mut u64,
    failed_count: &mut u64,
    retry_delay: Duration,
    status_results: Vec<TxStatusResult>,
) where
    'rpc_client: 'context,
    TxBuilder: Fn(/* blockhash_cache: */ &BlockhashCache) -> Transaction,
{
    for status_result in status_results.into_iter() {
        match status_result {
            TxStatusResult::Success { idx } => {
                in_status_check.remove(&idx);
                execution_status[idx].status_success();
                *succeeded_count += 1;
            }
            TxStatusResult::Absent { idx } => match execution_status[idx].status_absent() {
                StatusAbsentAction::WaitMore => (),
                StatusAbsentAction::Retry => {
                    in_status_check.remove(&idx);
                    sending_txs.push(send_one_tx(
                        rpc_client,
                        blockhash_cache,
                        retry_delay,
                        idx,
                        &tx_builders[idx],
                    ));
                }
                StatusAbsentAction::Failed => {
                    in_status_check.remove(&idx);
                    *failed_count += 1;
                }
            },
            TxStatusResult::Pending { idx, confirmations } => {
                execution_status[idx].status_pending(confirmations);
            }
            TxStatusResult::Fail { idx, error } => {
                in_status_check.remove(&idx);
                let retry = execution_status[idx].status_failed(error);
                if retry {
                    sending_txs.push(send_one_tx(
                        rpc_client,
                        blockhash_cache,
                        retry_delay,
                        idx,
                        &tx_builders[idx],
                    ));
                } else {
                    *failed_count += 1;
                }
            }
        }
    }
}

fn update_progress_bar(
    progress_bar: &ProgressBar,
    sending: usize,
    execution_status: &[TargetExecutionStatus],
    in_status_check: &HashSet<usize>,
    succeeded: u64,
    failed: u64,
) {
    progress_bar.tick();

    let awaiting_confirmation = in_status_check.len();

    const MAX_CONFIRMATIONS: u8 = (MAX_LOCKOUT_HISTORY + 1) as u8;
    let min_confirmations = in_status_check
        .iter()
        .map(|idx| execution_status[*idx].status_confirmations())
        .min()
        .unwrap_or(0);
    let min_confirmations = cmp::min(min_confirmations, MAX_CONFIRMATIONS);

    if failed == 0 {
        progress_bar.set_message(format!(
            "[{min_confirmations}/{MAX_CONFIRMATIONS}] \
             Sending: {sending} / Confirming: {awaiting_confirmation} / Succeeded: {succeeded}"
        ));
    } else {
        progress_bar.set_message(format!(
            "[{min_confirmations}/{MAX_CONFIRMATIONS}] \
             Sending: {sending} / Confirming: {awaiting_confirmation} / Succeeded: {succeeded} \
             Failed: {failed}"
        ));
    }
}

#[derive(Debug, Clone)]
pub enum TargetExecutionStatus {
    /// An async operation that is sending the transaction into the cluster has been started, but
    /// not completed yet.
    Sending {
        retry_count: usize,
    },
    /// Transaction was sent, and we are waiting for it to be accepted.
    WaitingConfirmation {
        /// Moment when we started waiting for this target to land a transaction.
        wait_start: Instant,
        /// When we retry, the next status will have this field decreased.
        retry_count: usize,
        signature: Signature,
        /// Number of confirmations this transaction received.
        confirmations: Option<u8>,
    },
    Success,
    /// We ran out of retires for this target, and so we just record the last error.
    Failed(String),
}

impl TargetExecutionStatus {
    fn send_success(&mut self, signature: Signature) {
        *self = match self {
            Self::Sending { retry_count } => Self::WaitingConfirmation {
                wait_start: Instant::now(),
                retry_count: *retry_count,
                signature,
                confirmations: None,
            },
            Self::WaitingConfirmation { .. } => panic!("Currently in `WaitingConfirmation` state"),
            Self::Success => panic!("Currently in `Success` state"),
            Self::Failed(_) => panic!("Currently in `Failed` state"),
        }
    }

    fn send_failed(&mut self, error: RpcClientError) -> bool {
        let res;

        (*self, res) = match self {
            Self::Sending { retry_count } if *retry_count > 0 => (
                Self::Sending {
                    retry_count: *retry_count - 1,
                },
                true,
            ),
            Self::Sending { retry_count: _ } => (Self::Failed(error.to_string()), false),
            Self::WaitingConfirmation { .. } => panic!("Currently in `WaitingConfirmation` state"),
            Self::Success => panic!("Currently in `Success` state"),
            Self::Failed(_) => panic!("Currently in `Failed` state"),
        };

        res
    }

    fn signature_for_status_check(&self) -> &Signature {
        match self {
            Self::Sending { .. } => panic!("Currently in `Sending` state"),
            Self::WaitingConfirmation { signature, .. } => signature,
            Self::Success => panic!("Currently in `Success` state"),
            Self::Failed(_) => panic!("Currently in `Failed` state"),
        }
    }

    fn status_success(&mut self) {
        *self = match self {
            Self::Sending { .. } => panic!("Currently in `Sending` state"),
            Self::WaitingConfirmation { .. } => Self::Success,
            Self::Success => panic!("Currently in `Success` state"),
            Self::Failed(_) => panic!("Currently in `Failed` state"),
        }
    }

    fn status_absent(&mut self) -> StatusAbsentAction {
        // Would be nice to have this delay as a configuration option, similar to the other delays.
        // 5 slots allows us to wait for the next leader, but otherwise it is a rather random
        // choice.  Plus time does not exactly match slots.
        const MAX_ABSENT_SLOTS: u64 = 5;

        match self {
            Self::Sending { .. } => panic!("Currently in `Sending` state"),
            Self::WaitingConfirmation {
                wait_start,
                retry_count,
                ..
            } => {
                if wait_start.elapsed() < Duration::from_millis(MAX_ABSENT_SLOTS * 400) {
                    StatusAbsentAction::WaitMore
                } else if *retry_count > 0 {
                    *self = Self::Sending {
                        retry_count: *retry_count - 1,
                    };
                    StatusAbsentAction::Retry
                } else {
                    *self = Self::Failed(format!(
                        "Transaction not present in the chain even after {MAX_ABSENT_SLOTS} slots"
                    ));
                    StatusAbsentAction::Failed
                }
            }
            Self::Success => panic!("Currently in `Success` state"),
            Self::Failed(_) => panic!("Currently in `Failed` state"),
        }
    }

    fn status_pending(&mut self, new_confirmations: u8) {
        match self {
            Self::Sending { .. } => panic!("Currently in `Sending` state"),
            Self::WaitingConfirmation { confirmations, .. } => {
                *confirmations = Some(new_confirmations)
            }
            Self::Success => panic!("Currently in `Success` state"),
            Self::Failed(_) => panic!("Currently in `Failed` state"),
        }
    }

    fn status_failed(&mut self, error: TransactionError) -> bool {
        let res;
        (*self, res) = match self {
            Self::Sending { .. } => panic!("Currently in `Sending` state"),
            Self::WaitingConfirmation { retry_count, .. } if *retry_count > 0 => (
                Self::Sending {
                    retry_count: *retry_count - 1,
                },
                true,
            ),
            Self::WaitingConfirmation { .. } => (Self::Failed(error.to_string()), false),
            Self::Success => panic!("Currently in `Success` state"),
            Self::Failed(_) => panic!("Currently in `Failed` state"),
        };

        res
    }

    fn status_confirmations(&self) -> u8 {
        match self {
            Self::Sending { .. } => panic!("Currently in `Sending` state"),
            Self::WaitingConfirmation { confirmations, .. } => confirmations.unwrap_or(0),
            Self::Success => panic!("Currently in `Success` state"),
            Self::Failed(_) => panic!("Currently in `Failed` state"),
        }
    }
}

enum StatusAbsentAction {
    WaitMore,
    Retry,
    Failed,
}

enum TxSendResult {
    Success { idx: usize, signature: Signature },
    Fail { idx: usize, error: RpcClientError },
}

impl TxSendResult {
    fn from_result(idx: usize, res: Result<Signature, RpcClientError>) -> Self {
        match res {
            Ok(signature) => Self::Success { idx, signature },
            Err(error) => Self::Fail { idx, error },
        }
    }
}

enum TxStatusResult {
    Success { idx: usize },
    Absent { idx: usize },
    Pending { idx: usize, confirmations: u8 },
    Fail { idx: usize, error: TransactionError },
}
