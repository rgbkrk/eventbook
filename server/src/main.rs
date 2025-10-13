use anyhow::Result;
use std::env;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Get configuration from environment variables or use defaults

    let port = env::var("EVENTBOOK_PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap_or(3000);

    info!("Starting EventBook server...");

    info!("Port: {}", port);

    // Start the server
    eventbook_server::start_server(port).await?;

    Ok(())
}
