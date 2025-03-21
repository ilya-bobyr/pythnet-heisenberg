use std::{iter, time::Duration};

use anyhow::{Context as _, Result};
use futures::{StreamExt as _, stream::FuturesUnordered};
use itertools::izip;
use solana_program::{pubkey::Pubkey, system_instruction};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{rent::Rent, signature::Keypair, signer::Signer as _, transaction::Transaction};
use tokio::{pin, select};
use tokio_util::sync::CancellationToken;

use crate::{
    args::{
        json_rpc_url_args::get_rpc_client,
        price_store::initialize_publisher::InitializePublisherArgs,
    },
    blockhash_cache::BlockhashCache,
    keypair_ext::{read_keypair_file, read_or_generate_keypair_file},
};

use super::instructions::{buffer_account_size, initialize_publisher};

pub async fn run(
    InitializePublisherArgs {
        json_rpc_url,
        program_id,
        payer_keypair,
        authority_keypair,
        publisher_pubkey: publisher_pubkeys,
        price_buffer_keypair: price_buffer_keypairs,
        max_prices,
    }: InitializePublisherArgs,
) -> Result<()> {
    let rpc_client = get_rpc_client(json_rpc_url);

    let services_shutdown = CancellationToken::new();

    let blockhash_cache = BlockhashCache::uninitialized();
    blockhash_cache.init(&rpc_client).await;

    let blockhash_cache_refresh_task = blockhash_cache.run_refresh_loop(
        &rpc_client,
        Duration::from_millis(400),
        services_shutdown.clone(),
    );
    pin!(blockhash_cache_refresh_task);

    let payer = read_keypair_file(&payer_keypair)?;
    let payer_pubkey = payer.pubkey();

    let authority = read_keypair_file(&authority_keypair)?;
    let authority_pubkey = authority.pubkey();

    let price_buffers = price_buffer_keypairs
        .into_iter()
        .map(|keypair| read_or_generate_keypair_file(&keypair))
        .collect::<Result<Vec<_>>>()?;

    let total_initializations = price_buffers.len();

    let mut init_ops = izip!(
        &publisher_pubkeys,
        &price_buffers,
        max_prices.into_iter().chain(iter::repeat(5_000u64))
    )
    .map(|(publisher_pubkey, price_buffer, max_prices)| {
        initialize_one_publisher(
            &rpc_client,
            &blockhash_cache,
            program_id,
            &payer,
            payer_pubkey,
            &authority,
            authority_pubkey,
            *publisher_pubkey,
            &price_buffer,
            max_prices,
        )
    })
    .collect::<FuturesUnordered<_>>();

    let mut successful_tx = 0;
    let mut failed_tx = 0;

    println!(
        "Initializing {} publishers in parallel...",
        total_initializations
    );

    loop {
        select! {
            init_res = init_ops.next() => {
                let Some(init_res) = init_res else {
                    // All operations finished.
                    break;
                };
                match init_res {
                    Ok(InitDetails { publisher, price_buffer }) => {
                        successful_tx += 1;
                        println!(
                            "Initialization {} of {}: Success for publisher {} price_buffer {}",
                            successful_tx + failed_tx,
                            total_initializations,
                            publisher,
                            price_buffer,
                        );
                    },
                    Err(err) => {
                        failed_tx += 1;
                        println!(
                            "Initialization {} of {}: Error: {}",
                            successful_tx + failed_tx,
                            total_initializations,
                            err,
                        );
                    }
                }
            }
            () = &mut blockhash_cache_refresh_task => {
                panic!("BlockhashCache should not stop until requested");
            }
        }
    }

    services_shutdown.cancel();
    blockhash_cache_refresh_task.await;

    Ok(())
}

struct InitDetails {
    publisher: Pubkey,
    price_buffer: Pubkey,
}

async fn initialize_one_publisher(
    rpc_client: &RpcClient,
    blockhash_cache: &BlockhashCache,
    program_id: Pubkey,
    payer: &Keypair,
    payer_pubkey: Pubkey,
    authority: &Keypair,
    authority_pubkey: Pubkey,
    publisher_pubkey: Pubkey,
    price_buffer: &Keypair,
    max_prices: u64,
) -> Result<InitDetails> {
    let price_buffer_pubkey = price_buffer.pubkey();

    let price_buffer_size = buffer_account_size(max_prices);
    let price_buffer_lamports = Rent::default().minimum_balance(
        usize::try_from(price_buffer_size).expect("Account size fits into a usize"),
    );

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer_pubkey,
                &price_buffer_pubkey,
                price_buffer_lamports,
                price_buffer_size,
                &program_id,
            ),
            initialize_publisher::instruction(
                program_id,
                authority_pubkey,
                publisher_pubkey,
                price_buffer_pubkey,
            ),
        ],
        Some(&payer_pubkey),
        &[&payer, &price_buffer, &authority],
        blockhash_cache.get(),
    );

    let _signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .context("Transaction execution failed")?;

    Ok(InitDetails {
        publisher: publisher_pubkey,
        price_buffer: price_buffer_pubkey,
    })
}
