use std::io;
use tokio::net::TcpStream;

/// Should be organized into these steps:
/// 1. Handle authentication negotation
/// 2. Handle client request (command + destination addr)
/// 2.1 Handle connect request
/// 2.2 Handle bind request
/// 2.3 Handle UDP associate request

pub async fn handle_connection(socket: TcpStream) -> io::Result<()> {
    // TODO: implement connection handling
    Ok(())
}
