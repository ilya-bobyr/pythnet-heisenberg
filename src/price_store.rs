use anyhow::Result;

use crate::args::price_store::Command;

mod initialize;
pub mod instructions;

pub async fn run(command: Command) -> Result<()> {
    match command {
        Command::Initialize(args) => initialize::run(args).await,
    }
}
