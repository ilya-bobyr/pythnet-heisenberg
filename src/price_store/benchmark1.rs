//! Benchmark that sends price updates to the Price Store.
//!
//! It is sending updates in parallel on behalf of each know publisher, for as many prices in each
//! update as specified.  Updates are sent directly to the UDP port of the current leader.
//!
//! Initially price for each product starts at the same specified value, but it drifts over time
//! randomly to make it a bit closer to the actual production cluster behavior.  This part most
//! likely does not matter.

use std::sync::Arc;

use anyhow::Result;
use derive_more::{Add, AddAssign};
use futures::{StreamExt as _, stream::FuturesUnordered};
use itertools::izip;
use log::warn;
use price_publisher::run_publisher;
use tokio::{select, time::sleep};
use tokio_util::sync::CancellationToken;

use crate::{
    args::{json_rpc_url_args::get_rpc_client, price_store::benchmark1::Benchmark1Args},
    blockhash_cache::BlockhashCache,
    keypair_ext::read_keypair_file,
    node_address_service::{NodeAddressService, with_node_address_service},
};

mod price_publisher;
mod price_source;

pub async fn run(
    Benchmark1Args {
        json_rpc_url,
        websocket_url,
        fanout_slots,
        program_id,
        payer_keypair: payer_keypairs,
        publisher_keypair: publisher_keypairs,
        price_buffer_pubkey: price_buffer_pubkeys,
        price_feed_index_start,
        price_feed_index_end,
        price_updates_per_tx,
        update_frequency,
        price_mean,
        price_range,
        confidence_mean,
        confidence_range,
        duration,
    }: Benchmark1Args,
) -> Result<()> {
    let rpc_client = Arc::new(get_rpc_client(json_rpc_url));

    let publishers_shutdown = CancellationToken::new();

    let payers = payer_keypairs
        .into_iter()
        .map(|keypair_file| read_keypair_file(&keypair_file))
        .collect::<Result<Vec<_>>>()?;

    let publishers = publisher_keypairs
        .into_iter()
        .map(|keypair_file| read_keypair_file(&keypair_file))
        .collect::<Result<Vec<_>>>()?;

    let price_feed_indices = price_feed_index_start..=price_feed_index_end;

    let benchmark_start = chrono::Local::now();
    let benchmark_end_timer = sleep(duration.into());
    tokio::pin!(benchmark_end_timer);

    println!("Benchmark start time: {}", benchmark_start);

    let mut stats = RunStats::default();

    let publishers_task = {
        let rpc_client = rpc_client.clone();
        let stats = &mut stats;
        move |blockhash_cache: BlockhashCache, node_address_service: NodeAddressService| {
            async move {
                let mut publishers = izip!(payers, publishers, price_buffer_pubkeys)
                    .map(|(payer, publisher, price_buffer)| {
                        run_publisher(
                            &rpc_client,
                            program_id,
                            payer,
                            publisher,
                            price_buffer,
                            price_feed_indices.clone(),
                            price_updates_per_tx,
                            update_frequency.into(),
                            price_mean,
                            price_range,
                            confidence_mean,
                            confidence_range,
                            &blockhash_cache,
                            &node_address_service,
                            fanout_slots,
                            publishers_shutdown.clone(),
                        )
                    })
                    .collect::<FuturesUnordered<_>>();

                loop {
                    select! {
                        completion_res = publishers.next() => match completion_res {
                            Some(res) => match res {
                                Ok(publisher_stats) => {
                                    *stats += publisher_stats;
                                }
                                Err(err) => {
                                    warn!("Publisher task execution failed: {err}");
                                }
                            }
                            None => {
                                // All publishers are done.
                                break;
                            }
                        },
                        () = &mut benchmark_end_timer, if !benchmark_end_timer.is_elapsed() => {
                            publishers_shutdown.cancel();
                        }
                    }
                }

                // Publishers should not exit by themselves, but it does not hurt to make sure
                // all the exit flags are set at this point.
                publishers_shutdown.cancel();
            }
        }
    };

    with_node_address_service(rpc_client, websocket_url.as_str())
        .run(publishers_task)
        .await?;

    println!("Benchmark end time:   {}", chrono::Local::now());
    println!("      Successful txs: {}", stats.successful_tx);
    println!("          Failed txs: {}", stats.failed_tx);

    Ok(())
}

#[derive(Debug, Clone)]
pub enum PriceUpdateResult {
    Success,
    Fail,
}

impl PriceUpdateResult {
    pub fn from_result<T, E>(result: Result<T, E>) -> Self {
        match result {
            Ok(_) => Self::Success,
            Err(_) => Self::Fail,
        }
    }
}

trait ResultIntoPriceUpdateResult {
    fn into_price_update_result(self) -> PriceUpdateResult;
}

impl<T, E> ResultIntoPriceUpdateResult for Result<T, E> {
    fn into_price_update_result(self) -> PriceUpdateResult {
        PriceUpdateResult::from_result(self)
    }
}

#[derive(Debug, Clone, Default, Add, AddAssign)]
pub struct RunStats {
    successful_tx: u64,
    failed_tx: u64,
}

impl From<PriceUpdateResult> for RunStats {
    fn from(result: PriceUpdateResult) -> Self {
        match result {
            PriceUpdateResult::Success => Self {
                successful_tx: 1,
                failed_tx: 0,
            },
            PriceUpdateResult::Fail => Self {
                successful_tx: 0,
                failed_tx: 1,
            },
        }
    }
}
