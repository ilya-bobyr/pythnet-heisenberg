use bytemuck::{Pod, Zeroable, bytes_of};
use solana_program::{instruction::AccountMeta, instruction::Instruction, pubkey::Pubkey};

// use pyth_oracle::{PriceAccount, PythAccount};

use super::{CommandHeader, OracleCommand, compute_permissions_account};

// `<PriceAccount as PythAccount>::INITIAL_SIZE` is 12576.
// `<PriceAccount as PythAccount>::MINIMUM_SIZE` is 12576.
pub const ACCOUNT_MIN_SIZE: u64 = 12576;

pub fn instruction(
    program_id: Pubkey,
    funding_account: Pubkey,
    product_account: Pubkey,
    new_price_account: Pubkey,
    permissions_account: Option<Pubkey>,
    exponent: i32,
) -> Instruction {
    let permissions_account = compute_permissions_account(program_id, permissions_account);

    let accounts = vec![
        AccountMeta::new(funding_account, true),
        AccountMeta::new(product_account, false),
        AccountMeta::new(new_price_account, false),
        AccountMeta::new(permissions_account, false),
    ];

    Instruction {
        program_id,
        accounts,
        data: bytes_of(&AddPriceArgs::new(exponent)).to_owned(),
    }
}

// `PC_PTYPE_PRICE` in the Oracle code.
pub const PC_PTYPE_PRICE: u32 = 1;

#[repr(C)]
#[derive(Zeroable, Pod, Copy, Clone)]
pub struct AddPriceArgs {
    pub header: CommandHeader,
    pub exponent: i32,
    pub price_type: u32,
}

impl AddPriceArgs {
    pub fn new(exponent: i32) -> Self {
        Self {
            header: CommandHeader::new(OracleCommand::AddPrice),
            exponent,
            price_type: PC_PTYPE_PRICE,
        }
    }
}
