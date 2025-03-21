use anyhow::{Context as _, Result};
use solana_sdk::signer::Signer as _;

use crate::{
    args::{json_rpc_url_args::get_rpc_client, price_store::initialize::InitializeArgs},
    keypair_ext::read_keypair_file,
    rpc_client_ext::RpcClientExt as _,
};

use super::instructions::initialize;

pub async fn run(
    InitializeArgs {
        json_rpc_url,
        program_id,
        payer_keypair,
        authority,
    }: InitializeArgs,
) -> Result<()> {
    let rpc_client = get_rpc_client(json_rpc_url);

    let payer = read_keypair_file(&payer_keypair)?;
    let payer_pubkey = payer.pubkey();

    let signature = rpc_client
        .send_with_payer_latest_blockhash_with_spinner(
            &[initialize::instruction(program_id, payer_pubkey, authority)],
            Some(&payer_pubkey),
            &[&payer],
        )
        .await
        .context("Transaction execution failed")?;

    println!("Price Store initialization tx: {signature}");

    Ok(())
}
