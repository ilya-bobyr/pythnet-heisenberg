//! Describes a `price` account of the Oracle program.
//!
//! Most of the code here is copied from the `program/rust/src/accounts/price.rs` file from the
//! `https://github.com/ilya-bobyr/pyth-client.git` repository, branch
//! `pythnet-update-oracle-v2.33.2`.

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use solana_program::pubkey::Pubkey;

use super::AccountHeader;

pub const PC_NUM_COMP_PYTHNET: u32 = 128;
// pub const PC_NUM_COMP: u32 = 64;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PriceAccount {
    pub header: AccountHeader,
    /// Type of the price account
    pub price_type: u32,
    /// Exponent for the published prices
    pub exponent: i32,
    /// Current number of authorized publishers
    pub num: u32,
    /// Number of valid quotes for the last aggregation
    pub num_qt: u32,
    /// Last slot with a succesful aggregation (status : TRADING)
    pub last_slot: u64,
    /// Second to last slot where aggregation was attempted
    pub valid_slot: u64,
    /// Ema for price
    pub twap: PriceEma,
    /// Ema for confidence
    pub twac: PriceEma,
    /// Last time aggregation was attempted
    pub timestamp: i64,
    /// Minimum valid publisher quotes for a succesful aggregation
    pub min_pub: u8,
    pub message_sent: u8,
    /// Configurable max latency in slots between send and receive
    pub max_latency: u8,
    /// Various flags
    pub flags: PriceAccountFlags,
    /// Globally unique price feed index used for publishing.
    /// Limited to 28 bites so that it can be packed together with trading status in a single u32.
    pub feed_index: u32,
    /// Corresponding product account
    pub product_account: Pubkey,
    /// Next price account in the list
    pub next_price_account: Pubkey,
    /// Second to last slot where aggregation was successful (i.e. status : TRADING)
    pub prev_slot: u64,
    /// Aggregate price at prev_slot_
    pub prev_price: i64,
    /// Confidence interval at prev_slot_
    pub prev_conf: u64,
    /// Timestamp of prev_slot_
    pub prev_timestamp: i64,
    /// Last attempted aggregate results
    pub agg: PriceInfo,
    /// Publishers' price components. NOTE(2023-10-06): On Pythnet, not all
    /// PC_NUM_COMP_PYTHNET slots are used due to stack size
    /// issues in the C code. For iterating over price components,
    /// PC_NUM_COMP must be used.
    pub comp: [PriceComponent; PC_NUM_COMP_PYTHNET as usize],
    /// Cumulative sums of aggregative price and confidence used to compute arithmetic moving averages
    pub price_cumulative: PriceCumulative,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PriceEma {
    pub val: i64,
    pub numer: i64,
    pub denom: i64,
}

#[repr(C)]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PriceComponent {
    pub pub_: Pubkey,
    pub agg: PriceInfo,
    pub latest: PriceInfo,
}

#[repr(C)]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PriceInfo {
    pub price: i64,
    pub conf: u64,
    pub status: u32,
    pub corp_act_status: u32,
    pub pub_slot: u64,
}

bitflags! {
    #[repr(C)]
    #[derive(Copy, Clone, Pod, Zeroable)]
    pub struct PriceAccountFlags: u8 {
        /// If set, the program doesn't do accumulation, but validator does.
        const ACCUMULATOR_V2 = 0b1;
        /// If unset, the program will remove old messages from its message buffer account
        /// and set this flag.
        const MESSAGE_BUFFER_CLEARED = 0b10;
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PriceCumulative {
    /// Cumulative sum of price * slot_gap
    pub price: i128,
    /// Cumulative sum of conf * slot_gap
    pub conf: u128,
    /// Cumulative number of slots where the price wasn't recently updated (within
    /// PC_MAX_SEND_LATENCY slots). This field should be used to calculate the downtime
    /// as a percent of slots between two times `T` and `t` as follows:
    /// `(T.num_down_slots - t.num_down_slots) / (T.agg_.pub_slot_ - t.agg_.pub_slot_)`
    pub num_down_slots: u64,
    /// Padding for alignment
    pub unused: u64,
}
