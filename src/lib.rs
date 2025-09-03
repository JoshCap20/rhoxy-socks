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
    handle_handshake(reader, writer, client_addr).await?;

    Ok(())
}

async fn handle_handshake(
    mut reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    mut writer: BufWriter<tokio::net::tcp::OwnedWriteHalf>,
    client_addr: SocketAddr,
) -> io::Result<()> {
    // TODO: implement handshake handling

    // 1. parse client greeting
    /// 	VER	NAUTH	AUTH
    /// Byte count	1	1	variable
    Ok(())
}
