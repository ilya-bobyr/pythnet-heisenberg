use anyhow::{Context as _, Result};
use solana_rpc_client::{
    http_sender::HttpSender, nonblocking::rpc_client::RpcClient, rpc_client::RpcClientConfig,
};
use solana_sdk::signer::Signer as _;

use crate::{
    args::{JsonRpcUrlArgs, oracle::update_permissions::UpdatePermissionsArgs},
    keypair_ext::read_keypair_file,
    rpc_client_ext::RpcClientExt as _,
};

use super::instructions::update_permissions_instruction;

pub async fn run(
    UpdatePermissionsArgs {
        json_rpc_url: JsonRpcUrlArgs { rpc_url },
        program_id,
        funding_keypair,
        permissions_account,
        master_authority,
        data_curation_authority,
        security_authority,
    }: UpdatePermissionsArgs,
) -> Result<()> {
    let rpc_client = RpcClient::new_sender(HttpSender::new(rpc_url), RpcClientConfig::default());

    let funding = read_keypair_file(&funding_keypair)?;
    let funding_pubkey = funding.pubkey();

    let signature = rpc_client
        .send_with_payer_latest_blockhash_with_spinner(
            &[update_permissions_instruction(
                program_id,
                funding_pubkey,
                permissions_account,
                master_authority,
                data_curation_authority,
                security_authority,
            )],
            Some(&funding_pubkey),
            &[&funding],
        )
        .await
        .context("Transaction execution failed")?;

    println!("Oracle permissions update tx: {signature}");

    Ok(())
}
