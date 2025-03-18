//! Helpers for dealing with `Keypair`s.

use std::path::Path;

use anyhow::{Context as _, Result, anyhow};
use rand_0_7::rngs::OsRng;
use solana_sdk::{signature::Keypair, signer::EncodableKey};

pub fn read_keypair_file(path: impl AsRef<Path>) -> Result<Keypair> {
    let path = path.as_ref();
    Keypair::read_from_file(path)
        // It is a bit strange, but `Box<dyn Error>` does not implement `Error` for some reason.
        // And `anyhow::Context::with_context` fails.  So I need to construct a new `Error`
        // instance explicitly here.
        .map_err(|err| anyhow!(err.to_string()))
        .with_context(|| format!("Error reading a keypair from: {}", path.to_string_lossy()))
}

#[allow(unused)]
pub fn read_or_generate_keypair_file(path: impl AsRef<Path>) -> Result<Keypair> {
    let path = path.as_ref();

    if path.exists() {
        return read_keypair_file(path);
    }

    let key = Keypair::generate(&mut OsRng);
    key.write_to_file(path)
        .map_err(|err| anyhow!(err.to_string()))
        .with_context(|| format!("Error reading a keypair from: {}", path.to_string_lossy()))?;

    Ok(key)
}
