use anyhow::Result;
use clap::Parser as _;

mod args;
pub mod blockhash_cache;
pub(crate) mod keypair_ext;
pub mod node_address_service;
mod oracle;
mod primordial_accounts;
pub(crate) mod rpc_client_ext;
mod stake_caps_parameters;

#[tokio::main]
async fn main() -> Result<()> {
    let args::Args { command } = args::Args::parse();

    match command {
        args::Command::StakeCapsParameters(command) => stake_caps_parameters::run(command).await,
        args::Command::PrimordialAccounts(command) => primordial_accounts::run(command).await,
        args::Command::Oracle(command) => oracle::run(command).await,
    }
}
