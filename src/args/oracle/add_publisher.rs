use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{ArgAction, Args};
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

    /// A keypair file for the account that would pay for the publisher account.
    ///
    /// It also needs to be the `master_authority` from the permissions account, as it is the only
    /// account that can add prices to products.
    #[arg(long)]
    pub funding_keypair: PathBuf,

    /// A keypair file for a price account that will be modified.
    ///
    /// You can add multiple publishers to multiple prices in parallel, if you repeat this, and
    /// `--publisher-pubkey` arguments.
    ///
    /// You need to repeat all these arguments the same number of times, as they form tuples.
    #[arg(long, action = ArgAction::Append)]
    pub price_keypair: Vec<PathBuf>,

    /// A address of the publisher to add.
    ///
    /// You can add multiple publishers to multiple prices in parallel, if you repeat this, and
    /// `--price-keypair` arguments.
    ///
    /// You need to repeat all these arguments the same number of times, as they form tuples.
    #[arg(long, action = ArgAction::Append)]
    pub publisher_pubkey: Vec<Pubkey>,
}

/// Additional validation of the [`AddPriceArgs`] instances.
impl AddPublisherArgs {
    pub fn check_are_valid(&self) -> Result<()> {
        let Self {
            price_keypair: price_keypairs,
            publisher_pubkey: publisher_pubkeys,
            ..
        } = self;

        if price_keypairs.len() != publisher_pubkeys.len() {
            bail!(
                "--price-keypair and --publisher-pubkey arguments should be repeated the same \
                 number of times.\n\
                 Provided --price-keypair arguments: {}\n\
                 Provided --publisher-pubkey arguments: {}",
                price_keypairs.len(),
                publisher_pubkeys.len(),
            );
        }

        Ok(())
    }
}
