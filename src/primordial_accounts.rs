use anyhow::Result;

use crate::args::primordial_accounts::Command;

mod loader_v3;

pub async fn run(command: Command) -> Result<()> {
    match command {
        Command::LoaderV3(args) => loader_v3::run(args).await,
    }
}
