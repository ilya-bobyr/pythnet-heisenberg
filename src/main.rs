use anyhow::Result;
use clap::Parser as _;

mod args;
pub(crate) mod keypair_ext;
pub(crate) mod rpc_client_ext;
mod stake_caps_parameters;

#[tokio::main]
async fn main() -> Result<()> {
    let args::Args { command } = args::Args::parse();

    match command {
        args::Command::StakeCapsParameters(command) => stake_caps_parameters::run(command).await,
    }
}
