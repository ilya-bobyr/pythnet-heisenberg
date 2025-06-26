use std::{collections::HashMap, io};

use anyhow::{Context as _, Result};
use base64::{self, Engine as _};
use bincode::{self, serde::encode_to_vec};
use solana_genesis::Base64Account;
use solana_sdk::{
    feature::{self, Feature},
    sysvar::rent::Rent,
};

use crate::args::primordial_accounts::feature::FeatureArgs;

pub async fn run(
    FeatureArgs {
        address,
        not_active,
    }: FeatureArgs,
) -> Result<()> {
    let rent = Rent::default();

    let feature_account = {
        let data = Feature {
            activated_at: if not_active { None } else { Some(0) },
        };
        let target_len = Feature::size_of();
        let mut data = encode_to_vec(data, bincode::config::legacy())
            .context("Encoding program data with `bincode`")?;
        if data.len() < target_len {
            data.resize(target_len, 0);
        }
        assert_eq!(data.len(), target_len);

        Base64Account {
            balance: rent.minimum_balance(data.len()),
            data: base64::engine::general_purpose::STANDARD.encode(data),
            executable: false,
            owner: feature::id().to_string(),
        }
    };

    serde_yaml::to_writer(
        io::stdout().lock(),
        &HashMap::<String, Base64Account>::from([(address.to_string(), feature_account)]),
    )
    .context("Constructing final YAML")?;

    Ok(())
}
