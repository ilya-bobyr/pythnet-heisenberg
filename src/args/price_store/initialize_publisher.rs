use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{ArgAction, Args, value_parser};
use solana_program::pubkey::Pubkey;

use crate::args::JsonRpcUrlArgs;

#[derive(Args, Debug)]
pub struct InitializePublisherArgs {
    #[command(flatten)]
    pub json_rpc_url: JsonRpcUrlArgs,

    /// Address of the Price Store program.
    #[arg(long)]
    pub program_id: Pubkey,

    /// A keypair file for the account that would pay for the publisher config account.
    ///
    /// Price Store program uses a config account, which is a PDA.  It needs to be constructed as
    /// the program initialization step.
    #[arg(long)]
    pub payer_keypair: PathBuf,

    /// An account that can add new publishers.
    #[arg(long)]
    pub authority_keypair: PathBuf,

    /// An address of the publisher to add.
    ///
    /// You can add multiple publishers in parallel, if you repeat this, `--price-buffer-keypair`,
    /// and `--max-prices` arguments.
    ///
    /// You need to repeat all these arguments the same number of times, as they form tuples.
    #[arg(long, action = ArgAction::Append)]
    pub publisher_pubkey: Vec<Pubkey>,

    /// An account that will hold price updates from this publisher.
    ///
    /// It is reused every block, but it needs to have enough space to store all the price updates
    /// from this publisher within any given block.
    ///
    /// If the path does not point to an existing file, a keypair will be generated and written to
    /// this file.
    ///
    /// The account does not need to exist before the call.
    ///
    /// The tool will create an account at this address, capable of holding as many prices as the
    /// `--max-prices` argument specifies.
    ///
    /// You can add multiple publishers in parallel, if you repeat this, `--publisher-pubkey`, and
    /// `--max-prices` arguments.
    ///
    /// You need to repeat all these arguments the same number of times, as they form tuples.
    #[arg(long, action = ArgAction::Append)]
    pub price_buffer_keypair: Vec<PathBuf>,

    /// Allocate space for this many prices in the buffer account.
    ///
    /// Maximum is 524,285 prices per buffer account.  But it would cost about 73 SOL to allocate a
    /// buffer of this size.
    ///
    /// Note that you can not change it after the buffer is created.
    ///
    /// You can add multiple publishers in parallel, if you repeat this, `--publisher-keypair`, and
    /// `--price-buffer-keypair` arguments.
    ///
    /// You need to repeat all these arguments the same number of times, as they form tuples.
    ///
    /// You can also omit this argument completely, to use a default of 5,000 for all price buffer
    /// accounts.
    #[arg(
        long,
        value_parser = value_parser!(u64).range(1..=524285),
        action = ArgAction::Append,
    )]
    pub max_prices: Vec<u64>,
}

/// Additional validation of the [`AddPriceArgs`] instances.
impl InitializePublisherArgs {
    pub fn check_are_valid(&self) -> Result<()> {
        let Self {
            publisher_pubkey: publisher_pubkeys,
            price_buffer_keypair: price_buffer_keypairs,
            max_prices,
            ..
        } = self;

        if publisher_pubkeys.len() != price_buffer_keypairs.len() {
            bail!(
                "--publisher-pubkeys and --price-buffer-keypairs arguments should be repeated the \
                 same number of times.\n\
                 Provided --publisher-pubkey arguments: {}\n\
                 Provided --price-buffer-keypair arguments: {}",
                publisher_pubkeys.len(),
                price_buffer_keypairs.len(),
            );
        }

        if !max_prices.is_empty() && publisher_pubkeys.len() != max_prices.len() {
            bail!(
                "--publisher-pubkeys and --max-prices arguments should be repeated the same number \
                 of times, if --max-prices argument is specified.\n\
                 Provided --publisher-pubkey arguments: {}\n\
                 Provided --max-prices arguments: {}",
                publisher_pubkeys.len(),
                max_prices.len(),
            );
        }

        Ok(())
    }
}
