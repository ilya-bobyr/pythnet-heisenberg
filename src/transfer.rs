use anyhow::Result;

use crate::args::transfer::Command;

mod fill_up_to;

pub async fn run(command: Command) -> Result<()> {
    match command {
        Command::FillUpTo(args) => fill_up_to::run(args).await,
    }
}
