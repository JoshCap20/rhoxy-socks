use std::{io, net::SocketAddr};
use tokio::io::{AsyncRead, AsyncWrite, BufReader, BufWriter};
use tracing::debug;

use crate::connection::method::method_handler::MethodHandler;

pub async fn perform_handshake<R, W>(
    reader: &mut BufReader<R>,
    writer: &mut BufWriter<W>,
    client_addr: SocketAddr,
    server_methods: &[u8],
) -> io::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    debug!("Performing handshake for client {}", client_addr);

    let client_greeting = match MethodHandler::parse_client_greeting(reader).await {
        Ok(greeting) => greeting,
        Err(e) => {
            debug!(
                "Failed to parse client greeting from {}: {}",
                client_addr, e
            );
            return Err(e);
        }
    };

    debug!(
        "Parsed client greeting from {}: version={}, methods={:?}",
        client_addr, client_greeting.version, client_greeting.methods
    );

    if let Err(validation_error) = client_greeting.validate() {
        debug!(
            "Invalid client greeting from {}: {}",
            client_addr, validation_error
        );
        return Err(io::Error::new(io::ErrorKind::InvalidData, validation_error));
    }

    let _selected_method = MethodHandler::handle_client_methods(
        &client_greeting.methods,
        server_methods,
        writer,
        client_addr,
    )
    .await?;

    debug!("Completed handshake for client {}", client_addr);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};

    #[tokio::test]
    async fn test_perform_handshake_success() {
        let (mut client, server) = duplex(1024);

        // Client sends valid greeting with no-auth method
        client.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
        client.flush().await.unwrap();

        let (client_reader, mut client_writer) = tokio::io::split(client);
        let (server_reader, server_writer) = tokio::io::split(server);

        let mut reader = BufReader::new(server_reader);
        let mut writer = BufWriter::new(server_writer);
        let client_addr = "127.0.0.1:8080".parse().unwrap();
        let server_methods = vec![0x00]; // Support no-auth

        let result =
            perform_handshake(&mut reader, &mut writer, client_addr, &server_methods).await;
        assert!(result.is_ok());

        // Verify response
        let mut response = [0u8; 2];
        let mut client_reader = BufReader::new(client_reader);
        client_reader.read_exact(&mut response).await.unwrap();
        assert_eq!(response, [0x05, 0x00]); // SOCKS5, no-auth
    }

    #[tokio::test]
    async fn test_perform_handshake_no_acceptable_methods() {
        let (mut client, server) = duplex(1024);

        // Client sends greeting with only GSSAPI method
        client.write_all(&[0x05, 0x01, 0x01]).await.unwrap();
        client.flush().await.unwrap();

        let (client_reader, _) = tokio::io::split(client);
        let (server_reader, server_writer) = tokio::io::split(server);

        let mut reader = BufReader::new(server_reader);
        let mut writer = BufWriter::new(server_writer);
        let client_addr = "127.0.0.1:8080".parse().unwrap();
        let server_methods = vec![0x00]; // Only support no-auth

        let result =
            perform_handshake(&mut reader, &mut writer, client_addr, &server_methods).await;
        assert!(result.is_err());
    }
}
