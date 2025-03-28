//! Benchmark that sends price updates to the Price Store.
//!
//! It is sending updates in parallel on behalf of each know publisher, for as many prices in each
//! update as specified.  Updates are sent directly to the UDP port of the current leader.
//!
//! Initially price for each product starts at the same specified value, but it drifts over time
//! randomly to make it a bit closer to the actual production cluster behavior.  This part most
//! likely does not matter.

use std::{sync::Arc, time::Duration};

use anyhow::{Context as _, Result};
use chrono;
use derive_more::{Add, AddAssign};
use futures::{StreamExt as _, stream::FuturesUnordered};
use itertools::izip;
use log::warn;
use price_publisher::run_publisher;
use solana_sdk::signer::Signer as _;
use tokio::{select, time::sleep};
use tokio_util::sync::CancellationToken;

use crate::{
    args::{json_rpc_url_args::get_rpc_client, price_store::benchmark1::Benchmark1Args},
    blockhash_cache::BlockhashCache,
    keypair_ext::read_keypair_file,
    node_address_service::NodeAddressService,
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

    let exit = CancellationToken::new();

    let blockhash_cache = BlockhashCache::uninitialized();
    blockhash_cache.init(&rpc_client).await;

    let blockhash_cache_refresh_task =
        blockhash_cache.run_refresh_loop(&rpc_client, Duration::from_millis(400), exit.clone());

    let mut node_address_service =
        NodeAddressService::new(rpc_client.clone(), websocket_url.as_str(), exit.clone())
            .await
            .context("NodeAddressService construction failed")?;

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

    let mut publishers = izip!(payers, publishers, price_buffer_pubkeys)
        .map(|(payer, publisher, price_buffer)| {
            //- println!(
            //-     "D.run.1: Starting run_publisher({}, {}, {}) task.",
            //-     payer.pubkey(),
            //-     publisher.pubkey(),
            //-     price_buffer,
            //- );
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
                exit.clone(),
            )
        })
        .collect::<FuturesUnordered<_>>();

    let mut stats = RunStats::default();

    loop {
        select! {
            completion_res = publishers.next() => match completion_res {
                Some(res) => match res {
                    Ok(publisher_stats) => {
                        //- println!("D.run.2: Publisher task terminated");
                        stats += publisher_stats;
                    }
                    Err(err) => {
                        //- println!("D.run.2: Publisher task failed");
                        warn!("Publisher task execution failed: {err}");
                    }
                }
                None => {
                    //- println!("D.run.3: All publishers task are done");
                    break;
                }
            },
            () = &mut benchmark_end_timer, if !benchmark_end_timer.is_elapsed() => {
                //- println!("D.run.4: Run time ended, setting the exit flag");
                exit.cancel();
            }
        }
    }

    // Publishers should not exit by themselves, but if they do, make sure other tasks also start
    // their shutdown sequence.
    exit.cancel();

    drop(publishers);

    node_address_service
        .join()
        .await
        .context("Waiting for the NodeAddressService task to stop")?;
    blockhash_cache_refresh_task.await;

    println!("Benchmark end time:   {}", chrono::Local::now());
    println!("      Successful txs: {}", stats.successful_tx);
    println!("          Failed txs: {}", stats.failed_tx);

    Ok(())
}

#[derive(Debug, Clone, Default, Add, AddAssign)]
pub struct RunStats {
    successful_tx: u64,
    failed_tx: u64,
}

impl RunStats {
    pub fn successful_tx() -> Self {
        Self {
            successful_tx: 1,
            failed_tx: 0,
        }
    }

    pub fn failed_tx() -> Self {
        Self {
            successful_tx: 0,
            failed_tx: 1,
        }
    }
}
