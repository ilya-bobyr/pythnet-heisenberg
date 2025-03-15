//! `pyth-oracle` does not export data structures that describe the instruction accounts or
//! arguments.  So I copy/pasted code from the current version of `pyth-oracle` version 2.33.1, as
//! recorded in the `pythnet-update-oracle-v2.33.1` branch in
//!
//!   https://github.com/ilya-bobyr/pyth-client.git
//!
//! Added a few helper functions for convenience.

use bytemuck::{Pod, Zeroable, bytes_of};
use solana_program::{
    bpf_loader_upgradeable, instruction::AccountMeta, instruction::Instruction, pubkey::Pubkey,
    system_program,
};
// use pyth_oracle::{self, PythAccount as _};

pub const PC_VERSION: u32 = 2;

#[repr(i32)]
#[derive(PartialEq, Eq)]
/// This is a partial copy of the `OracleCommand`.  I've only took commands that matter for this
/// tool.
pub enum OracleCommand {
    /// Initialize first mapping list account
    // account[0] funding account       [signer writable]
    // account[1] mapping account       [signer writable]
    // account[2] permissions account   []
    #[allow(dead_code)]
    InitMapping = 0,
    /// Update authorities
    // key[0] upgrade authority         [signer writable]
    // key[1] programdata account       []
    // key[2] permissions account       [writable]
    // key[3] system program            []
    UpdPermissions = 17,
}

#[repr(C)]
#[derive(Zeroable, Pod, Copy, Clone)]
pub struct CommandHeader {
    pub version: u32,
    pub command: i32,
}

impl CommandHeader {
    pub fn new(command: OracleCommand) -> Self {
        Self {
            version: PC_VERSION,
            command: command as i32,
        }
    }
}

fn compute_permissions_account(program_id: Pubkey, permissions_account: Option<Pubkey>) -> Pubkey {
    permissions_account
        .unwrap_or_else(|| Pubkey::find_program_address(&[b"permissions"], &program_id).0)
}

// -- InitMapping --

// `<MappingAccount as PythAccount>::INITIAL_SIZE` is 56, but `<MappingAccount as
// PythAccount>::MINIMUM_SIZE is 20536.
// pub const MAPPING_ACCOUNT_MIN_SIZE: u64 = pyth_oracle::MappingAccount::MINIMUM_SIZE as u64;
pub const MAPPING_ACCOUNT_MIN_SIZE: u64 = 20536;

// This constant is called `PC_MAP_TABLE_SIZE` in the Oracle code.
// pub const MAPPING_ACCOUNT_MAX_PRODUCTS: u64 = 640;

pub fn init_mapping_instruction(
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

// -- UpdPermissions --

pub fn update_permissions_instruction(
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
            master_authority.clone(),
            data_curation_authority.clone(),
            security_authority.clone(),
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
