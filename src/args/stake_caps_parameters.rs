use clap::Subcommand;

pub mod set_parameters;

#[derive(Subcommand, Debug)]
#[command(name = "show")]
pub enum Command {
    /// Initialize or update the cluster stake cap parameters account.
    SetParameters(set_parameters::SetParametersArgs),
}
