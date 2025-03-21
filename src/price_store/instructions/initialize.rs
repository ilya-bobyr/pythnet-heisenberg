use bytemuck::{Pod, Zeroable, bytes_of};
use solana_program::{
    instruction::AccountMeta, instruction::Instruction, pubkey::Pubkey, system_program,
};

use super::{InstructionId, compute_config_account};

pub fn instruction(program_id: Pubkey, payer: Pubkey, authority: Pubkey) -> Instruction {
    let (config_account, config_account_bump) = compute_config_account(program_id);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new(config_account, false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Instruction {
        program_id,
        accounts,
        data: bytes_of(&InitializeArgs::new(config_account_bump, authority)).to_owned(),
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct InitializeArgs {
    /// Set to [`InstructionId::Initialize`].
    pub id: u8,

    /// PDA bump of the config account.
    pub config_bump: u8,

    /// The signature of the authority account will be required to execute
    /// `InitializePublisher` instruction.
    pub authority: [u8; 32],
}

impl InitializeArgs {
    pub fn new(config_bump: u8, authority: Pubkey) -> Self {
        Self {
            id: InstructionId::Initialize as u8,
            config_bump,
            authority: authority.to_bytes(),
        }
    }
}
