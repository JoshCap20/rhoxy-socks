pub mod address_type;
pub mod command;
pub mod error;
pub mod method;
pub mod reply;
pub mod request;

use std::{io, net::SocketAddr};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use tracing::debug;

use crate::connection::{
    address_type::AddressType, error::SocksError, method::method_handler::MethodHandler,
};

pub const SOCKS5_VERSION: u8 = 0x05;
pub const RESERVED: u8 = 0x00;
// For errors prior to established connection (in which case command returns the host, port)
// these are used for connection errors (i.e. dns failure in domain name translation)
pub const ERROR_ADDR: [u8; 4] = [0, 0, 0, 0];
pub const ERROR_PORT: u16 = 0;

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

async fn resolve_domain(domain: &str) -> io::Result<Vec<std::net::SocketAddr>> {
    let addrs: Vec<_> = tokio::net::lookup_host((domain, 0)).await?.collect();
    Ok(addrs)
}

pub async fn send_reply<W>(
    writer: &mut BufWriter<W>,
    reply_code: u8,
    addr_type: u8,
    addr_bytes: &[u8],
    port: u16,
) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    // TODO: Single call with stack-allocated buffer
    writer.write_u8(SOCKS5_VERSION).await?;
    writer.write_u8(reply_code).await?;
    writer.write_u8(RESERVED).await?;
    writer.write_u8(addr_type).await?;
    writer.write_all(addr_bytes).await?;
    writer.write_u16(port).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn send_socks_error_reply<W>(
    writer: &mut BufWriter<W>,
    socks_error: &SocksError,
) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    // CommandResult error reply does not use this
    // but socks request parsing failures do
    let error_code = socks_error.to_reply_code();
    send_error_reply(writer, error_code).await
}

pub async fn send_error_reply<W>(writer: &mut BufWriter<W>, error_code: u8) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    send_reply(
        writer,
        error_code,
        AddressType::IPV4,
        &ERROR_ADDR,
        ERROR_PORT,
    )
    .await
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
