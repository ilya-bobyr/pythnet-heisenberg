use anyhow::{Context as _, Result};
use solana_program::system_instruction;
use solana_sdk::{rent::Rent, signer::Signer as _};

use crate::{
    args::{json_rpc_url_args::get_rpc_client, oracle::init_mapping::InitMappingArgs},
    keypair_ext::{read_keypair_file, read_or_generate_keypair_file},
    rpc_client_ext::RpcClientExt as _,
};

use super::instructions::init_mapping::{self, ACCOUNT_MIN_SIZE};

pub async fn run(
    InitMappingArgs {
        json_rpc_url,
        program_id,
        permissions_account,
        funding_keypair,
        mapping_keypair,
    }: InitMappingArgs,
) -> Result<()> {
    let rpc_client = get_rpc_client(json_rpc_url);

    let funding = read_keypair_file(&funding_keypair)?;
    let funding_pubkey = funding.pubkey();

    let mapping = read_or_generate_keypair_file(&mapping_keypair)?;
    let mapping_pubkey = mapping.pubkey();

    let account_size = ACCOUNT_MIN_SIZE;
    let account_lamports = Rent::default()
        .minimum_balance(usize::try_from(account_size).expect("Account size fits into a usize"));

    let signature = rpc_client
        .send_with_payer_latest_blockhash_with_spinner(
            &[
                system_instruction::create_account(
                    &funding_pubkey,
                    &mapping_pubkey,
                    account_lamports,
                    account_size,
                    &program_id,
                ),
                init_mapping::instruction(
                    program_id,
                    funding_pubkey,
                    mapping_pubkey,
                    permissions_account,
                ),
            ],
            Some(&funding_pubkey),
            &[&funding, &mapping],
        )
        .await
        .context("Transaction execution failed")?;

    println!("Init mapping tx: {signature}");

    Ok(())
}
