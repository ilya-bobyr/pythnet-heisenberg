use bytemuck::{Pod, Zeroable, bytes_of};
use solana_program::{
    bpf_loader_upgradeable, instruction::AccountMeta, instruction::Instruction, pubkey::Pubkey,
    system_program,
};

use super::{CommandHeader, OracleCommand, compute_permissions_account};

pub fn instruction(
    program_id: Pubkey,
    upgade_authority: Pubkey,
    permissions_account: Option<Pubkey>,
    master_authority: Pubkey,
    data_curation_authority: Pubkey,
    security_authority: Pubkey,
) -> Instruction {
    let (program_data_account, _) =
        Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::id());

    let permissions_account = compute_permissions_account(program_id, permissions_account);

    let accounts = vec![
        AccountMeta::new(upgade_authority, true),
        AccountMeta::new_readonly(program_data_account, false),
        AccountMeta::new(permissions_account, false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Instruction {
        program_id,
        accounts,
        data: bytes_of(&UpdPermissionsArgs::new(
            master_authority,
            data_curation_authority,
            security_authority,
        ))
        .to_owned(),
    }
}

#[repr(C)]
#[derive(Zeroable, Pod, Copy, Clone)]
pub struct UpdPermissionsArgs {
    pub header: CommandHeader,
    pub master_authority: Pubkey,
    pub data_curation_authority: Pubkey,
    pub security_authority: Pubkey,
}

impl UpdPermissionsArgs {
    pub fn new(
        master_authority: Pubkey,
        data_curation_authority: Pubkey,
        security_authority: Pubkey,
    ) -> Self {
        Self {
            header: CommandHeader::new(OracleCommand::UpdPermissions),
            master_authority,
            data_curation_authority,
            security_authority,
        }
    }
}
