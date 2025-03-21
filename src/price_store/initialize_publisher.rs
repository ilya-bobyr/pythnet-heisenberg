use anyhow::{Context as _, Result};
use solana_program::system_instruction;
use solana_sdk::{rent::Rent, signer::Signer as _};

use crate::{
    args::{
        json_rpc_url_args::get_rpc_client,
        price_store::initialize_publisher::InitializePublisherArgs,
    },
    keypair_ext::{read_keypair_file, read_or_generate_keypair_file},
    rpc_client_ext::RpcClientExt as _,
};

use super::instructions::{buffer_account_size, initialize_publisher};

pub async fn run(
    InitializePublisherArgs {
        json_rpc_url,
        program_id,
        payer_keypair,
        authority_keypair,
        publisher_pubkey,
        price_buffer_keypair,
        max_prices,
    }: InitializePublisherArgs,
) -> Result<()> {
    let rpc_client = get_rpc_client(json_rpc_url);

    let payer = read_keypair_file(&payer_keypair)?;
    let payer_pubkey = payer.pubkey();

    let authority = read_keypair_file(&authority_keypair)?;
    let authority_pubkey = authority.pubkey();

    let price_buffer = read_or_generate_keypair_file(&price_buffer_keypair)?;
    let price_buffer_pubkey = price_buffer.pubkey();

    let price_buffer_size = buffer_account_size(max_prices);
    let price_buffer_lamports = Rent::default().minimum_balance(
        usize::try_from(price_buffer_size).expect("Account size fits into a usize"),
    );

    let signature = rpc_client
        .send_with_payer_latest_blockhash_with_spinner(
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
        )
        .await
        .context("Transaction execution failed")?;

    println!("Price Store publisher initialization tx: {signature}");

    Ok(())
}
