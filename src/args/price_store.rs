use clap::Subcommand;

pub mod initialize;

#[derive(Subcommand, Debug)]
#[command(name = "price-store")]
pub enum Command {
    /// Configures access permissions for the Price Store program.
    Initialize(initialize::InitializeArgs),
}
