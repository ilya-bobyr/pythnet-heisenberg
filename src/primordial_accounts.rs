use anyhow::Result;

use crate::args::primordial_accounts::Command;

mod feature;
mod loader_v3;

pub async fn run(command: Command) -> Result<()> {
    match command {
        Command::Feature(args) => feature::run(args).await,
        Command::LoaderV3(args) => loader_v3::run(args).await,
    }
}
