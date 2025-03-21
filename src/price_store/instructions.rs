//! `price-store` export data structures that describe the instruction accounts and arguments.  But
//! it also has exact dependencies on Solana SDK 1.14 (as in `=1.14`), which is almost guaranteed to
//! cause problems in the dependency graph.
//!
//! So I copy/pasted code from the current version of `pyth-price-store` taken from
//! `pyth-crosschain` as of this commit:
//!
//! ```gitcommit
//! commit e399a0325f81ee55f678df605d4b2dd6e7fbb01f
//! Author: Pavel Strakhov <ri@idzaaus.org>
//! Date:   Tue Dec 10 15:40:02 2024 +0000
//!
//! feat(lazer): add solana contract migration script, add message parsing to protocol (#2181)
//! ```
//!
//! Added a few helper functions for convenience.

use solana_program::pubkey::Pubkey;

pub mod initialize;

pub const CONFIG_SEED: &str = "CONFIG";

#[repr(u8)]
#[derive(PartialEq, Eq)]
/// This is a copy of the `Instruction` enum.
pub enum InstructionId {
    // key[0] payer     [signer writable]
    // key[1] config    [writable]
    // key[2] system    []
    Initialize = 0,
}

/// Address of the Price Store config account.
fn compute_config_account(program_id: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[CONFIG_SEED.as_bytes()], &program_id)
}
