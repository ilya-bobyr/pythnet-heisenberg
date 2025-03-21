//! Describes accounts of the Oracle program.

use bytemuck::{Pod, Zeroable};

pub mod price;

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct AccountHeader {
    pub magic_number: u32,
    pub version: u32,
    pub account_type: u32,
    pub size: u32,
}
