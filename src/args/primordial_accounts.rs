use clap::Subcommand;

pub mod feature;
pub mod loader_v3;

#[derive(Subcommand, Debug)]
#[command(name = "primordial-accounts")]
pub enum Command {
    /// Output accounts that match a feature activation with `solana feature activate`.
    /// Allows one to activate features as part of genesis.
    Feature(feature::FeatureArgs),

    /// Output accounts that match deployment of a program with loader v3, aka
    /// `BPFLoaderUpgradeab1e11111111111111111111111`.
    LoaderV3(loader_v3::LoaderV3Args),
}
