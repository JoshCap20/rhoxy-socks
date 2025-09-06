use std::io;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use rhoxy_socks::{config::{ConnectionConfig, ProxyConfig}, server::ProxyServer};

#[tokio::main]
async fn main() -> io::Result<()> {
    let config = ProxyConfig::from_args();

    if let Err(e) = config.validate() {
        eprintln!("Configuration error: {}", e);
        std::process::exit(1);
    }

    tracing_subscriber::fmt()
        .with_max_level(config.tracing_level())
        .init();

    config.display_summary();

    let server_addr = match config.server_addr() {
        Ok(addr) => addr,
        Err(e) => {
            error!("Failed to parse server address: {}", e);
            return Err(e);
        }
    };

    let mut server = ProxyServer::new(server_addr, Arc::new(config)).await?;
    server.run().await
}
