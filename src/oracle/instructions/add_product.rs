use std::iter;

use bytemuck::bytes_of;
use solana_program::{instruction::AccountMeta, instruction::Instruction, pubkey::Pubkey};

use super::{CommandHeader, OracleCommand, compute_permissions_account};

// `<ProductAccount as PythAccount>::INITIAL_SIZE` is 56.
// `<ProductAccount as PythAccount>::MINIMUM_SIZE` is 512.
// `MINIMUM_SIZE` is defined as `PC_PROD_ACC_SIZE`.
pub const ACCOUNT_MIN_SIZE: u64 = 512;

pub fn instruction<'metadata>(
    program_id: Pubkey,
    funding_account: Pubkey,
    mapping_account: Pubkey,
    new_product_account: Pubkey,
    permissions_account: Option<Pubkey>,
    metadata: &'metadata [(&'metadata str, &'metadata str)],
) -> Instruction {
    let permissions_account = compute_permissions_account(program_id, permissions_account);

    let accounts = vec![
        AccountMeta::new(funding_account, true),
        AccountMeta::new(mapping_account, true),
        AccountMeta::new(new_product_account, true),
        AccountMeta::new_readonly(permissions_account, false),
    ];

    Instruction {
        program_id,
        accounts,
        data: AddProductArgs::new(metadata).as_instruction_data(),
    }
}

#[derive(Clone)]
pub struct AddProductArgs<'source> {
    pub header: CommandHeader,
    pub metadata: &'source [(&'source str, &'source str)],
}

impl<'source> AddProductArgs<'source> {
    pub fn new(metadata: &'source [(&'source str, &'source str)]) -> Self {
        Self {
            header: CommandHeader::new(OracleCommand::AddProduct),
            metadata,
        }
    }

    pub fn as_instruction_data(&self) -> Vec<u8> {
        let header_size = bytes_of(&self.header).len();
        let size = header_size
            + self
                .metadata
                .iter()
                .map(|(key, value)| 1 + key.len() + 1 + value.len())
                .sum::<usize>();
        let mut res = Vec::with_capacity(size);

        res.extend(bytes_of(&self.header));

        fn append_string(into: &mut Vec<u8>, s: &str) {
            into.extend(iter::once(u8::try_from(s.len()).expect(
                "All metadata keys and values should be shorter than 256 bytes long",
            )));
            into.extend(s.as_bytes());
        }

        for (key, value) in self.metadata {
            append_string(&mut res, key);
            append_string(&mut res, value);
        }

        assert_eq!(res.len(), size);

        res
    }
}
