use clap::Subcommand;

pub mod initialize;
pub mod initialize_publisher;
pub mod submit_prices;

#[derive(Subcommand, Debug)]
#[command(name = "price-store")]
pub enum Command {
    /// Configures access permissions for the Price Store program.
    Initialize(initialize::InitializeArgs),

    /// Add a new publisher to the Price Store program configuration.
    InitializePublisher(initialize_publisher::InitializePublisherArgs),

    /// Publish a price from a specific publisher.
    SubmitPrices(submit_prices::SubmitPricesArgs),
}
