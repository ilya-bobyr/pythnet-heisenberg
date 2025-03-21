use bytemuck::{Pod, Zeroable, bytes_of};
use solana_program::{
    instruction::AccountMeta, instruction::Instruction, pubkey::Pubkey, system_program,
};

use super::{InstructionId, compute_config_account, compute_publisher_config_account};

pub fn instruction(
    program_id: Pubkey,
    authority: Pubkey,
    publisher: Pubkey,
    buffer: Pubkey,
) -> Instruction {
    let (config_account, config_account_bump) = compute_config_account(program_id);

    let (publisher_config, publisher_config_bump) =
        compute_publisher_config_account(program_id, publisher);

    let accounts = vec![
        AccountMeta::new(authority, true),
        AccountMeta::new_readonly(config_account, false),
        AccountMeta::new(publisher_config, false),
        AccountMeta::new(buffer, false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Instruction {
        program_id,
        accounts,
        data: bytes_of(&InitializePublisherArgs::new(
            config_account_bump,
            publisher_config_bump,
            publisher,
        ))
        .to_owned(),
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct InitializePublisherArgs {
    /// Set to [`InstructionId::InitializePublisher`].
    pub id: u8,

    /// PDA bump of the config account.
    pub config_bump: u8,

    /// PDA bump of the publisher config account.
    pub publisher_config_bump: u8,

    /// The publisher to be initialized.
    pub publisher: [u8; 32],
}

impl InitializePublisherArgs {
    pub fn new(config_bump: u8, publisher_config_bump: u8, publisher: Pubkey) -> Self {
        Self {
            id: InstructionId::InitializePublisher as u8,
            config_bump,
            publisher_config_bump,
            publisher: publisher.to_bytes(),
        }
    }
}
