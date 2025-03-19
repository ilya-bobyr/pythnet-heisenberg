use anyhow::{Context as _, Result};
use solana_program::system_instruction;
use solana_sdk::{rent::Rent, signer::Signer as _};

use crate::{
    args::{json_rpc_url_args::get_rpc_client, oracle::add_price::AddPriceArgs},
    keypair_ext::{read_keypair_file, read_or_generate_keypair_file},
    rpc_client_ext::RpcClientExt as _,
};

use super::instructions::add_price::{self, ACCOUNT_MIN_SIZE};

pub async fn run(
    AddPriceArgs {
        json_rpc_url,
        program_id,
        permissions_account,
        funding_keypair,
        product_pubkey,
        price_keypair,
        exponent,
    }: AddPriceArgs,
) -> Result<()> {
    let rpc_client = get_rpc_client(json_rpc_url);

    let funding = read_keypair_file(&funding_keypair)?;
    let funding_pubkey = funding.pubkey();

    let price = read_or_generate_keypair_file(&price_keypair)?;
    let price_pubkey = price.pubkey();

    let account_size = ACCOUNT_MIN_SIZE;
    let account_lamports = Rent::default()
        .minimum_balance(usize::try_from(account_size).expect("Account size fits into a usize"));

    let signature = rpc_client
        .send_with_payer_latest_blockhash_with_spinner(
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
                    price_pubkey,
                    permissions_account,
                    exponent,
                ),
            ],
            Some(&funding_pubkey),
            &[&funding, &price],
        )
        .await
        .context("Transaction execution failed")?;

    println!("Add price tx: {signature}");

    Ok(())
}
