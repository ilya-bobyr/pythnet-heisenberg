use clap::Args;
use solana_program::pubkey::Pubkey;

use crate::args::JsonRpcUrlArgs;

#[derive(Args, Debug)]
pub struct GetPriceFeedIndexArgs {
    #[command(flatten)]
    pub json_rpc_url: JsonRpcUrlArgs,

    /// An address of a price account to get the price feed index for.
    #[arg(long)]
    pub price_pubkey: Pubkey,
}
