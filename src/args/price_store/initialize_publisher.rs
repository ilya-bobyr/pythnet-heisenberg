use std::path::PathBuf;

use clap::{Args, value_parser};
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
    #[arg(long)]
    pub publisher_pubkey: Pubkey,

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
    #[arg(long)]
    pub price_buffer_keypair: PathBuf,

    /// Allocate space for this many prices in the buffer account.
    ///
    /// Maximum is 524,285 prices per buffer account.  But it would cost about 73 SOL to allocate a
    /// buffer of this size.
    ///
    /// Note that you can not change it after the buffer is created.
    #[arg(
        long,
        default_value_t = 5000,
        value_parser = value_parser!(u64).range(0..=524285)
    )]
    pub max_prices: u64,
}
