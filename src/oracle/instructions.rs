//! `pyth-oracle` does not export data structures that describe the instruction accounts or
//! arguments.  So I copy/pasted code from the current version of `pyth-oracle` version 2.33.1, as
//! recorded in the `pythnet-update-oracle-v2.33.1` branch in
//!
//!   https://github.com/ilya-bobyr/pyth-client.git
//!
//! Added a few helper functions for convenience.

use bytemuck::{Pod, Zeroable};
use solana_program::pubkey::Pubkey;

pub mod add_product;
pub mod init_mapping;
pub mod update_permissions;

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
    /// Initialize and add new product reference data account
    // account[0] funding account       [signer writable]
    // account[1] mapping account       [signer writable]
    // account[2] new product account   [signer writable]
    // account[3] permissions account   []
    AddProduct = 2,
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
