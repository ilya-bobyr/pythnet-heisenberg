use std::mem::size_of_val;

use bytemuck::{Pod, Zeroable, bytes_of};
use enum_utils::{FromStr, TryFromRepr};
use solana_program::{instruction::AccountMeta, instruction::Instruction, pubkey::Pubkey};

use super::{InstructionId, compute_publisher_config_account};

pub fn instruction(
    program_id: Pubkey,
    publisher: Pubkey,
    price_buffer: Pubkey,
    prices: &[BufferedPrice],
) -> Instruction {
    let (publisher_config, publisher_config_bump) =
        compute_publisher_config_account(program_id, publisher);

    let accounts = vec![
        AccountMeta::new(publisher, true),
        AccountMeta::new_readonly(publisher_config, false),
        AccountMeta::new(price_buffer, false),
    ];

    let data_size = size_of::<SubmitPricesArgsHeader>() + size_of_val(prices);
    let mut data = Vec::with_capacity(data_size);
    data.extend(bytes_of(&SubmitPricesArgsHeader::new(
        publisher_config_bump,
    )));
    for price in prices {
        data.extend(bytes_of(price));
    }
    assert_eq!(data.len(), data_size);

    Instruction {
        program_id,
        accounts,
        data,
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct SubmitPricesArgsHeader {
    /// Set to [`InstructionId::SubmitPrices`].
    pub id: u8,

    /// PDA bump of the publisher config account.
    pub publisher_config_bump: u8,
}

impl SubmitPricesArgsHeader {
    pub fn new(publisher_config_bump: u8) -> Self {
        Self {
            id: InstructionId::SubmitPrices as u8,
            publisher_config_bump,
        }
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, FromStr, TryFromRepr)]
#[enumeration(case_insensitive)]
pub enum TradingStatus {
    // pub const PC_STATUS_UNKNOWN: u32 = 0;
    Unknown = 0,
    // pub const PC_STATUS_TRADING: u32 = 1;
    Trading = 1,
    // pub const PC_STATUS_HALTED: u32 = 2;
    Halted = 2,
    // pub const PC_STATUS_AUCTION: u32 = 3;
    Auction = 3,
    // pub const PC_STATUS_IGNORED: u32 = 4;
    Ignored = 4,
}

pub const FEED_INDEX_MAX: u32 = (1 << 28) - 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Zeroable, Pod)]
#[repr(C, packed)]
pub struct BufferedPrice {
    // 4 high bits: trading status
    // 28 low bits: feed index
    pub trading_status_and_feed_index: u32,
    pub price: i64,
    pub confidence: u64,
}

impl BufferedPrice {
    pub fn new(
        trading_status: TradingStatus,
        feed_index: u32,
        price: i64,
        confidence: u64,
    ) -> Self {
        assert!(feed_index <= FEED_INDEX_MAX);

        Self {
            trading_status_and_feed_index: ((trading_status as u32) << 28) | feed_index,
            price,
            confidence,
        }
    }
}
