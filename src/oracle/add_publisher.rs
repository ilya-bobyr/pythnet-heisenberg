use anyhow::{Context as _, Result};
use solana_sdk::signer::Signer as _;

use crate::{
    args::{json_rpc_url_args::get_rpc_client, oracle::add_publisher::AddPublisherArgs},
    keypair_ext::read_keypair_file,
    rpc_client_ext::RpcClientExt as _,
};

use super::instructions::add_publisher;

pub async fn run(
    AddPublisherArgs {
        json_rpc_url,
        program_id,
        permissions_account,
        funding_keypair,
        price_keypair,
        publisher_pubkey,
    }: AddPublisherArgs,
) -> Result<()> {
    let rpc_client = get_rpc_client(json_rpc_url);

    let funding = read_keypair_file(&funding_keypair)?;
    let funding_pubkey = funding.pubkey();

    let price = read_keypair_file(&price_keypair)?;
    let price_pubkey = price.pubkey();

    let signature = rpc_client
        .send_with_payer_latest_blockhash_with_spinner(
            &[add_publisher::instruction(
                program_id,
                funding_pubkey,
                price_pubkey,
                permissions_account,
                publisher_pubkey,
            )],
            Some(&funding_pubkey),
            &[&funding, &price],
        )
        .await
        .context("Transaction execution failed")?;

    println!("Add publisher tx: {signature}");

    Ok(())
}
