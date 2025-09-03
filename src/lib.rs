pub mod connection;

use std::io;
use std::net::SocketAddr;
use tokio::io::{BufReader, BufWriter};
use tokio::net::TcpStream;

pub async fn handle_connection(stream: TcpStream, client_addr: SocketAddr) -> io::Result<()> {
    let (reader, writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    connection::handshake::perform_handshake(&mut reader, &mut writer, client_addr).await?;
    connection::request::handle_request(&mut reader, &mut writer, client_addr).await?;

    Ok(())
}
