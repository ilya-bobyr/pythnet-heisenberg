use std::path::PathBuf;

use clap::Args;
use solana_program::pubkey::Pubkey;

use crate::args::JsonRpcUrlArgs;

#[derive(Args, Debug)]
pub struct AddPublisherArgs {
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
    /// account that can add prices to products.
    #[arg(long)]
    pub funding_keypair: PathBuf,

    /// A keypair file for a price account that will be modified.
    #[arg(long)]
    pub price_keypair: PathBuf,

    /// A address of the publisher to add.
    #[arg(long)]
    pub publisher_pubkey: Pubkey,
}
