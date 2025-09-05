use std::{io, net::SocketAddr};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use tracing::{debug, error};

use crate::connection::{Method, SOCKS5_VERSION};

#[derive(Debug)]
pub struct HandshakeRequest {
    pub version: u8,
    pub nmethods: u8,
    pub methods: Vec<u8>,
}

pub async fn perform_handshake<R, W>(
    reader: &mut BufReader<R>,
    writer: &mut BufWriter<W>,
    client_addr: SocketAddr,
) -> io::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    debug!("Performing handshake for client {}", client_addr);

    let handshake_request = parse_client_greeting(reader).await?;
    debug!(
        "Parsed client greeting for {}: {:?}",
        client_addr, handshake_request
    );

    handle_client_greeting(&handshake_request, writer).await?;
    debug!("Completed handshake for client {}", client_addr);

    Ok(())
}

async fn parse_client_greeting<R>(reader: &mut BufReader<R>) -> io::Result<HandshakeRequest>
where
    R: AsyncRead + Unpin,
{
    let version = reader.read_u8().await?;
    if version != SOCKS5_VERSION {
        error!("Invalid SOCKS version: {}", version);
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Expected SOCKS version {}, got {}", SOCKS5_VERSION, version),
        ));
    }
    let nmethods = reader.read_u8().await?;

    let mut methods: Vec<u8> = vec![0; nmethods as usize];
    reader.read_exact(&mut methods).await?;

    Ok(HandshakeRequest {
        version,
        nmethods,
        methods,
    })
}

async fn handle_client_greeting<W>(
    handshake_request: &HandshakeRequest,
    writer: &mut BufWriter<W>,
) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    // TODO: Implement method negotation and those specific methods
    let response = [SOCKS5_VERSION, Method::NO_AUTHENTICATION_REQUIRED];
    writer.write_all(&response).await?;
    writer.flush().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn test_parse_client_greeting_valid() {
        let (mut client, server) = tokio::io::duplex(1024);
        client.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let request = parse_client_greeting(&mut reader)
            .await
            .expect("Should parse valid greeting");
        assert_eq!(request.version, SOCKS5_VERSION);
        assert_eq!(request.nmethods, 1);
        assert_eq!(request.methods, vec![0x00]);
    }

    #[tokio::test]
    async fn test_parse_client_greeting_invalid_version() {
        let (mut client, server) = tokio::io::duplex(1024);
        client.write_all(&[0x04, 0x01, 0x00]).await.unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let result = parse_client_greeting(&mut reader).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Expected SOCKS version 5"));
    }

    #[tokio::test]
    async fn test_handle_client_greeting_no_auth() {
        let request = HandshakeRequest {
            version: SOCKS5_VERSION,
            nmethods: 1,
            methods: vec![Method::NO_AUTHENTICATION_REQUIRED],
        };

        let (server, mut client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(server);

        handle_client_greeting(&request, &mut writer)
            .await
            .expect("Should handle no-auth");
        writer.flush().await.unwrap();

        let mut response = [0u8; 2];
        client.read_exact(&mut response).await.unwrap();
        assert_eq!(
            response,
            [SOCKS5_VERSION, Method::NO_AUTHENTICATION_REQUIRED]
        );
    }

    #[tokio::test]
    async fn test_parse_client_greeting_zero_methods() {
        let (mut client, server) = tokio::io::duplex(1024);
        client.write_all(&[0x05, 0x00]).await.unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let request = parse_client_greeting(&mut reader)
            .await
            .expect("Should parse zero methods");
        assert_eq!(request.version, SOCKS5_VERSION);
        assert_eq!(request.nmethods, 0);
        assert_eq!(request.methods, Vec::<u8>::new());
    }

    #[tokio::test]
    async fn test_parse_client_greeting_multiple_methods() {
        let (mut client, server) = tokio::io::duplex(1024);
        client
            .write_all(&[0x05, 0x03, 0x00, 0x01, 0x02])
            .await
            .unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let request = parse_client_greeting(&mut reader)
            .await
            .expect("Should parse multiple methods");
        assert_eq!(request.version, SOCKS5_VERSION);
        assert_eq!(request.nmethods, 3);
        assert_eq!(request.methods, vec![0x00, 0x01, 0x02]);
    }

    #[tokio::test]
    async fn test_parse_client_greeting_invalid_version_4() {
        let (mut client, server) = tokio::io::duplex(1024);
        client.write_all(&[0x04, 0x01, 0x00]).await.unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let result = parse_client_greeting(&mut reader).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Expected SOCKS version 5, got 4"));
    }

    #[tokio::test]
    async fn test_parse_client_greeting_version_zero() {
        let (mut client, server) = tokio::io::duplex(1024);
        client.write_all(&[0x00, 0x01, 0x00]).await.unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let result = parse_client_greeting(&mut reader).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Expected SOCKS version 5, got 0"));
    }

    #[tokio::test]
    async fn test_parse_client_greeting_version_six() {
        let (mut client, server) = tokio::io::duplex(1024);
        client.write_all(&[0x06, 0x01, 0x00]).await.unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let result = parse_client_greeting(&mut reader).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Expected SOCKS version 5, got 6"));
    }

    #[tokio::test]
    async fn test_parse_client_greeting_truncated_after_version() {
        let (mut client, server) = tokio::io::duplex(1024);
        client.write_all(&[0x05]).await.unwrap();
        client.flush().await.unwrap();
        drop(client);

        let mut reader = BufReader::new(server);
        let result = parse_client_greeting(&mut reader).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    }

    #[tokio::test]
    async fn test_parse_client_greeting_truncated_after_nmethods() {
        let (mut client, server) = tokio::io::duplex(1024);
        client.write_all(&[0x05, 0x02]).await.unwrap();
        client.flush().await.unwrap();
        drop(client);

        let mut reader = BufReader::new(server);
        let result = parse_client_greeting(&mut reader).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    }

    #[tokio::test]
    async fn test_parse_client_greeting_partial_methods() {
        let (mut client, server) = tokio::io::duplex(1024);
        client.write_all(&[0x05, 0x03, 0x00, 0x01]).await.unwrap(); // Says 3 methods but only provides 2
        client.flush().await.unwrap();
        drop(client);

        let mut reader = BufReader::new(server);
        let result = parse_client_greeting(&mut reader).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    }

    #[tokio::test]
    async fn test_parse_client_greeting_max_methods() {
        let (mut client, server) = tokio::io::duplex(1024);
        let mut data = vec![0x05, 0xFF]; // 255 methods
        data.extend(vec![0x00; 255]); // All no-auth methods
        client.write_all(&data).await.unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let request = parse_client_greeting(&mut reader)
            .await
            .expect("Should handle max methods");
        assert_eq!(request.nmethods, 255);
        assert_eq!(request.methods.len(), 255);
    }

    #[tokio::test]
    async fn test_handle_client_greeting_unsupported_methods() {
        let request = HandshakeRequest {
            version: SOCKS5_VERSION,
            nmethods: 2,
            methods: vec![0x01, 0x02], // GSSAPI and USERNAME/PASSWORD (not supported)
        };

        let (server, mut client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(server);

        handle_client_greeting(&request, &mut writer)
            .await
            .expect("Should handle unsupported methods");
        writer.flush().await.unwrap();

        let mut response = [0u8; 2];
        client.read_exact(&mut response).await.unwrap();
        // Just returns no auth required for now
        assert_eq!(
            response,
            [SOCKS5_VERSION, Method::NO_AUTHENTICATION_REQUIRED]
        );
    }

    #[tokio::test]
    async fn test_handle_client_greeting_mixed_methods() {
        let request = HandshakeRequest {
            version: SOCKS5_VERSION,
            nmethods: 3,
            methods: vec![0x01, 0x00, 0x02],
        };

        let (server, mut client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(server);

        handle_client_greeting(&request, &mut writer)
            .await
            .expect("Should handle mixed methods");
        writer.flush().await.unwrap();

        let mut response = [0u8; 2];
        client.read_exact(&mut response).await.unwrap();
        assert_eq!(
            response,
            [SOCKS5_VERSION, Method::NO_AUTHENTICATION_REQUIRED]
        );
    }
}
