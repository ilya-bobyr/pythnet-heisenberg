use std::time::Duration;

use anyhow::{Context as _, Result};
use futures::{StreamExt as _, stream::FuturesUnordered};
use itertools::izip;
use solana_program::{pubkey::Pubkey, system_instruction};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{rent::Rent, signature::Keypair, signer::Signer as _, transaction::Transaction};
use tokio::{pin, select};
use tokio_util::sync::CancellationToken;

use crate::{
    args::{json_rpc_url_args::get_rpc_client, oracle::add_price::AddPriceArgs},
    blockhash_cache::BlockhashCache,
    keypair_ext::{read_keypair_file, read_or_generate_keypair_file},
};

use super::instructions::add_price::{self, ACCOUNT_MIN_SIZE};

pub async fn run(
    AddPriceArgs {
        json_rpc_url,
        program_id,
        permissions_account,
        funding_keypair,
        product_pubkey: product_pubkeys,
        price_keypair: price_keypairs,
        exponent: exponents,
    }: AddPriceArgs,
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

    let funding = read_keypair_file(&funding_keypair)?;
    let funding_pubkey = funding.pubkey();

    let prices = price_keypairs
        .into_iter()
        .map(|keypair| read_or_generate_keypair_file(&keypair))
        .collect::<Result<Vec<_>>>()?;

    let account_size = ACCOUNT_MIN_SIZE;
    let account_lamports = Rent::default()
        .minimum_balance(usize::try_from(account_size).expect("Account size fits into a usize"));

    let total_additions = prices.len();

    let mut add_ops = izip!(&product_pubkeys, &prices, &exponents)
        .map(|(product_pubkey, price, exponent)| {
            add_one_price(
                &rpc_client,
                &blockhash_cache,
                program_id,
                permissions_account.clone(),
                &funding,
                funding_pubkey,
                *product_pubkey,
                price,
                *exponent,
                account_size,
                account_lamports,
            )
        })
        .collect::<FuturesUnordered<_>>();

    let mut successful_tx = 0;
    let mut failed_tx = 0;

    println!("Adding {} prices in parallel...", total_additions);

    loop {
        select! {
            add_res = add_ops.next() => {
                let Some(add_res) = add_res else {
                    // All operations finished.
                    break;
                };
                match add_res {
                    Ok(AddDetails { product, price }) => {
                        successful_tx += 1;
                        println!(
                            "Add {} of {}: Success for product {} price {}",
                            successful_tx + failed_tx,
                            total_additions,
                            product,
                            price,
                        );
                    },
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
            () = &mut blockhash_cache_refresh_task => {
                panic!("BlockhashCache should not stop until requested");
            }
        }
    }

    services_shutdown.cancel();
    blockhash_cache_refresh_task.await;

    Ok(())
}

struct AddDetails {
    product: Pubkey,
    price: Pubkey,
}

async fn add_one_price(
    rpc_client: &RpcClient,
    blockhash_cache: &BlockhashCache,
    program_id: Pubkey,
    permissions_account: Option<Pubkey>,
    funding_keypair: &Keypair,
    funding_pubkey: Pubkey,
    product_pubkey: Pubkey,
    price_keypair: &Keypair,
    exponent: i32,
    account_size: u64,
    account_lamports: u64,
) -> Result<AddDetails> {
    let price_pubkey = price_keypair.pubkey();

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &funding_pubkey,
                &price_pubkey,
                account_lamports,
                account_size,
                &program_id,
            ),
            add_price::instruction(
                program_id,
                funding_pubkey,
                product_pubkey,
                price_pubkey.clone(),
                permissions_account,
                exponent,
            ),
        ],
        Some(&funding_pubkey),
        &[&funding_keypair, &price_keypair],
        blockhash_cache.get(),
    );

    let _signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .context("Transaction execution failed")?;

    Ok(AddDetails {
        product: product_pubkey,
        price: price_pubkey,
    })
}
