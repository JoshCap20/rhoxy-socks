use std::{io, sync::Arc};

use tokio::{net::TcpListener, signal, sync::broadcast};
use tracing::{debug, error, info, warn};

use crate::{
    config::{ConnectionConfig, ProxyConfig},
    handle_connection,
};

struct ConnectionGuard {
    counter: Arc<std::sync::atomic::AtomicUsize>,
}

impl ConnectionGuard {
    fn new(counter: Arc<std::sync::atomic::AtomicUsize>) -> Self {
        Self { counter }
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let prev_count = self
            .counter
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        debug!("Connection finished (active: {})", prev_count - 1);
    }
}

pub struct ProxyServer {
    listener: TcpListener,
    config: Arc<ProxyConfig>,
    connection_config: ConnectionConfig,
    active_connections: Arc<std::sync::atomic::AtomicUsize>,
    shutdown_tx: broadcast::Sender<()>,
}

impl ProxyServer {
    pub async fn new(
        server_addr: std::net::SocketAddr,
        config: Arc<ProxyConfig>,
    ) -> io::Result<Self> {
        info!("Starting server on {}", server_addr);

        let listener = match TcpListener::bind(&server_addr).await {
            Ok(listener) => {
                info!("Server listening on {}", server_addr);
                listener
            }
            Err(e) => {
                error!("Failed to bind to {}: {}", server_addr, e);
                return Err(e);
            }
        };

        let connection_config = ConnectionConfig::from(config.as_ref());
        let active_connections = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (shutdown_tx, _) = broadcast::channel(1);

        Ok(Self {
            listener,
            config,
            connection_config,
            active_connections,
            shutdown_tx,
        })
    }

    pub async fn run(&mut self) -> io::Result<()> {
        info!(
            "Ready to accept connections (max: {})",
            self.config.max_connections
        );

        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::select! {
            result = self.accept_loop() => {
                error!("Accept loop terminated unexpectedly: {:?}", result);
                result
            }
            _ = self.wait_for_shutdown() => {
                info!("Shutdown signal received, stopping server");
                self.shutdown().await;
                Ok(())
            }
            _ = shutdown_rx.recv() => {
                info!("Shutdown requested, stopping server");
                self.shutdown().await;
                Ok(())
            }
        }
    }

    async fn accept_loop(&self) -> io::Result<()> {
        loop {
            let (socket, socket_addr) = match self.listener.accept().await {
                Ok(result) => result,
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                    continue;
                }
            };

            if self.should_reject_connection()? {
                debug!("Connection limit reached, rejecting {}", socket_addr);
                drop(socket);
                continue;
            }

            self.spawn_connection_handler(socket, socket_addr).await;
        }
    }

    fn should_reject_connection(&self) -> io::Result<bool> {
        let new_count = self
            .active_connections
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;

        if new_count > self.config.max_connections {
            self.active_connections
                .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            return Ok(true);
        }

        Ok(false)
    }

    async fn spawn_connection_handler(
        &self,
        socket: tokio::net::TcpStream,
        socket_addr: std::net::SocketAddr,
    ) {
        let active_count = self
            .active_connections
            .load(std::sync::atomic::Ordering::Relaxed);
        debug!(
            "Accepted connection from {} (active: {}/{})",
            socket_addr, active_count, self.config.max_connections
        );

        let conn_config = self.connection_config.clone();
        let conn_counter = self.active_connections.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            let _connection_guard = ConnectionGuard::new(conn_counter.clone());

            let result = tokio::select! {
                result = handle_connection(socket, socket_addr, conn_config.clone()) => {
                    result
                }
                _ = shutdown_rx.recv() => {
                    debug!("Connection {} interrupted by shutdown", socket_addr);
                    return;
                }
            };

            match result {
                Ok(_) => {
                    debug!("Connection {} completed successfully", socket_addr);
                }
                Err(e) => {
                    error!("Connection error for {}: {}", socket_addr, e);
                }
            }
        });
    }

    async fn wait_for_shutdown(&self) {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                info!("Received SIGINT (Ctrl+C)");
            }
            _ = terminate => {
                info!("Received SIGTERM");
            }
        }
    }

    async fn shutdown(&self) {
        let active_count = self
            .active_connections
            .load(std::sync::atomic::Ordering::Relaxed);

        if active_count > 0 {
            info!(
                "Gracefully shutting down with {} active connections",
                active_count
            );
            let _ = self.shutdown_tx.send(());
            let start = tokio::time::Instant::now();

            while self
                .active_connections
                .load(std::sync::atomic::Ordering::Relaxed)
                > 0
                && start.elapsed() < self.connection_config.shutdown_timeout
            {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            let remaining = self
                .active_connections
                .load(std::sync::atomic::Ordering::Relaxed);
            if remaining > 0 {
                warn!(
                    "Shutdown timeout reached, {} connections still active - forcing close",
                    remaining
                );
                let _ = self.shutdown_tx.send(());
            } else {
                info!("All connections closed gracefully");
            }
        } else {
            info!("Shutting down with no active connections");
        }
    }
}
