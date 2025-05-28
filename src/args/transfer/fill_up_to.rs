use std::path::PathBuf;

use clap::Args;
use solana_program::pubkey::Pubkey;

use crate::args::{JsonRpcUrlArgs, u64_nice_parser};

#[derive(Args, Debug)]
pub struct FillUpToArgs {
    #[command(flatten)]
    pub json_rpc_url: JsonRpcUrlArgs,

    /// A keypair file for the signer of the transfer transactions.
    #[arg(long)]
    pub signer_keypair: PathBuf,

    /// A keypair file for the account that would pay for the transaction.
    ///
    /// Defaults to the `--signer-keypair`.
    #[arg(long)]
    pub payer_keypair: Option<PathBuf>,

    /// An account to transfer SOL from.
    ///
    /// Defaults to the `--payer-keypair`.
    #[arg(long)]
    pub from_keypair: Option<PathBuf>,

    /// A balance that we want to see on all the specified target accounts, in lamports.
    #[arg(long, value_parser = u64_nice_parser)]
    pub target_balance: u64,

    /// Print expected balance increments for all the accounts that are going to receive balance
    /// transfers.
    #[arg(long)]
    pub print_target_increments: bool,

    /// Target accounts, that after successful execution should all have a balance equal to
    /// `--target-balance`.
    ///
    /// These accounts do not need to exist.
    pub recepients: Vec<Pubkey>,
}
