use anyhow::Result;

use crate::args::price_store::Command;

mod benchmark1;
mod initialize;
mod initialize_publisher;
pub mod instructions;
mod submit_prices;

pub async fn run(command: Command) -> Result<()> {
    match command {
        Command::Initialize(args) => initialize::run(args).await,
        Command::InitializePublisher(args) => {
            args.check_are_valid()?;
            initialize_publisher::run(args).await
        }
        Command::SubmitPrices(args) => submit_prices::run(args).await,
        Command::Benchmark1(args) => {
            args.check_are_valid()?;
            benchmark1::run(args).await
        }
    }
}
