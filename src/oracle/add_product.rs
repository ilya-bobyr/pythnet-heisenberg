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
    args::{
        json_rpc_url_args::get_rpc_client,
        oracle::add_product::{AddProductArgs, per_product_metadata},
    },
    blockhash_cache::BlockhashCache,
    keypair_ext::{read_keypair_file, read_or_generate_keypair_file},
};

use super::instructions::add_product::{self, ACCOUNT_MIN_SIZE};

pub async fn run(
    AddProductArgs {
        json_rpc_url,
        program_id,
        permissions_account,
        funding_keypair,
        mapping_keypair,
        product_keypair: product_keypairs,
        metadata,
    }: AddProductArgs,
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

    let mapping = read_keypair_file(&mapping_keypair)?;
    let mapping_pubkey = mapping.pubkey();

    let products = product_keypairs
        .into_iter()
        .map(|keypair| read_or_generate_keypair_file(&keypair))
        .collect::<Result<Vec<_>>>()?;

    let metadata = per_product_metadata(&metadata);

    let account_size = ACCOUNT_MIN_SIZE;
    let account_lamports = Rent::default()
        .minimum_balance(usize::try_from(account_size).expect("Account size fits into a usize"));

    let total_additions = products.len();

    let mut add_ops = izip!(&products, &metadata)
        .map(|(product, metadata)| {
            add_one_product(
                &rpc_client,
                &blockhash_cache,
                program_id,
                permissions_account.clone(),
                &funding,
                funding_pubkey,
                &mapping,
                mapping_pubkey,
                product,
                metadata,
                account_size,
                account_lamports,
            )
        })
        .collect::<FuturesUnordered<_>>();

    let mut successful_tx = 0;
    let mut failed_tx = 0;

    loop {
        select! {
            add_res = add_ops.next() => match add_res {
                Some(res) => match res {
                    Ok(product_pubkey) => {
                        successful_tx += 1;
                        println!(
                            "Add {} of {}: Success for product {}",
                            successful_tx + failed_tx,
                            total_additions,
                            product_pubkey,
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
                None => break,
            },
            () = &mut blockhash_cache_refresh_task => {
                panic!("BlockhashCache should not stop until requested");
            }
        }
    }

    services_shutdown.cancel();
    blockhash_cache_refresh_task.await;

    Ok(())
}

async fn add_one_product(
    rpc_client: &RpcClient,
    blockhash_cache: &BlockhashCache,
    program_id: Pubkey,
    permissions_account: Option<Pubkey>,
    funding_keypair: &Keypair,
    funding_pubkey: Pubkey,
    mapping_keypair: &Keypair,
    mapping_pubkey: Pubkey,
    product_keypair: &Keypair,
    metadata: &[(&str, &str)],
    account_size: u64,
    account_lamports: u64,
) -> Result<Pubkey> {
    let product_pubkey = product_keypair.pubkey();

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &funding_pubkey,
                &product_pubkey,
                account_lamports,
                account_size,
                &program_id,
            ),
            add_product::instruction(
                program_id,
                funding_pubkey,
                mapping_pubkey,
                product_pubkey,
                permissions_account,
                metadata,
            ),
        ],
        Some(&funding_pubkey),
        &[&funding_keypair, &mapping_keypair, &product_keypair],
        blockhash_cache.get(),
    );

    let _signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .context("Transaction execution failed")?;

    Ok(product_pubkey)
}
