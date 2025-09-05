pub mod config;
pub mod connection;

use std::io;
use std::net::SocketAddr;
use tokio::io::{BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::debug;

pub async fn handle_connection(
    mut stream: TcpStream,
    client_addr: SocketAddr,
    config: config::ConnectionConfig,
) -> io::Result<()> {
    debug!("Handling connection from {}", client_addr);

    if config.tcp_nodelay {
        // fuck it, we enable nodelay on the client stream also
        // only really matters in handle_request when connecting to target
        // which is enabled separately
        if let Err(e) = stream.set_nodelay(true) {
            debug!("Failed to set TCP_NODELAY for {}: {}", client_addr, e);
        }
    }

    // TODO: Apply keep-alive
    let (reader, writer) = stream.into_split();
    let mut reader = BufReader::with_capacity(config.buffer_size, reader);
    let mut writer = BufWriter::with_capacity(config.buffer_size, writer);

    match timeout(
        config.handshake_timeout,
        connection::perform_handshake(
            &mut reader,
            &mut writer,
            client_addr,
            &config.supported_auth_methods,
        ),
    )
    .await
    {
        Ok(result) => result?,
        Err(_) => {
            debug!(
                "Handshake timeout for {} after {:?}",
                client_addr, config.handshake_timeout
            );
            return Err(io::Error::new(io::ErrorKind::TimedOut, "Handshake timeout"));
        }
    }
    match timeout(
        config.connection_timeout,
        connection::request::SocksRequest::handle_request(
            &mut reader,
            &mut writer,
            client_addr,
            config.tcp_nodelay,
        ),
    )
    .await
    {
        Ok(result) => result?,
        Err(_) => {
            debug!(
                "Connection {} timed out after {:?}",
                client_addr, config.connection_timeout
            );
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("Connection timed out after {:?}", config.connection_timeout),
            ));
        }
    }

    Ok(())
}
