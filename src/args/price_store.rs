use clap::Subcommand;

pub mod benchmark1;
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

    /// Continuously sends price traffic to the Price Store.
    ///
    /// Will stop either when the specified duration has elapsed (`--duration`) or if an INT or a
    /// TERM signal is received.
    Benchmark1(benchmark1::Benchmark1Args),
}
