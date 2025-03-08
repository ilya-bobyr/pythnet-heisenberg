use anchor_lang::{InstructionData, ToAccountMetas};
use anyhow::{Context as _, Result};
use solana_program::{instruction::Instruction, pubkey::Pubkey, system_program};
use solana_sdk::signer::Signer as _;
use stake_caps_parameters as program;

use crate::{
    args::{
        json_rpc_url_args::get_rpc_client, stake_caps_parameters::set_parameters::SetParametersArgs,
    },
    keypair_ext::read_keypair_file,
    rpc_client_ext::RpcClientExt as _,
};

pub async fn run(
    SetParametersArgs {
        json_rpc_url,
        signer_keypair,
        program_id,
        parameters_account,
        m,
        z,
        update_authority,
    }: SetParametersArgs,
) -> Result<()> {
    let rpc_client = get_rpc_client(json_rpc_url);

    let signer = read_keypair_file(&signer_keypair)?;
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

    let signature = rpc_client
        .send_with_payer_latest_blockhash_with_spinner(
            &[instruction],
            Some(&signer_pubkey),
            &[&signer],
        )
        .await
        .context("Transaction execution failed")?;

    println!("State cap parameters update tx: {signature}");

    Ok(())
}
