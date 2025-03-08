//! Helpers for dealing with `Keypair`s.

use std::path::Path;

use anyhow::{Context as _, Result, anyhow};
use solana_sdk::{signature::Keypair, signer::keypair::read_keypair_file as sdk_read_keypair_file};

pub fn read_keypair_file(path: impl AsRef<Path>) -> Result<Keypair> {
    let path = path.as_ref();
    sdk_read_keypair_file(path)
        // It is a bit strange, but `Box<dyn Error>` does not implement `Error` for some reason.
        // And `anyhow::Context::with_context` fails.  So I need to construct a new `Error`
        // instance explicitly here.
        .map_err(|err| anyhow!(err.to_string()))
        .with_context(|| format!("Error reading a keypair from: {}", path.to_string_lossy()))
}
