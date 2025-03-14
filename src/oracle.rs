use anyhow::Result;

use crate::args::oracle::Command;

pub mod instructions;
mod update_permissions;

pub async fn run(command: Command) -> Result<()> {
    match command {
        Command::UpdatePermissions(args) => update_permissions::run(args).await,
    }
}
