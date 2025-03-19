use anyhow::Result;

use crate::args::oracle::Command;

mod add_price;
mod add_product;
mod add_publisher;
mod init_mapping;
pub mod instructions;
mod update_permissions;

pub async fn run(command: Command) -> Result<()> {
    match command {
        Command::UpdatePermissions(args) => update_permissions::run(args).await,
        Command::InitMapping(args) => init_mapping::run(args).await,
        Command::AddProduct(args) => add_product::run(args).await,
        Command::AddPrice(args) => add_price::run(args).await,
        Command::AddPublisher(args) => add_publisher::run(args).await,
    }
}
