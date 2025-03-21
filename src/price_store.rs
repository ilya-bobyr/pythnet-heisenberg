use anyhow::Result;

use crate::args::price_store::Command;

mod initialize;
mod initialize_publisher;
pub mod instructions;

pub async fn run(command: Command) -> Result<()> {
    match command {
        Command::Initialize(args) => initialize::run(args).await,
        Command::InitializePublisher(args) => {
            args.check_are_valid()?;
            initialize_publisher::run(args).await
        }
    }
}
