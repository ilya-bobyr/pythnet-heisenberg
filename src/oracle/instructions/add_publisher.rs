use bytemuck::{Pod, Zeroable, bytes_of};
use solana_program::{instruction::AccountMeta, instruction::Instruction, pubkey::Pubkey};

use super::{CommandHeader, OracleCommand, compute_permissions_account};

pub fn instruction(
    program_id: Pubkey,
    funding_account: Pubkey,
    price_account: Pubkey,
    permissions_account: Option<Pubkey>,
    publisher: Pubkey,
) -> Instruction {
    let permissions_account = compute_permissions_account(program_id, permissions_account);

    let accounts = vec![
        AccountMeta::new(funding_account, true),
        AccountMeta::new(price_account, true),
        AccountMeta::new(permissions_account, false),
    ];

    Instruction {
        program_id,
        accounts,
        data: bytes_of(&AddPublisherArgs::new(publisher)).to_owned(),
    }
}

#[repr(C)]
#[derive(Zeroable, Pod, Copy, Clone)]
pub struct AddPublisherArgs {
    pub header: CommandHeader,
    pub publisher: Pubkey,
}

impl AddPublisherArgs {
    pub fn new(publisher: Pubkey) -> Self {
        Self {
            header: CommandHeader::new(OracleCommand::AddPublisher),
            publisher,
        }
    }
}
