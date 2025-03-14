use std::path::PathBuf;

use clap::Args;
use solana_program::pubkey::Pubkey;

use crate::args::JsonRpcUrlArgs;

#[derive(Args, Debug)]
pub struct UpdatePermissionsArgs {
    #[command(flatten)]
    pub json_rpc_url: JsonRpcUrlArgs,

    /// Address of the Oracle program.
    #[arg(long)]
    pub program_id: Pubkey,

    /// A keypair file for the account that would pay for the permissions account.
    ///
    /// It also needs to be an account that controls upgrades of the Oracle program.  This is the
    /// only account that can update permissions.
    #[arg(long)]
    pub funding_keypair: PathBuf,

    /// An address of the permissions account for this Oracle.
    ///
    /// It can be computed like this, and defaults to this value if not specified:
    ///
    ///   solana find-program-derived-address
    ///     "[Oracle program pubkey]" string:permissions
    #[arg(long)]
    pub permissions_account: Option<Pubkey>,

    /// An account that would have permissions to perform any privileged operations on this Oracle.
    #[arg(long)]
    pub master_authority: Pubkey,

    /// This account does not have any additional permissions, according to the Oracle program code.
    //
    // The `PermissionAccount` doc says that this account can:
    //
    // - Add mapping accounts
    // - Add price accounts
    // - Add product accounts
    // - Delete price accounts
    // - Delete product accounts
    // - Update product accounts
    //
    // But `AddMapping` instruction has been removed, there is only an `InitMapping` instruction.
    // `AddProduct` and all the other commands use `PermissionAccount::is_authorized` that ignores
    // the `data_curation_authority` completely.
    //
    // See `pyth-client/program/rust/src/accounts/permission.rs`.
    #[arg(long)]
    pub data_curation_authority: Pubkey,

    /// An account that would have permissions to resize price accounts, via the
    /// `ResizePriceAccount` instruction.
    //
    // The `PermissionAccount` doc says that this account can:
    //
    // - Add publishers
    // - Delete publishers
    // - Set minimum number of publishers
    //
    // But `AddPublisher`, `DelPublisher` and `SetMinPub` instruction use
    // `PermissionAccount::is_authorized` that does not allow `security_authority` access for any of
    // this commands.
    //
    // See `pyth-client/program/rust/src/accounts/permission.rs`.
    #[arg(long)]
    pub security_authority: Pubkey,
}
