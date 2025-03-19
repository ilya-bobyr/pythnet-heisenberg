use anyhow::{Context as _, Result};
use futures::{StreamExt as _, stream::FuturesUnordered};
use itertools::izip;
use solana_program::pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{signature::Keypair, signer::Signer as _, transaction::Transaction};

use crate::{
    args::{json_rpc_url_args::get_rpc_client, oracle::add_publisher::AddPublisherArgs},
    blockhash_cache::{BlockhashCache, with_blockhash},
    keypair_ext::read_keypair_file,
};

use super::instructions::add_publisher;

pub async fn run(
    AddPublisherArgs {
        json_rpc_url,
        program_id,
        permissions_account,
        funding_keypair,
        price_keypair: price_keypairs,
        publisher_pubkey: publisher_pubkeys,
    }: AddPublisherArgs,
) -> Result<()> {
    let rpc_client = get_rpc_client(json_rpc_url);
    let rpc_client = &rpc_client;

    let funding = read_keypair_file(&funding_keypair)?;
    let funding_pubkey = funding.pubkey();

    let prices = price_keypairs
        .into_iter()
        .map(|keypair| read_keypair_file(&keypair))
        .collect::<Result<Vec<_>>>()?;

    let total_additions = prices.len();

    let mut successful_tx = 0;
    let mut failed_tx = 0;

    println!("Adding {} publishers in parallel...", total_additions);

    with_blockhash(rpc_client)
        .run(async move |blockhash_cache: &BlockhashCache| {
            let mut add_ops = izip!(&prices, &publisher_pubkeys)
                .map(|(price, publisher_pubkey)| {
                    add_one_publisher(
                        rpc_client,
                        blockhash_cache,
                        program_id,
                        permissions_account,
                        &funding,
                        funding_pubkey,
                        price,
                        *publisher_pubkey,
                    )
                })
                .collect::<FuturesUnordered<_>>();

            while let Some(add_res) = add_ops.next().await {
                match add_res {
                    Ok(AddDetails { price, publisher }) => {
                        successful_tx += 1;
                        println!(
                            "Add {} of {}: Success for price {} publisher {}",
                            successful_tx + failed_tx,
                            total_additions,
                            price,
                            publisher,
                        );
                    }
                    Err(err) => {
                        failed_tx += 1;
                        println!(
                            "Add {} of {}: Error: {}",
                            successful_tx + failed_tx,
                            total_additions,
                            err,
                        );
                    }
                }
            }
        })
        .await;

    Ok(())
}

struct AddDetails {
    price: Pubkey,
    publisher: Pubkey,
}

#[allow(clippy::too_many_arguments)]
async fn add_one_publisher(
    rpc_client: &RpcClient,
    blockhash_cache: &BlockhashCache,
    program_id: Pubkey,
    permissions_account: Option<Pubkey>,
    funding_keypair: &Keypair,
    funding_pubkey: Pubkey,
    price_keypair: &Keypair,
    publisher_pubkey: Pubkey,
) -> Result<AddDetails> {
    let price_pubkey = price_keypair.pubkey();

    let transaction = Transaction::new_signed_with_payer(
        &[add_publisher::instruction(
            program_id,
            funding_pubkey,
            price_pubkey,
            permissions_account,
            publisher_pubkey,
        )],
        Some(&funding_pubkey),
        &[&funding_keypair, &price_keypair],
        blockhash_cache.get(),
    );

    let _signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .context("Transaction execution failed")?;

    Ok(AddDetails {
        price: price_pubkey,
        publisher: publisher_pubkey,
    })
}
