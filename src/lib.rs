pub mod connection;

use std::io;
use std::net::SocketAddr;
use tokio::io::{BufReader, BufWriter};
use tokio::net::TcpStream;

/// Should be organized into these steps:
/// 1. Handle handshake/authentication negotation
/// 2. Handle client request (command + destination addr)
/// 2.1 Handle connect request
/// 2.2 Handle bind request
/// 2.3 Handle UDP associate request

pub async fn handle_connection(stream: TcpStream, client_addr: SocketAddr) -> io::Result<()> {
    // TODO: implement connection handling
    let (reader, writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    // 1. handle initial request
    connection::handshake::perform_handshake(reader, writer, client_addr).await?;

    Ok(())
}
