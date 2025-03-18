use anyhow::Result;

use crate::args::oracle::Command;

mod add_product;
mod init_mapping;
pub mod instructions;
mod update_permissions;

pub async fn run(command: Command) -> Result<()> {
    match command {
        Command::UpdatePermissions(args) => update_permissions::run(args).await,
        Command::InitMapping(args) => init_mapping::run(args).await,
        Command::AddProduct(args) => {
            args.check_are_valid()?;
            add_product::run(args).await
        }
    }
}
