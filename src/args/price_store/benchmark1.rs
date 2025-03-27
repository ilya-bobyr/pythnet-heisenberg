use std::{path::PathBuf, time::Duration as StdDuration};

use anyhow::{Result, bail};
use clap::{ArgAction, Args, value_parser};
use humantime::Duration;
use reqwest::Url;
use solana_program::pubkey::Pubkey;

use crate::args::JsonRpcUrlArgs;

#[derive(Args, Debug)]
pub struct Benchmark1Args {
    #[command(flatten)]
    pub json_rpc_url: JsonRpcUrlArgs,

    #[arg(long, value_name = "URL", default_value = "ws://localhost:8900")]
    /// A WebSocket address of a Pythnet node.
    pub websocket_url: Url,

    #[arg(long, default_value_t = 4)]
    /// Send each transaction to validators that cover this many slots in the future.
    ///
    /// We look at all the validators in the schedule starting with the current slot estimate and
    /// send each transaction to all validators that cover the current and this many future slots.
    pub fanout_slots: u8,

    /// Address of the Price Store program.
    #[arg(long)]
    pub program_id: Pubkey,

    /// A keypair file for an account that would pay for transactions.
    ///
    /// You want to have as may payers as there are publishers.  Each publisher has a single price
    /// buffer, which limits the execution parallelism.
    ///
    /// You want to avoid serializing transaction execution due to the publisher account overlap.
    ///
    // TODO Enforce the above restriction?
    #[arg(long, action = ArgAction::Append)]
    pub payer_keypair: Vec<PathBuf>,

    /// An address of a publisher publishing a price update.
    ///
    /// The benchmark will send price updates on behalf of all of the specified publishers in
    /// parallel.
    #[arg(long, action = ArgAction::Append)]
    pub publisher_keypair: Vec<PathBuf>,

    /// An account that holds price updates from a particular publisher.
    ///
    /// There should be exactly the same number of `--publisher-keypair` arguments as there are
    /// `--price-buffer-pubkey` arguments.
    ///
    /// Price buffers are reused every block, but they need to have enough space to store all the
    /// price updates from their publisher within any given block.
    #[arg(long, action = ArgAction::Append)]
    pub price_buffer_pubkey: Vec<Pubkey>,

    /// Send price updates for price feed indices starting at this value.
    #[arg(long, default_value_t = 1)]
    pub price_feed_index_start: u32,

    /// Send price updates for price feed indices ending at this value.
    #[arg(long)]
    pub price_feed_index_end: u32,

    /// Number of price feed updates to aggregate in the same transaction.
    ///
    /// Range: [1, 50]
    #[arg(long, default_value_t = 10, value_parser = value_parser!(u8).range(1..50))]
    pub price_updates_per_tx: u8,

    /// Delay between consecutive updates from the same publisher.
    ///
    /// The tool will try to publish updated prices for all prices for each given publisher.  And
    /// then it will wait before publishing the next price update for a given publisher, if less
    /// time has passed.
    #[arg(long, default_value_t = StdDuration::from_millis(400).into())]
    pub update_frequency: Duration,

    /// Prices will fluctuate around this point.
    ///
    /// Each publisher will have their own value of the price, for each of the price feeds, but they
    /// all will fluctuate around this point.
    #[arg(long, allow_negative_numbers = true)]
    pub price_mean: i64,

    /// Maximum value that can be added or subtracted from the `--price-mean` as a result of the
    /// price fluctuation.
    #[arg(long)]
    pub price_range: u64,

    /// Price confidence intervals will fluctuate around this point.
    ///
    /// Each publisher will have their own value of the price confidence, for each of the price
    /// feeds, but they all will fluctuate around this point.
    #[arg(long)]
    pub confidence_mean: u64,

    /// Maximum value that can be added or subtracted from the `--confidence-mean` as a result of
    /// the price confidence fluctuation.
    ///
    /// Note that price confidence can never become negative, so the fluctuation math is saturating.
    #[arg(long)]
    pub confidence_range: u64,

    /// The benchmark will run for this long.
    ///
    /// This accepts any formats that the `humantime` library can parse, for the `Duration` values:
    ///
    /// https://docs.rs/humantime/latest/humantime/
    #[arg(long)]
    pub duration: Duration,

    /// An interval for reporting transaction stats.
    ///
    /// This accepts any formats that the `humantime` library can parse, for the `Duration` values:
    ///
    /// https://docs.rs/humantime/latest/humantime/
    #[arg(long, default_value_t = StdDuration::from_secs(60).into())]
    pub stats_update_interval: Duration,
}

/// Additional validation of the [`SubmitPricesArgs`] instances.
impl Benchmark1Args {
    pub fn check_are_valid(&self) -> Result<()> {
        let Self {
            publisher_keypair,
            price_buffer_pubkey,
            price_feed_index_start,
            price_feed_index_end,
            ..
        } = self;

        if price_feed_index_start > price_feed_index_end {
            bail!("--price-feed-index-start must be at or below --price-feed-index-end");
        }

        if publisher_keypair.is_empty() {
            bail!("You need to specify at least one publisher with --publisher-keypair");
        }

        if publisher_keypair.len() != price_buffer_pubkey.len() {
            bail!(
                "You have to specify the same number of --publisher-keypair and \
                 --price-buffer-pubkey arguments.\n\
                 Got --publisher-keypair: {}\n\
                 Got --price-buffer-pubkey: {}",
                publisher_keypair.len(),
                price_buffer_pubkey.len(),
            );
        }

        Ok(())
    }
}
