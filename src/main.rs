use clap::Parser;
use tokio::net::{TcpListener, TcpStream};
use tracing::info;
use std::net::{SocketAddr, ToSocketAddrs};

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
async fn main() {
    let args = Args::parse();

    if args.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    let server_addr = format!("{}:{}", args.host, args.port)
        .to_socket_addrs()
        .expect("Failed to parse server address")
        .next()
        .expect("No addresses found");

    start_server(server_addr).await;
}

async fn start_server(server_addr: SocketAddr) {
    info!("Starting server on {}", server_addr);
    let listener = TcpListener::bind(&server_addr)
        .await
        .expect("Failed to bind to address");

    loop {
        let (socket, socket_addr) = listener
            .accept()
            .await
            .expect("Failed to accept connection");
        info!("Accepted connection from {}", socket_addr);
        tokio::spawn(handle_connection(socket));
    }
}

/// Should be organized into these steps:
/// 1. Handle authentication negotation
/// 2. Handle client request (command + destination addr)
/// 2.1 Handle connect request
/// 2.2 Handle bind request
/// 2.3 Handle UDP associate request
async fn handle_connection(socket: TcpStream) {
    // TODO: implement connection handling
}
