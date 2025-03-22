use clap::Args;
use reqwest::Url;
use solana_rpc_client::{
    http_sender::HttpSender, nonblocking::rpc_client::RpcClient, rpc_client::RpcClientConfig,
};
use solana_sdk::commitment_config::CommitmentConfig;

/// A common argument used by multiple different commands.
#[derive(Args, Debug)]
pub struct JsonRpcUrlArgs {
    #[arg(long, value_name = "URL", default_value = "http://localhost:8899")]
    /// An HTTP address of the Pythnet node that speaks Solana RPC.
    pub rpc_url: Url,
}

pub fn get_rpc_client(JsonRpcUrlArgs { rpc_url }: JsonRpcUrlArgs) -> RpcClient {
    RpcClient::new_sender(
        HttpSender::new(rpc_url),
        RpcClientConfig {
            // TODO Expose as a CLI argument.
            commitment_config: CommitmentConfig::finalized(),
            confirm_transaction_initial_timeout: None,
        },
    )
}
