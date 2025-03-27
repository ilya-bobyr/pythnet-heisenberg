use std::{
    net::{Ipv4Addr, SocketAddr},
    ops::RangeInclusive,
    time::{Duration, Instant},
};

use anyhow::{Context as _, Result};
use bincode::{self, serde::encode_to_vec};
use futures::{
    future::BoxFuture,
    stream::{FuturesUnordered, StreamExt as _},
};
use log::warn;
use solana_program::{hash::Hash, pubkey::Pubkey};
use solana_sdk::{
    clock::NUM_CONSECUTIVE_LEADER_SLOTS, signature::Keypair, signer::Signer as _,
    transaction::Transaction,
};
use tokio::{net::UdpSocket, select, sync::mpsc, time::sleep};
use tokio_util::sync::CancellationToken;

use crate::{
    blockhash_cache::BlockhashCache,
    node_address_service::NodeAddressService,
    price_store::instructions::submit_prices::{self, BufferedPrice, TradingStatus},
};

use super::{PriceUpdateResult, price_source::PriceSource};

#[allow(clippy::too_many_arguments)]
pub async fn run_publisher(
    program_id: Pubkey,
    payer: Keypair,
    publisher: Keypair,
    price_buffer: Pubkey,
    price_feed_indices: RangeInclusive<u32>,
    price_updates_per_tx: u8,
    update_frequency: Duration,
    price_mean: i64,
    price_range: u64,
    confidence_mean: u64,
    confidence_range: u64,
    blockhash_cache: &BlockhashCache,
    node_address_service: &NodeAddressService,
    fanout_slots: u8,
    update_results_consumer: mpsc::Sender<PriceUpdateResult>,
    exit: CancellationToken,
) -> Result<()> {
    let payer_pubkey = payer.pubkey();
    let publisher_pubkey = publisher.pubkey();

    let price_sources = price_feed_indices
        .map(|price_feed_index| {
            PriceSource::new(
                price_feed_index,
                price_mean,
                price_range,
                confidence_mean,
                confidence_range,
            )
        })
        .collect::<Vec<_>>();

    let start_time = Instant::now();

    // This socket will be used by all the publisher requests.
    //
    // Socket will be bound to a specific interface on the first `send_to()` call.  And we then
    // assume that all nodes are reachable over the same network interface and the network
    // configuration does not change in such a way that the send interface needs to be updated.
    let send_socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
        .await
        .context("Creation of a UDP socket")?;

    let mut pending_price_updates = PriceUpdateFutures::new();
    // We should not see more than 2 nodes as our send target, as we are going to query leaders for
    // the next 4 slots only.
    let mut target_nodes = Vec::with_capacity(
        usize::try_from(u64::from(fanout_slots) / NUM_CONSECUTIVE_LEADER_SLOTS)
            .expect("`fanout_slots / NUM_CONSECUTIVE_LEADER_SLOTS` fits into a usize"),
    );

    'publishing_all: loop {
        let iteration_start_time = Instant::now();

        let latest_blockhash = blockhash_cache.get();
        target_nodes.clear();
        node_address_service.get_tpu_for_next_in_schedule(&mut target_nodes, fanout_slots.into());

        start_all_price_updates(
            &mut pending_price_updates,
            &send_socket,
            latest_blockhash,
            &target_nodes,
            (iteration_start_time - start_time).as_secs_f64(),
            program_id,
            &payer,
            payer_pubkey,
            &publisher,
            publisher_pubkey,
            price_buffer,
            price_updates_per_tx,
            &price_sources,
        )
        .context("start_all_price_updates()")?;

        // Wait for all the updates of this iteration to finish.
        'all_iteration_updates: loop {
            select! {
                send_task_res = pending_price_updates.next() => match send_task_res {
                    Some(send_result) => {
                        // Another send is done, keep waiting.
                        match update_results_consumer.send(send_result).await {
                            Ok(()) => (),
                            Err(_) => break,
                        }
                    }
                    None => {
                        // All updates are done.
                        break 'all_iteration_updates;
                    }
                },
                _ = exit.cancelled() => break 'publishing_all,
            }
        }

        let iteration_time_left = update_frequency.saturating_sub(iteration_start_time.elapsed());
        if !iteration_time_left.is_zero() {
            select! {
                _ = sleep(iteration_time_left) => (),
                _ = exit.cancelled() => break 'publishing_all,
            }
        }
    }

    Ok(())
}

type PriceUpdateFutures<'env> = FuturesUnordered<BoxFuture<'env, PriceUpdateResult>>;

#[allow(clippy::too_many_arguments)]
fn start_all_price_updates<'update_deps, 'socket: 'update_deps>(
    price_updates: &mut PriceUpdateFutures<'update_deps>,
    socket: &'socket UdpSocket,
    latest_blockhash: Hash,
    target_nodes: &[SocketAddr],
    time: f64,
    program_id: Pubkey,
    payer: &Keypair,
    payer_pubkey: Pubkey,
    publisher_keypair: &Keypair,
    publisher_pubkey: Pubkey,
    price_buffer_pubkey: Pubkey,
    price_updates_per_tx: u8,
    price_sources: &[PriceSource],
) -> Result<()> {
    let prices = price_sources
        .iter()
        .map(|price_source| {
            let (price, confidence) = price_source.get(time);

            BufferedPrice::new(
                TradingStatus::Trading,
                price_source.price_feed_index,
                price,
                confidence,
            )
        })
        .collect::<Vec<_>>();

    for prices in prices.chunks(price_updates_per_tx.into()) {
        let transaction = Transaction::new_signed_with_payer(
            &[submit_prices::instruction(
                program_id,
                publisher_pubkey,
                price_buffer_pubkey,
                prices,
            )],
            Some(&payer_pubkey),
            &[&payer, &publisher_keypair],
            latest_blockhash,
        );

        let buf = encode_to_vec(transaction, bincode::config::legacy())
            .context("Serialization of the submit prices transaction")?;
        for node_address in target_nodes.iter().copied() {
            price_updates.push({
                let buf = buf.clone();
                Box::pin(async move {
                    match socket.send_to(&buf, node_address).await {
                        Ok(sent) => {
                            if sent != buf.len() {
                                warn!("Failed to send a submit price transaction in one packet");
                                PriceUpdateResult::Fail
                            } else {
                                PriceUpdateResult::Success
                            }
                        }
                        Err(_) => {
                            // We do not care if the send fails.  We are not going to retry it.
                            PriceUpdateResult::Fail
                        }
                    }
                })
            });
        }
    }

    Ok(())
}
