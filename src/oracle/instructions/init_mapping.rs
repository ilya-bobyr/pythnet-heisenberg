use bytemuck::{Pod, Zeroable, bytes_of};
use solana_program::{instruction::AccountMeta, instruction::Instruction, pubkey::Pubkey};

// use pyth_oracle::{self, PythAccount as _};

use super::{CommandHeader, OracleCommand, compute_permissions_account};

// `<MappingAccount as PythAccount>::INITIAL_SIZE` is 56.
// `<MappingAccount as PythAccount>::MINIMUM_SIZE` is 20536.
//
// 20536 is 56 + 640 * 32.  640 is `PC_MAP_TABLE_SIZE` in the Oracle code.
//
// But the program deployed on the main Pythnet cluster actually want the size to be 160056.  Which
// is 56 + 5000 * 32.
//
// pub const MAPPING_ACCOUNT_MIN_SIZE: u64 = pyth_oracle::MappingAccount::MINIMUM_SIZE as u64;
pub const ACCOUNT_MIN_SIZE: u64 = 56 + 5000 * 32;

// This constant is called `PC_MAP_TABLE_SIZE` in the Oracle code.
// pub const MAPPING_ACCOUNT_MAX_PRODUCTS: u64 = 640;

pub fn instruction(
    program_id: Pubkey,
    funding_account: Pubkey,
    new_mapping_account: Pubkey,
    permissions_account: Option<Pubkey>,
) -> Instruction {
    let permissions_account = compute_permissions_account(program_id, permissions_account);

    let accounts = vec![
        AccountMeta::new(funding_account, true),
        AccountMeta::new(new_mapping_account, true),
        AccountMeta::new_readonly(permissions_account, false),
    ];

    Instruction {
        program_id,
        accounts,
        data: bytes_of(&InitMappingArgs::new()).to_owned(),
    }
}

#[repr(C)]
#[derive(Zeroable, Pod, Copy, Clone)]
pub struct InitMappingArgs {
    pub header: CommandHeader,
}

impl InitMappingArgs {
    pub fn new() -> Self {
        Self {
            header: CommandHeader::new(OracleCommand::InitMapping),
        }
    }
}
