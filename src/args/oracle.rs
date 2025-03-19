use clap::Subcommand;

pub mod add_price;
pub mod add_product;
pub mod add_publisher;
pub mod init_mapping;
pub mod update_permissions;

#[derive(Subcommand, Debug)]
#[command(name = "oracle")]
pub enum Command {
    /// Configures access permissions for the Oracle program.
    UpdatePermissions(update_permissions::UpdatePermissionsArgs),

    /// Initialize a mapping - root account used to describe a set of products, and their prices.
    InitMapping(init_mapping::InitMappingArgs),

    /// Adds one or more products to a mapping.
    AddProduct(add_product::AddProductArgs),

    /// Adds a new price account to a product account.
    AddPrice(add_price::AddPriceArgs),

    /// Adds a publisher to a price account.
    AddPublisher(add_publisher::AddPublisherArgs),
}
