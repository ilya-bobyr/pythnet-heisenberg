use anyhow::Result;
use clap::Parser as _;
use solana_rpc_client::{
    http_sender::HttpSender, nonblocking::rpc_client::RpcClient, rpc_client::RpcClientConfig,
};

mod args;
mod stake_caps_parameters;

#[tokio::main]
async fn main() -> Result<()> {
    let args::Args { rpc_url, command } = args::Args::parse();

    let rpc_client = RpcClient::new_sender(HttpSender::new(rpc_url), RpcClientConfig::default());

    match command {
        args::Command::StakeCapsParameters(command) => {
            stake_caps_parameters::run(&rpc_client, command).await
        }
    }
}
