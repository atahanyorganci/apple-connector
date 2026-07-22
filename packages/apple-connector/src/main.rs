use std::error::Error;

use apple_connector::Cli;
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("apple_connector=info".parse()?),
        )
        .init();

    let cli = Cli::parse();
    apple_connector::run(cli).await
}
