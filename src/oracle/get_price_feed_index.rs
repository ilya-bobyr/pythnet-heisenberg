use anyhow::{Context as _, Result};
use bytemuck::from_bytes;

use crate::{
    args::{
        json_rpc_url_args::get_rpc_client, oracle::get_price_feed_index::GetPriceFeedIndexArgs,
    },
    oracle::accounts::price::PriceAccount,
};

pub async fn run(
    GetPriceFeedIndexArgs {
        json_rpc_url,
        price_pubkey,
    }: GetPriceFeedIndexArgs,
) -> Result<()> {
    let rpc_client = get_rpc_client(json_rpc_url);

    let account = rpc_client
        .get_account(&price_pubkey)
        .await
        .with_context(|| format!("Failed to fetch account at {price_pubkey}"))?;

    let price_account: &PriceAccount = from_bytes(&account.data);

    println!("{}", price_account.feed_index);

    Ok(())
}
