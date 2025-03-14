use anyhow::{Context as _, Result};
use solana_sdk::signer::Signer as _;

use crate::{
    args::{json_rpc_url_args::get_rpc_client, oracle::update_permissions::UpdatePermissionsArgs},
    keypair_ext::read_keypair_file,
    rpc_client_ext::RpcClientExt as _,
};

use super::instructions::update_permissions;

pub async fn run(
    UpdatePermissionsArgs {
        json_rpc_url,
        program_id,
        funding_keypair,
        permissions_account,
        master_authority,
        data_curation_authority,
        security_authority,
    }: UpdatePermissionsArgs,
) -> Result<()> {
    let rpc_client = get_rpc_client(json_rpc_url);

    let funding = read_keypair_file(&funding_keypair)?;
    let funding_pubkey = funding.pubkey();

    let signature = rpc_client
        .send_with_payer_latest_blockhash_with_spinner(
            &[update_permissions::instruction(
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
