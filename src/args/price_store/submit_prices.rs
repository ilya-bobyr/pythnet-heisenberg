use std::{convert::TryFrom, path::PathBuf, str::FromStr as _};

use clap::{ArgAction, Args};
use solana_program::pubkey::Pubkey;

use crate::{
    args::JsonRpcUrlArgs,
    price_store::instructions::submit_prices::{BufferedPrice, FEED_INDEX_MAX, TradingStatus},
};

#[derive(Args, Debug)]
pub struct SubmitPricesArgs {
    #[command(flatten)]
    pub json_rpc_url: JsonRpcUrlArgs,

    /// Address of the Price Store program.
    #[arg(long)]
    pub program_id: Pubkey,

    /// A keypair file for the account that would pay for the transaction.
    #[arg(long)]
    pub payer_keypair: PathBuf,

    /// An address of the publisher publishing this price update.
    #[arg(long)]
    pub publisher_keypair: PathBuf,

    /// An account that holds price updates from this publisher.
    ///
    /// It is reused every block, but it needs to have enough space to store all the price updates
    /// from this publisher within any given block.
    #[arg(long)]
    pub price_buffer_pubkey: Pubkey,

    /// New price data.  Can be repeated.
    ///
    /// Need to be specified as a 4 part tuple, items separated with ':'.  Items are:
    ///
    ///   1. Trading status: unknown, trading, halted, auction, or ignored.  You can also use and
    ///      index between 0 and 4, respectively.
    ///
    ///   2. Feed index.  Identifies a price account in the Oracle price configuration.
    ///
    ///   3. Price as a signed integer.  To be scaled by the configured price exponent.
    ///
    ///   4. Confidence as an unsigned integer.  To be scaled by the configured price exponent.
    ///
    /// For example, "trading:1:10000000000:100000000" is a price update for feed index 1, that
    /// specifies a price of 100 units with a confidence interval of +/- 1 unit, assuming the
    /// exponent for the price with feed index 1 is set to -8.
    ///
    /// This price update is added to the publisher buffer.
    ///
    /// You can add up to about 50 prices in one transaction.
    #[arg(long, value_parser = price_update_parser, action = ArgAction::Append)]
    pub price: Vec<BufferedPrice>,
}

fn price_update_parser(input: &str) -> Result<BufferedPrice, String> {
    let parts: Vec<&str> = input.split(':').collect();
    if parts.len() != 4 {
        return Err("`--price` value should have exactly 4 parts separated by colons.".to_owned());
    }

    let trading_status = TradingStatus::from_str(parts[0]).or_else(|_| {
        let v = parts[0].parse::<u8>().map_err(|_| {
            format!(
                "{}: trading status: not a trading status text or a u8: {}",
                input, parts[0]
            )
        })?;
        TradingStatus::try_from(v).map_err(|_| {
            format!(
                "{}: trading status: trading status index must be in [0, 4] range: {}",
                input, parts[0]
            )
        })
    })?;

    let feed_index = {
        let v = parts[1]
            .parse::<u32>()
            .map_err(|err| format!("{}: feed index part: not a u32: {}", input, err))?;

        if v > FEED_INDEX_MAX {
            return Err(format!(
                "{}: feed index part: value exceeds the max of {}: {}",
                input, FEED_INDEX_MAX, v
            ));
        }

        v
    };

    let price = parts[2]
        .parse::<i64>()
        .map_err(|err| format!("{}: price part: not an i64: {}", input, err))?;
    let confidence = parts[3]
        .parse::<u64>()
        .map_err(|err| format!("{}: confidence part: not a u64: {}", input, err))?;

    Ok(BufferedPrice::new(
        trading_status,
        feed_index,
        price,
        confidence,
    ))
}
