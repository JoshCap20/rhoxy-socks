use std::{io, net::SocketAddr};

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::io::{AsyncReadExt, BufReader, BufWriter};
use tracing::{debug, error};

use crate::connection::command::Command;
use crate::connection::{ATYP_DOMAIN, ATYP_IPV4, ATYP_IPV6, RESERVED, SOCKS5_VERSION};

#[derive(Debug)]
pub struct SocksRequest {
    pub version: u8,
    pub command: u8,
    pub reserved: u8,
    pub address_type: u8,
    pub dest_addr: std::net::IpAddr,
    pub dest_port: u16,
}

pub async fn handle_request<R, W>(
    reader: &mut BufReader<R>,
    writer: &mut BufWriter<W>,
    client_addr: SocketAddr,
) -> io::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    debug!("Handling request from {}", client_addr);

    let client_request = parse_request(reader).await?;
    debug!(
        "Parsed client request from {}: {:?}",
        client_addr, client_request
    );

    handle_client_request(client_request, client_addr, reader, writer).await?;

    Ok(())
}

async fn parse_request<R>(reader: &mut BufReader<R>) -> io::Result<SocksRequest>
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

    let command = reader.read_u8().await?;
    let reserved = reader.read_u8().await?;
    if reserved != RESERVED {
        error!("Invalid reserved byte: {}", reserved);
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Reserved byte must be 0x00",
        ));
    }

    let address_type = reader.read_u8().await?;

    let dest_addr = match address_type {
        ATYP_IPV4 => {
            let mut addr = [0u8; 4];
            reader.read_exact(&mut addr).await?;
            std::net::IpAddr::from(addr)
        }
        ATYP_DOMAIN => {
            error!("Domain name address type not yet supported");
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Domain name resolution not implemented",
            ));
        }
        ATYP_IPV6 => {
            let mut addr = [0u8; 16];
            reader.read_exact(&mut addr).await?;
            std::net::IpAddr::from(addr)
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unsupported address type",
            ));
        }
    };
    let dest_port = reader.read_u16().await?;

    Ok(SocksRequest {
        version,
        command,
        reserved,
        address_type,
        dest_addr,
        dest_port,
    })
}

async fn handle_client_request<R, W>(
    client_request: SocksRequest,
    client_addr: SocketAddr,
    reader: &mut BufReader<R>,
    writer: &mut BufWriter<W>,
) -> io::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let command: Command = Command::parse_command(client_request.command).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid command from client {}", client_addr),
        )
    })?;

    command
        .execute(client_request, client_addr, reader, writer)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::connection::{ATYP_IPV4, ATYP_IPV6, CONNECT, REPLY_SUCCESS};

    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn test_parse_request_connect_ipv4() {
        let (mut client, server) = tokio::io::duplex(1024);
        client
            .write_all(&[0x05, 0x01, 0x00, 0x01, 127, 0, 0, 1, 0, 80])
            .await
            .unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let request = parse_request(&mut reader)
            .await
            .expect("Should parse valid request");
        assert_eq!(request.version, SOCKS5_VERSION);
        assert_eq!(request.command, CONNECT);
        assert_eq!(request.address_type, ATYP_IPV4);
        assert_eq!(
            request.dest_addr,
            std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))
        );
        assert_eq!(request.dest_port, 80);
    }

    #[tokio::test]
    async fn test_parse_request_invalid_atyp() {
        let (mut client, server) = tokio::io::duplex(1024);
        client
            .write_all(&[0x05, 0x01, 0x00, 0xFF, 127, 0, 0, 1, 0, 80])
            .await
            .unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let result = parse_request(&mut reader).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn test_send_reply_success_ipv6() {
        let (server, mut client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(server);

        let addr_bytes = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1).octets().to_vec();
        send_reply(&mut writer, REPLY_SUCCESS, ATYP_IPV6, &addr_bytes, 8080)
            .await
            .expect("Should send reply");
        writer.flush().await.unwrap();

        let mut response = vec![0u8; 4 + 16 + 2];
        client.read_exact(&mut response).await.unwrap();
        assert_eq!(response[0], SOCKS5_VERSION);
        assert_eq!(response[1], REPLY_SUCCESS);
        assert_eq!(response[3], ATYP_IPV6);
        assert_eq!(&response[4..20], &addr_bytes);
        assert_eq!(&response[20..22], 8080u16.to_be_bytes());
    }

    #[tokio::test]
    async fn test_parse_request_connect_ipv6() {
        let (mut client, server) = tokio::io::duplex(1024);
        let addr_bytes = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1).octets();
        let mut data = vec![0x05, 0x01, 0x00, 0x04];
        data.extend_from_slice(&addr_bytes);
        data.extend_from_slice(&443u16.to_be_bytes());
        client.write_all(&data).await.unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let request = parse_request(&mut reader)
            .await
            .expect("Should parse IPv6 request");
        assert_eq!(request.version, SOCKS5_VERSION);
        assert_eq!(request.command, CONNECT);
        assert_eq!(request.address_type, ATYP_IPV6);
        assert_eq!(
            request.dest_addr,
            std::net::IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1))
        );
        assert_eq!(request.dest_port, 443);
    }

    #[tokio::test]
    async fn test_parse_request_domain_name_unsupported() {
        let (mut client, server) = tokio::io::duplex(1024);
        client
            .write_all(&[
                0x05, 0x01, 0x00, 0x03, // ATYP_DOMAIN
                0x0B, // Domain length (11)
                b'e', b'x', b'a', b'm', b'p', b'l', b'e', b'.', b'c', b'o', b'm', 0x00,
                0x50, // Port 80
            ])
            .await
            .unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let result = parse_request(&mut reader).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);
        assert!(
            err.to_string()
                .contains("Domain name resolution not implemented")
        );
    }

    #[tokio::test]
    async fn test_parse_request_bind_command() {
        let (mut client, server) = tokio::io::duplex(1024);
        client
            .write_all(&[0x05, 0x02, 0x00, 0x01, 127, 0, 0, 1, 0, 80]) // BIND command
            .await
            .unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let request = parse_request(&mut reader)
            .await
            .expect("Should parse BIND request");
        assert_eq!(request.command, BIND);
        assert_eq!(
            request.dest_addr,
            std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))
        );
    }

    #[tokio::test]
    async fn test_parse_request_udp_associate_command() {
        let (mut client, server) = tokio::io::duplex(1024);
        client
            .write_all(&[0x05, 0x03, 0x00, 0x01, 127, 0, 0, 1, 0, 80]) // UDP_ASSOCIATE command
            .await
            .unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let request = parse_request(&mut reader)
            .await
            .expect("Should parse UDP_ASSOCIATE request");
        assert_eq!(request.command, UDP_ASSOCIATE);
        assert_eq!(
            request.dest_addr,
            std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))
        );
    }

    #[tokio::test]
    async fn test_parse_request_invalid_command() {
        let (mut client, server) = tokio::io::duplex(1024);
        client
            .write_all(&[0x05, 0xFF, 0x00, 0x01, 127, 0, 0, 1, 0, 80]) // Invalid command
            .await
            .unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let request = parse_request(&mut reader)
            .await
            .expect("Should parse request with invalid command");
        assert_eq!(request.command, 0xFF);
    }

    #[tokio::test]
    async fn test_parse_request_invalid_reserved_byte() {
        let (mut client, server) = tokio::io::duplex(1024);
        client
            .write_all(&[0x05, 0x01, 0xFF, 0x01, 127, 0, 0, 1, 0, 80]) // Invalid reserved byte
            .await
            .unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let result = parse_request(&mut reader).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Reserved byte must be 0x00"));
    }

    #[tokio::test]
    async fn test_parse_request_invalid_version() {
        let (mut client, server) = tokio::io::duplex(1024);
        client
            .write_all(&[0x04, 0x01, 0x00, 0x01, 127, 0, 0, 1, 0, 80]) // SOCKS4 instead of SOCKS5
            .await
            .unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let result = parse_request(&mut reader).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Expected SOCKS version 5, got 4"));
    }

    #[tokio::test]
    async fn test_parse_request_port_zero() {
        let (mut client, server) = tokio::io::duplex(1024);
        client
            .write_all(&[0x05, 0x01, 0x00, 0x01, 127, 0, 0, 1, 0, 0]) // Port 0
            .await
            .unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let request = parse_request(&mut reader)
            .await
            .expect("Should parse port 0");
        assert_eq!(request.dest_port, 0);
    }

    #[tokio::test]
    async fn test_parse_request_port_max() {
        let (mut client, server) = tokio::io::duplex(1024);
        client
            .write_all(&[0x05, 0x01, 0x00, 0x01, 127, 0, 0, 1, 0xFF, 0xFF]) // Port 65535
            .await
            .unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let request = parse_request(&mut reader)
            .await
            .expect("Should parse port 65535");
        assert_eq!(request.dest_port, 65535);
    }

    #[tokio::test]
    async fn test_parse_request_truncated_after_version() {
        let (mut client, server) = tokio::io::duplex(1024);
        client.write_all(&[0x05]).await.unwrap();
        client.flush().await.unwrap();
        drop(client);

        let mut reader = BufReader::new(server);
        let result = parse_request(&mut reader).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    }

    #[tokio::test]
    async fn test_parse_request_truncated_after_command() {
        let (mut client, server) = tokio::io::duplex(1024);
        client.write_all(&[0x05, 0x01]).await.unwrap();
        client.flush().await.unwrap();
        drop(client);

        let mut reader = BufReader::new(server);
        let result = parse_request(&mut reader).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    }

    #[tokio::test]
    async fn test_parse_request_truncated_ipv4_address() {
        let (mut client, server) = tokio::io::duplex(1024);
        client
            .write_all(&[0x05, 0x01, 0x00, 0x01, 127, 0]) // Incomplete IPv4
            .await
            .unwrap();
        client.flush().await.unwrap();
        drop(client);

        let mut reader = BufReader::new(server);
        let result = parse_request(&mut reader).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    }

    #[tokio::test]
    async fn test_parse_request_truncated_ipv6_address() {
        let (mut client, server) = tokio::io::duplex(1024);
        let mut data = vec![0x05, 0x01, 0x00, 0x04];
        data.extend_from_slice(&[0; 8]); // Half an IPv6 address
        client.write_all(&data).await.unwrap();
        client.flush().await.unwrap();
        drop(client);

        let mut reader = BufReader::new(server);
        let result = parse_request(&mut reader).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    }

    #[tokio::test]
    async fn test_parse_request_truncated_port() {
        let (mut client, server) = tokio::io::duplex(1024);
        client
            .write_all(&[0x05, 0x01, 0x00, 0x01, 127, 0, 0, 1, 0]) // Missing second port byte
            .await
            .unwrap();
        client.flush().await.unwrap();
        drop(client);

        let mut reader = BufReader::new(server);
        let result = parse_request(&mut reader).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    }

    #[tokio::test]
    async fn test_send_reply_success_ipv4() {
        let (server, mut client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(server);

        let addr_bytes = Ipv4Addr::new(192, 168, 1, 1).octets().to_vec();
        send_reply(&mut writer, REPLY_SUCCESS, ATYP_IPV4, &addr_bytes, 3128)
            .await
            .expect("Should send IPv4 reply");
        writer.flush().await.unwrap();

        let mut response = vec![0u8; 4 + 4 + 2];
        client.read_exact(&mut response).await.unwrap();
        assert_eq!(response[0], SOCKS5_VERSION);
        assert_eq!(response[1], REPLY_SUCCESS);
        assert_eq!(response[2], RESERVED);
        assert_eq!(response[3], ATYP_IPV4);
        assert_eq!(&response[4..8], &addr_bytes);
        assert_eq!(&response[8..10], 3128u16.to_be_bytes());
    }

    #[tokio::test]
    async fn test_send_reply_error_codes() {
        let error_codes = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

        for &error_code in &error_codes {
            let (server, mut client) = tokio::io::duplex(1024);
            let mut writer = BufWriter::new(server);

            let addr_bytes = vec![127, 0, 0, 1];
            send_reply(&mut writer, error_code, ATYP_IPV4, &addr_bytes, 0)
                .await
                .expect("Should send error reply");
            writer.flush().await.unwrap();

            let mut response = vec![0u8; 10];
            client.read_exact(&mut response).await.unwrap();
            assert_eq!(response[0], SOCKS5_VERSION);
            assert_eq!(response[1], error_code);
            assert_eq!(response[2], RESERVED);
        }
    }
}
