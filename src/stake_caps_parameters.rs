use anyhow::Result;

use crate::args::stake_caps_parameters::Command;

mod set_parameters;

pub async fn run(command: Command) -> Result<()> {
    match command {
        Command::SetParameters(args) => set_parameters::run(args).await,
    }
}
