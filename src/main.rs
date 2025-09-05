use std::io;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{debug, error, info};

use rhoxy_socks::config::{ConnectionConfig, ProxyConfig};

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

async fn start_server(
    server_addr: std::net::SocketAddr,
    config: Arc<ProxyConfig>,
) -> io::Result<()> {
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

    let active_connections = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let connection_config = ConnectionConfig::from(config.as_ref());

    info!(
        "Ready to accept connections (max: {})",
        config.max_connections
    );

    loop {
        let (socket, socket_addr) = match listener.accept().await {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to accept connection: {}", e);
                continue;
            }
        };

        let current_connections = active_connections.load(std::sync::atomic::Ordering::Relaxed);

        if current_connections >= config.max_connections {
            debug!(
                "Connection limit reached ({}/{}), rejecting {}",
                current_connections, config.max_connections, socket_addr
            );
            drop(socket);
            continue;
        }

        active_connections.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let new_count = current_connections + 1;

        debug!(
            "Accepted connection from {} (active: {}/{})",
            socket_addr, new_count, config.max_connections
        );
        let conn_config = connection_config.clone();
        let conn_counter = active_connections.clone();
        tokio::spawn(async move {
            let result =
                rhoxy_socks::handle_connection(socket, socket_addr, conn_config.clone()).await;

            let prev_count = conn_counter.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);

            match result {
                Ok(_) => {
                    if conn_config.metrics_enabled {
                        debug!(
                            "Connection {} completed successfully (active: {})",
                            socket_addr,
                            prev_count - 1
                        );
                    }
                }
                Err(e) => {
                    error!(
                        "Connection error for {}: {} (active: {})",
                        socket_addr,
                        e,
                        prev_count - 1
                    );
                }
            }
        });
    }
}
