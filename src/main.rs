use std::io;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{debug, error, info};

use rhoxy_socks::config::{ProxyConfig, ConnectionConfig};

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

    start_server(server_addr, Arc::new(config)).await?;
    Ok(())
}

async fn start_server(server_addr: std::net::SocketAddr, config: Arc<ProxyConfig>) -> io::Result<()> {
    info!("Starting server on {}", server_addr);
    
    let listener: TcpListener = match TcpListener::bind(&server_addr).await {
        Ok(listener) => {
            info!("Server listening on {}", server_addr);
            listener
        }
        Err(e) => {
            error!("Failed to bind to {}: {}", server_addr, e);
            return Err(e);
        }
    };

    loop {
        let (socket, socket_addr) = match listener.accept().await {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to accept connection: {}", e);
                continue;
            }
        };
        debug!("Accepted connection from {}", socket_addr);
        tokio::spawn(async move {
            if let Err(e) = rhoxy_socks::handle_connection(socket, socket_addr).await {
                error!("Connection error for {}: {}", socket_addr, e);
            }
        });
    }
}

