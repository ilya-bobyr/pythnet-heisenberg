use std::path::PathBuf;

use clap::{ArgAction, Args};
use solana_program::pubkey::Pubkey;

use crate::args::JsonRpcUrlArgs;

#[derive(Args, Debug)]
pub struct AddProductArgs {
    #[command(flatten)]
    pub json_rpc_url: JsonRpcUrlArgs,

    /// Address of the Oracle program.
    #[arg(long)]
    pub program_id: Pubkey,

    /// An address of the permissions account for this Oracle.
    ///
    /// It can be computed like this, and defaults to this value if not specified:
    ///
    ///   solana find-program-derived-address
    ///     "[Oracle program pubkey]" string:permissions
    #[arg(long)]
    pub permissions_account: Option<Pubkey>,

    /// A keypair file for the account that would pay for the mapping account.
    ///
    /// It also needs to be the `master_authority` from the permissions account, as it is the only
    /// account that can create new products.
    #[arg(long)]
    pub funding_keypair: PathBuf,

    /// A keypair file for a mapping account that will point to this product.
    ///
    /// Create this account, and optionally the key, with an `init_mapping` call.
    #[arg(long)]
    pub mapping_keypair: PathBuf,

    /// A keypair file for an account that will hold the new product.
    ///
    /// If the path does not point to an existing file, a keypair will be generated and written to
    /// this file.
    ///
    /// The account is not expected to exist before the call.
    ///
    /// The tool will create an account at this address, with an appropriate size, funded by the
    /// `--funding_keypair`, and then transfer the ownership to the Oracle program.
    #[arg(long)]
    pub product_keypair: PathBuf,

    /// Product metadata as "key=value" format.
    ///
    /// Each product may have arbitrary set of key/value pairs defined for it.
    ///
    /// Keys and values when UTF-8 encoded may not exceed 256 bytes.
    ///
    /// Each key and each value is stored by prefixing a single length byte to the UTF-8 encoded
    /// string bytes.  Keys and values are recorded after the product account header.
    ///
    /// Metadata (that is, keys and values, including the length bytes) can not exceed 424 bytes.
    #[arg(long, value_parser = metadata_key_value_pair, action = ArgAction::Append)]
    pub metadata: Vec<(String, String)>,
}

fn metadata_key_value_pair(input: &str) -> Result<(String, String), String> {
    let (key, value) = input
        .split_once('=')
        .ok_or("`metadata` value must contain an '=' to separate the key from the value")?;

    if key.len() > u8::MAX.into() {
        return Err(format!(
            "`key` is limited to {} bytes.  Got: {}",
            u8::MAX,
            key.len()
        ));
    }

    if value.len() > u8::MAX.into() {
        return Err(format!(
            "`value` is limited to {} bytes.  Got: {}",
            u8::MAX,
            value.len()
        ));
    }

    Ok((key.to_owned(), value.to_owned()))
}
