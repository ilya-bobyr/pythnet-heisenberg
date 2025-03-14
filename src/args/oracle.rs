use clap::Subcommand;

pub mod update_permissions;

#[derive(Subcommand, Debug)]
#[command(name = "oracle")]
pub enum Command {
    /// Configures access permissions for the Oracle program.
    UpdatePermissions(update_permissions::UpdatePermissionsArgs),
}
