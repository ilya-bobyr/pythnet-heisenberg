use anyhow::{Context as _, Result};
use solana_program::system_instruction;
use solana_rpc_client::{
    http_sender::HttpSender, nonblocking::rpc_client::RpcClient, rpc_client::RpcClientConfig,
};
use solana_sdk::{rent::Rent, signer::Signer as _};

use crate::{
    args::{JsonRpcUrlArgs, oracle::init_mapping::InitMappingArgs},
    keypair_ext::read_keypair_file,
    rpc_client_ext::RpcClientExt as _,
};

use super::instructions::{MAPPING_ACCOUNT_MIN_SIZE, init_mapping_instruction};

pub async fn run(
    InitMappingArgs {
        json_rpc_url: JsonRpcUrlArgs { rpc_url },
        program_id,
        permissions_account,
        funding_keypair,
        mapping_keypair,
    }: InitMappingArgs,
) -> Result<()> {
    let rpc_client = RpcClient::new_sender(HttpSender::new(rpc_url), RpcClientConfig::default());

    let funding = read_keypair_file(&funding_keypair)?;
    let funding_pubkey = funding.pubkey();

    let mapping = read_keypair_file(&mapping_keypair)?;
    let mapping_pubkey = mapping.pubkey();

    let mapping_size = MAPPING_ACCOUNT_MIN_SIZE;
    let mapping_lamports = Rent::default()
        .minimum_balance(usize::try_from(mapping_size).expect("Account size fits into a usize"));

    let signature = rpc_client
        .send_with_payer_latest_blockhash_with_spinner(
            &[
                system_instruction::create_account(
                    &funding_pubkey,
                    &mapping_pubkey,
                    mapping_lamports,
                    mapping_size,
                    &program_id,
                ),
                init_mapping_instruction(
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
