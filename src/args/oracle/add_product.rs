use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{ArgAction, Args};
use once_cell::sync::OnceCell;
use regex::Regex;
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

    /// A keypair file for the account that would pay for the product accounts.
    ///
    /// It also needs to be the `master_authority` from the permissions account, as it is the only
    /// account that can create new products.
    #[arg(long)]
    pub funding_keypair: PathBuf,

    /// A keypair file for a mapping account that will point to all the added products.
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
    ///
    /// You can multiple products in parallel, if you repeat this argument.
    ///
    /// The `--metadata` arguments need to be indexed in this case.
    #[arg(long, action = ArgAction::Append)]
    pub product_keypair: Vec<PathBuf>,

    /// Product metadata in "[index:]key=value" format.
    ///
    /// Each product may have arbitrary set of key/value pairs defined for it.
    ///
    /// If the command is adding more than one product, then the metadata should use the `index:`
    /// prefix to specify which product this metadata key applies to.
    ///
    /// When the `index` is not provided, it defaults to `0`.
    ///
    /// Keys and values when UTF-8 encoded must not exceed 256 bytes.
    ///
    /// Each key and each value is stored by prefixing a single length byte to the UTF-8 encoded
    /// bytes of the string.  Keys and values are recorded after the product account header.
    ///
    /// Metadata (that is, keys and values, including the length bytes) can not exceed 424 bytes.
    #[arg(long, value_parser = metadata_key_value_parser, action = ArgAction::Append)]
    pub metadata: Vec<MetadataProductKeyValue>,
}

/// First element is the product index, using the order the products are passed in on the command
/// line.
pub type MetadataProductKeyValue = (usize, String, String);

/// Metadata for a single key/value pair.  Product assignment is implicit.
pub type MetadataKeyValueRef<'source> = (&'source str, &'source str);

fn metadata_key_value_parser(input: &str) -> Result<MetadataProductKeyValue, String> {
    static RE: OnceCell<Regex> = OnceCell::new();
    let re = RE.get_or_init(|| {
        Regex::new(
            r"(?x)
             ^
             (?-u: ( \d+ ) : )?
             ( [^=]+ ) = ( .* )
             $",
        )
        .unwrap()
    });

    let Some(matches) = re.captures(input) else {
        return Err("`metadata` value must contain an '=' to separate the key from the value")?;
    };

    let (index, key, value) = {
        let index = match matches.get(1) {
            Some(index) => index
                .as_str()
                .parse::<usize>()
                .map_err(|err| format!("`index` should be an integer.  Err: {err:?}"))?,
            None => 0,
        };

        (
            index,
            matches.get(2).unwrap().as_str(),
            matches.get(3).unwrap().as_str(),
        )
    };

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

    Ok((index, key.to_owned(), value.to_owned()))
}

/// Additional validation of the [`AddProductArgs`] instances.
impl AddProductArgs {
    pub fn check_are_valid(&self) -> Result<()> {
        let Self {
            product_keypair,
            metadata,
            ..
        } = self;

        for metadata_arg in metadata {
            let index = metadata_arg.0;
            let max_index = product_keypair.len();
            if index >= max_index {
                bail!(
                    "--metadata index must refer to one of the product arguments.\n\
                     Got index of {index}, that exceeds the number of products: {max_index}."
                );
            }
        }

        Ok(())
    }
}

pub fn per_product_metadata(
    metadata: &[MetadataProductKeyValue],
) -> Vec<Vec<MetadataKeyValueRef<'_>>> {
    let mut per_product: Vec<Vec<MetadataKeyValueRef>> = vec![];

    for (index, key, value) in metadata {
        if *index >= per_product.len() {
            per_product.resize(index + 1, vec![]);
        }

        per_product[*index].push((key, value));
    }

    per_product
}
