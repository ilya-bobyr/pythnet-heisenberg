use clap::Args;
use reqwest::Url;

/// A common argument used by multiple different commands.
#[derive(Args, Debug)]
pub struct JsonRpcUrlArgs {
    #[arg(long, value_name = "URL", default_value = "http://localhost:8899")]
    /// An HTTP address of the Pythnet node that speaks Solana RPC.
    pub rpc_url: Url,
}
