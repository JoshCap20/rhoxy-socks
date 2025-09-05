pub mod connection;
pub mod config;

use std::io;
use std::net::SocketAddr;
use tokio::io::{BufReader, BufWriter};
use tokio::net::TcpStream;
use tracing::debug;

pub async fn handle_connection(
    mut stream: TcpStream,
    client_addr: SocketAddr,
    config: config::ConnectionConfig,
) -> io::Result<()> {
    debug!("Handling connection from {}", client_addr);
    let (reader, writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    // might want to propagate errors to here to send error reply
    connection::handshake::perform_handshake(&mut reader, &mut writer, client_addr).await?;
    connection::handler::handle_request(&mut reader, &mut writer, client_addr).await?;

    Ok(())
}
