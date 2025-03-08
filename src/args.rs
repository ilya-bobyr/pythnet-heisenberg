use clap::{Parser, Subcommand};
use num_format::{Locale, ToFormattedString, parsing::ParseFormatted};

pub mod json_rpc_url_args;
pub mod stake_caps_parameters;

pub use json_rpc_url_args::JsonRpcUrlArgs;

/// Suite of tools for testing a Pythnet cluster.
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

/// A specific action to perform.
#[derive(Subcommand, Debug)]
pub enum Command {
    #[command(subcommand)]
    /// Interact with the stake caps parameters program.
    StakeCapsParameters(stake_caps_parameters::Command),
}

fn u64_nice_parser(value: &str) -> Result<u64, String> {
    // `SystemLocale` fails to parse a `u64` if instantiated on a system with "C.UTF-8" environment
    // locale.  Not sure why.
    // let locale = SystemLocale::new().unwrap();
    let locale = Locale::en;
    value
        .parse_formatted(&locale)
        .map_err(|err| err.to_string())
}

fn u64_nice_printer(value: u64) -> String {
    // See `u64_nice_printer` for the reason local here is hardcoded.
    let locale = Locale::en;
    value.to_formatted_string(&locale)
}
