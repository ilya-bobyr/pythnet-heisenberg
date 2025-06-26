use clap::Args;
use solana_program::pubkey::Pubkey;

#[derive(Args, Debug)]
pub struct FeatureArgs {
    /// An address of the feature to activate.
    #[arg(long)]
    pub address: Pubkey,

    /// Do not mark the feature as already active.
    ///
    /// If not specified, feature accounts are created in a state as if the activation has already
    /// happened.
    ///
    /// Creating a feature account that is not initially active will cause the feature activation to
    /// happen at the end of the first epoch.  This might(?) matter for features that have any logic
    /// attached to the feature activation itself.
    #[arg(long)]
    pub not_active: bool,
}
