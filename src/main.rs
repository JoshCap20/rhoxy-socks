use clap::Parser;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "localhost")]
    host: String,

    #[arg(short, long, default_value = "8080", help = "Port to listen on")]
    port: u16,

    #[arg(long, help = "Enable debug logging")]
    verbose: bool,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_max_level(if args.verbose {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .init();

    let server_addr = format!("{}:{}", args.host, args.port);
    let server_addr = match server_addr.to_socket_addrs() {
        Ok(mut addrs) => addrs.next().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "No valid socket address found")
        })?,
        Err(e) => {
            error!("Failed to resolve address {}: {}", server_addr, e);
            return Err(e);
        }
    };

    start_server(server_addr).await?;
    Ok(())
}

async fn start_server(server_addr: SocketAddr) -> io::Result<()> {
    info!("Starting server on {}", server_addr.to_string());
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

    loop {
        let (socket, socket_addr) = match listener.accept().await {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to accept connection: {}", e);
                continue;
            }
        };
        info!("Accepted connection from {}", socket_addr);
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket).await {
                error!("Connection error for {}: {}", socket_addr, e);
            }
        });
    }
}

/// Should be organized into these steps:
/// 1. Handle authentication negotation
/// 2. Handle client request (command + destination addr)
/// 2.1 Handle connect request
/// 2.2 Handle bind request
/// 2.3 Handle UDP associate request

async fn handle_connection(socket: TcpStream) -> io::Result<()> {
    // TODO: implement connection handling
    Ok(())
}
