use std::{collections::HashMap, fs, io};

use anyhow::{Context as _, Result};
use base64::{self, Engine as _};
use bincode::{
    self,
    serde::{encode_into_slice, encode_to_vec},
};
use solana_genesis::Base64Account;
use solana_sdk::{
    bpf_loader_upgradeable::{self, UpgradeableLoaderState},
    pubkey::Pubkey,
    sysvar::rent::Rent,
};

use crate::args::primordial_accounts::loader_v3::LoaderV3Args;

pub async fn run(
    LoaderV3Args {
        program_id,
        last_modified_slot,
        program_data,
        upgrade_authority,
    }: LoaderV3Args,
) -> Result<()> {
    let rent = Rent::default();

    let program_so_data = fs::read(&program_data).with_context(|| {
        format!(
            "Failed to read the --program-data file: {}",
            program_data.to_string_lossy()
        )
    })?;

    let (program_data_address, _) =
        Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::id());

    let program_account = {
        let data = UpgradeableLoaderState::Program {
            programdata_address: program_data_address,
        };

        let data = encode_to_vec(data, bincode::config::legacy())
            .context("Encoding program data with `bincode`")?;
        assert_eq!(data.len(), UpgradeableLoaderState::size_of_program());

        Base64Account {
            balance: rent.minimum_balance(data.len()),
            data: base64::engine::general_purpose::STANDARD.encode(data),
            executable: true,
            owner: bpf_loader_upgradeable::id().to_string(),
        }
    };

    let program_data_account = {
        let data = UpgradeableLoaderState::ProgramData {
            slot: last_modified_slot,
            upgrade_authority_address: upgrade_authority,
        };

        let data = {
            let data_len = UpgradeableLoaderState::size_of_programdata(program_so_data.len());
            let mut buf = vec![0; data_len];

            let encoded_header_size = encode_into_slice(
                data,
                &mut buf[0..UpgradeableLoaderState::size_of_programdata_metadata()],
                bincode::config::legacy(),
            )
            .context("Encoding program data header with `bincode`")?;
            assert_eq!(
                encoded_header_size,
                UpgradeableLoaderState::size_of_programdata_metadata()
            );

            buf[UpgradeableLoaderState::size_of_programdata_metadata()..]
                .copy_from_slice(&program_so_data);

            buf
        };
        debug_assert!(
            data.len() == UpgradeableLoaderState::size_of_programdata(program_so_data.len())
        );

        Base64Account {
            balance: rent.minimum_balance(data.len()),
            data: base64::engine::general_purpose::STANDARD.encode(data),
            executable: false,
            owner: bpf_loader_upgradeable::id().to_string(),
        }
    };

    serde_yaml::to_writer(
        io::stdout().lock(),
        &HashMap::<String, Base64Account>::from([
            (program_id.to_string(), program_account),
            (program_data_address.to_string(), program_data_account),
        ]),
    )
    .context("Constructing final YAML")?;

    Ok(())
}
