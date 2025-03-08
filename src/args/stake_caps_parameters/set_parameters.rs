use std::path::PathBuf;

use clap::Args;
use solana_program::pubkey::Pubkey;

use crate::args::{JsonRpcUrlArgs, u64_nice_parser, u64_nice_printer};

#[derive(Args, Debug)]
pub struct SetParametersArgs {
    #[command(flatten)]
    pub json_rpc_url: JsonRpcUrlArgs,

    /// A keypair file for the signer of the update transaction.
    #[arg(long)]
    pub signer_keypair: PathBuf,

    /// An address of the stake_caps_parameters program.
    #[arg(long)]
    pub program_id: Pubkey,

    /// An address of the parameters account from the stake_caps_parameters program.
    ///
    /// It can be computed like this, and defaults to this value if not specified:
    ///
    ///   solana find-program-derived-address
    ///     "[stake_caps_parameters program pubkey]" string:parameters
    #[arg(long)]
    pub parameters_account: Option<Pubkey>,

    /// Value of the `m` parameter.
    #[arg(
        long,
        default_value = u64_nice_printer(1_800_000_000_000),
        value_parser = u64_nice_parser
    )]
    pub m: u64,

    /// Value of the `z` parameter.
    #[arg(long, default_value = "10", value_parser = crate::args::u64_nice_parser)]
    pub z: u64,

    /// An authority that would be able to make changes to the parameters in the future.
    ///
    /// Defaults to the `--signer-keypair`, if not specified.
    #[arg(long)]
    pub update_authority: Option<Pubkey>,
}
