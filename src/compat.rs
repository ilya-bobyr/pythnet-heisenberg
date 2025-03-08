//! This module deals with the issue of the Solana types that are incompatible across the major
//! crate version.
//!
//! In particular, the stake_caps_parameters program uses `solana_pubkey` version 1.18.  And while
//! 2.x `Pubkey`s are identical, as the major version is different, they are treated as unrelated
//! types.

use solana_program_v1_18::pubkey::Pubkey as Pubkey_v1_18;
use solana_pubkey::Pubkey;

trait PubkeyCompat {
    // TODO
}

pub fn to_v1_18_pubkey(pubkey: Pubkey) -> Pubkey_v1_18 {
    Pubkey_v1_18::new_from_array(pubkey.to_bytes())
}

pub fn from_v1_18_pubkey(pubkey: Pubkey_v1_18) -> Pubkey {
    Pubkey::new_from_array(pubkey.to_bytes())
}
