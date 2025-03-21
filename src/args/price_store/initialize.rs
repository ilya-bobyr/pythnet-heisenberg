use std::path::PathBuf;

use clap::Args;
use solana_program::pubkey::Pubkey;

use crate::args::JsonRpcUrlArgs;

#[derive(Args, Debug)]
pub struct InitializeArgs {
    #[command(flatten)]
    pub json_rpc_url: JsonRpcUrlArgs,

    /// Address of the Price Store program.
    #[arg(long)]
    pub program_id: Pubkey,

    /// A keypair file for the account that would pay for the config account.
    ///
    /// Price Store program uses a config account, which is a PDA.  It needs to be constructed as
    /// the program initialization step.
    #[arg(long)]
    pub payer_keypair: PathBuf,

    /// An account that would be able to add new publishers to this price store.
    #[arg(long)]
    pub authority: Pubkey,
}
