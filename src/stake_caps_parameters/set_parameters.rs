use anchor_lang::{InstructionData, ToAccountMetas};
use anyhow::{Context as _, Result, anyhow};
use solana_program::{instruction::Instruction, system_program};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::config::RpcSendTransactionConfig;
use solana_sdk::{
    pubkey::Pubkey,
    signer::{Signer as _, keypair::read_keypair_file},
    transaction::Transaction,
};
use stake_caps_parameters as program;

use crate::args::stake_caps_parameters::set_parameters::SetParametersArgs;

pub async fn run(
    rpc_client: &RpcClient,
    SetParametersArgs {
        signer_keypair,
        program_id,
        parameters_account,
        m,
        z,
        update_authority,
    }: SetParametersArgs,
) -> Result<()> {
    let signer = read_keypair_file(&signer_keypair)
        // It is a bit strange, but `Box<dyn Error>` does not implement `Error` for some reason.
        // And `anyhow::Context::with_context` fails.  So I need to construct a new `Error`
        // instance explicitly here.
        .map_err(|err| anyhow!(err.to_string()))
        .with_context(|| {
            format!(
                "Error reading a keypair from: {}",
                signer_keypair.to_string_lossy()
            )
        })?;

    let signer_pubkey = signer.pubkey();

    let parameters_account = parameters_account
        .unwrap_or_else(|| Pubkey::find_program_address(&[b"parameters"], &program_id).0);

    let accounts = stake_caps_parameters::accounts::SetParameters {
        signer: signer_pubkey,
        parameters: parameters_account,
        system_program: system_program::id(),
    };

    let instruction = Instruction {
        program_id,
        accounts: accounts.to_account_metas(None),
        data: program::instruction::SetParameters {
            parameters: program::Parameters {
                m,
                z,
                current_authority: update_authority.unwrap_or(signer_pubkey),
            },
        }
        .data(),
    };

    let transaction = {
        let latest_blockhash = rpc_client
            .get_latest_blockhash()
            .await
            .context("Getting a blockhash from the cluster")?;
        Transaction::new_signed_with_payer(
            &[instruction],
            Some(&signer_pubkey),
            &[&signer],
            latest_blockhash,
        )
    };

    let transaction_signature = rpc_client
        .send_and_confirm_transaction_with_spinner_and_config(
            &transaction,
            rpc_client.commitment(),
            RpcSendTransactionConfig {
                skip_preflight: true,
                ..RpcSendTransactionConfig::default()
            },
        )
        .await
        .context("Transaction execution failed")?;

    println!("State cap parameters update tx: {transaction_signature}");

    Ok(())
}
