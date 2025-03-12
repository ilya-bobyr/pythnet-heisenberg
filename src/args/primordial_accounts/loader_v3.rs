use std::path::PathBuf;

use clap::Args;
use solana_program::pubkey::Pubkey;

#[derive(Args, Debug)]
pub struct LoaderV3Args {
    /// Address of the program main account.  Aka, program ID.
    #[arg(long)]
    pub program_id: Pubkey,

    /// A slot when the program was last updated.
    ///
    /// If you are adding the program to the genesis, you probably want to keep this at 0.
    #[arg(long, default_value_t = 0)]
    pub last_modified_slot: u64,

    /// An SO file that holds the program data.
    #[arg(long)]
    pub program_data: PathBuf,

    /// Account that can upgrade the program in the future.  Non-upgradable, if not specified.
    #[arg(long)]
    pub upgrade_authority: Option<Pubkey>,
}
