use anyhow::Result;
use clap::Parser as _;

mod args;
pub mod blockhash_cache;
pub(crate) mod keypair_ext;
pub mod node_address_service;
mod oracle;
mod price_store;
mod primordial_accounts;
pub(crate) mod rpc_client_ext;
mod stake_caps_parameters;
mod transfer;
mod tx_sheppard;

#[tokio::main]
async fn main() -> Result<()> {
    let args::Args { command } = args::Args::parse();

    match command {
        args::Command::PrimordialAccounts(command) => primordial_accounts::run(command).await,
        args::Command::Transfer(command) => transfer::run(command).await,
        args::Command::StakeCapsParameters(command) => stake_caps_parameters::run(command).await,
        args::Command::Oracle(command) => oracle::run(command).await,
        args::Command::PriceStore(command) => price_store::run(command).await,
    }
}
