use std::path::PathBuf;

use clap::Args;
use solana_program::pubkey::Pubkey;

use crate::args::JsonRpcUrlArgs;

#[derive(Args, Debug)]
pub struct InitMappingArgs {
    #[command(flatten)]
    pub json_rpc_url: JsonRpcUrlArgs,

    /// Address of the Oracle program.
    #[arg(long)]
    pub program_id: Pubkey,

    /// An address of the permissions account for this Oracle.
    ///
    /// It can be computed like this, and defaults to this value if not specified:
    ///
    ///   solana find-program-derived-address
    ///     "[Oracle program pubkey]" string:permissions
    #[arg(long)]
    pub permissions_account: Option<Pubkey>,

    /// A keypair file for the account that would pay for the mapping account.
    ///
    /// It also needs to be the `master_authority` from the permissions account, as it is the only
    /// account that can create new mappings.
    #[arg(long)]
    pub funding_keypair: PathBuf,

    /// A keypair file for an account that will hold the new mapping.
    ///
    /// If the path does not point to an existing file, a keypair will be generated and written to
    /// this file.
    ///
    /// The account is not expected to exist before the call.
    ///
    /// The tool will create an account at this address, with an appropriate size, funded by the
    /// `--funding_keypair`, and then transfer the ownership to the Oracle program.
    #[arg(long)]
    pub mapping_keypair: PathBuf,
}
