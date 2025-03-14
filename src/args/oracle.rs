use clap::Subcommand;

pub mod init_mapping;
pub mod update_permissions;

#[derive(Subcommand, Debug)]
#[command(name = "oracle")]
pub enum Command {
    /// Configures access permissions for the Oracle program.
    UpdatePermissions(update_permissions::UpdatePermissionsArgs),

    /// Initialize a mapping - root account used to describe a set of products, and their prices.
    InitMapping(init_mapping::InitMappingArgs),
}
