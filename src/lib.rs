use std::io;
use std::net::SocketAddr;
use tokio::net::TcpStream;

/// Should be organized into these steps:
/// 1. Handle handshake/authentication negotation
/// 2. Handle client request (command + destination addr)
/// 2.1 Handle connect request
/// 2.2 Handle bind request
/// 2.3 Handle UDP associate request

pub async fn handle_connection(socket: TcpStream, client_addr: SocketAddr) -> io::Result<()> {
    // TODO: implement connection handling

    // 1. handle initial request
    handle_handshake(socket, client_addr).await?;

    Ok(())
}

async fn handle_handshake(socket: TcpStream, client_addr: SocketAddr) -> io::Result<()> {
    // TODO: implement handshake handling

    // 1. parse client greeting
    /// 	VER	NAUTH	AUTH
    /// Byte count	1	1	variable
    Ok(())
}
