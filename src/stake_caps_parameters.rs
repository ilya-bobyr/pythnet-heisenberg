use anyhow::Result;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;

use crate::args::stake_caps_parameters::Command;

mod set_parameters;

pub async fn run(rpc_client: &RpcClient, command: Command) -> Result<()> {
    match command {
        Command::SetParameters(args) => set_parameters::run(rpc_client, args).await,
    }
}
