use anyhow::{Context as _, Result};
use solana_sdk::signer::Signer as _;

use crate::{
    args::{json_rpc_url_args::get_rpc_client, price_store::submit_prices::SubmitPricesArgs},
    keypair_ext::read_keypair_file,
    rpc_client_ext::RpcClientExt as _,
};

use super::instructions::submit_prices;

pub async fn run(
    SubmitPricesArgs {
        json_rpc_url,
        program_id,
        payer_keypair,
        publisher_keypair,
        price_buffer_pubkey,
        price: prices,
    }: SubmitPricesArgs,
) -> Result<()> {
    let rpc_client = get_rpc_client(json_rpc_url);

    let payer = read_keypair_file(&payer_keypair)?;
    let payer_pubkey = payer.pubkey();

    let publisher = read_keypair_file(&publisher_keypair)?;
    let publisher_pubkey = publisher.pubkey();

    let signature = rpc_client
        .send_with_payer_latest_blockhash_with_spinner(
            &[submit_prices::instruction(
                program_id,
                publisher_pubkey,
                price_buffer_pubkey,
                &prices,
            )],
            Some(&payer_pubkey),
            &[&payer, &publisher],
        )
        .await
        .context("Transaction execution failed")?;

    println!("Price Store submit price tx: {signature}");

    Ok(())
}
