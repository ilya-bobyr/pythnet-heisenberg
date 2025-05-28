use clap::Subcommand;

pub mod fill_up_to;

#[derive(Subcommand, Debug)]
#[command(name = "transfer")]
pub enum Command {
    /// Makes sure that the specified accounts have at least a certain balance.
    FillUpTo(fill_up_to::FillUpToArgs),
}
