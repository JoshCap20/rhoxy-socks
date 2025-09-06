use std::{io, net::SocketAddr};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, BufReader, BufWriter};
use tracing::{debug, error};

use crate::connection::{
    AddressType, RESERVED, SOCKS5_VERSION, SocksError, command::Command,
    command::connect::handle_data_transfer, reply::Reply, send_error_reply, send_socks_error_reply,
};

#[derive(Debug)]
pub struct SocksRequest {
    pub version: u8,
    pub command: u8,
    pub reserved: u8,
    pub address_type: u8,
    pub dest_addr: std::net::IpAddr,
    pub dest_port: u16,
}

impl SocksRequest {
    pub async fn handle_request<R, W>(
        reader: &mut BufReader<R>,
        writer: &mut BufWriter<W>,
        client_addr: SocketAddr,
        tcp_nodelay: bool,
    ) -> io::Result<()>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        debug!("Handling request from {}", client_addr);

        let client_request = SocksRequest::parse_request(reader, writer).await?;
        debug!(
            "Parsed client request from {}: {:?}",
            client_addr, client_request
        );

        let command: Command = match Command::parse_command(client_request.command) {
            Some(cmd) => cmd,
            None => {
                debug!(
                    "Invalid command {} from client {}",
                    client_request.command, client_addr
                );
                if let Err(e) = send_error_reply(writer, Reply::COMMAND_NOT_SUPPORTED).await {
                    debug!("Failed to send error reply to {}: {}", client_addr, e);
                    return Err(e);
                }
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Unsupported SOCKS command",
                ));
            }
        };

        let result = command
            .execute(client_request, client_addr, reader, writer, tcp_nodelay)
            .await?;
        debug!("Command execution result for {}: {:?}", client_addr, result);

        Ok(())
    }

    // Public for testing, should find a better way
    pub async fn parse_request<R, W>(
        reader: &mut BufReader<R>,
        writer: &mut BufWriter<W>,
    ) -> io::Result<SocksRequest>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let version = SocksRequest::read_u8_with_err(reader, "Failed to read version").await?;

        let command = SocksRequest::read_u8_with_err(reader, "Failed to read command").await?;

        let reserved =
            SocksRequest::read_u8_with_err(reader, "Failed to read reserved byte").await?;

        let address_type =
            SocksRequest::read_u8_with_err(reader, "Failed to read address type").await?;

        let dest_addr = match AddressType::parse(reader, address_type).await {
            Ok(addr) => addr,
            Err(socks_error) => {
                error!("Failed to parse address: {:?}", socks_error);
                if let Err(write_err) = send_socks_error_reply(writer, &socks_error).await {
                    debug!("Failed to send address parsing error reply: {}", write_err);
                }
                return Err(socks_error.to_io_error());
            }
        };

        let dest_port = reader.read_u16().await.map_err(|e| {
            let err = io::Error::new(io::ErrorKind::UnexpectedEof, "Failed to read port");
            error!("Failed to read port: {}", e);
            err
        })?;

        if version != SOCKS5_VERSION {
            error!(
                "Invalid SOCKS version: expected {}, got {}",
                SOCKS5_VERSION, version
            );
            let socks_error = SocksError::InvalidVersion(version);
            if let Err(write_err) = send_socks_error_reply(writer, &socks_error).await {
                debug!("Failed to send version error reply: {}", write_err);
            }
            return Err(socks_error.to_io_error());
        }

        if reserved != RESERVED {
            error!(
                "Invalid reserved byte: expected {}, got {}",
                RESERVED, reserved
            );
            let socks_error = SocksError::InvalidReservedByte(reserved);
            if let Err(write_err) = send_socks_error_reply(writer, &socks_error).await {
                debug!("Failed to send reserved byte error reply: {}", write_err);
            }
            return Err(socks_error.to_io_error());
        }

        Ok(SocksRequest {
            version,
            command,
            reserved,
            address_type,
            dest_addr,
            dest_port,
        })
    }

    async fn read_u8_with_err<R>(reader: &mut BufReader<R>, err_msg: &str) -> io::Result<u8>
    where
        R: AsyncRead + Unpin,
    {
        reader
            .read_u8()
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::UnexpectedEof, err_msg))
    }
}

#[cfg(test)]
mod tests {
    use crate::connection::{
        AddressType, RESERVED, SOCKS5_VERSION, command::Command, reply::Reply,
        request::SocksRequest, send_reply,
    };

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
        let mut writer = BufWriter::new(client);
        let request = SocksRequest::parse_request(&mut reader, &mut writer)
            .await
            .expect("Should parse valid request");
        assert_eq!(request.version, SOCKS5_VERSION);
        assert_eq!(request.command, Command::CONNECT);
        assert_eq!(request.address_type, AddressType::IPV4);
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
        let mut writer = BufWriter::new(client);
        let result = SocksRequest::parse_request(&mut reader, &mut writer).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn test_send_reply_success_ipv6() {
        let (server, mut client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(server);

        let addr_bytes = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1).octets().to_vec();
        send_reply(
            &mut writer,
            Reply::SUCCESS,
            AddressType::IPV6,
            &addr_bytes,
            8080,
        )
        .await
        .expect("Should send reply");
        writer.flush().await.unwrap();

        let mut response = vec![0u8; 4 + 16 + 2];
        client.read_exact(&mut response).await.unwrap();
        assert_eq!(response[0], SOCKS5_VERSION);
        assert_eq!(response[1], Reply::SUCCESS);
        assert_eq!(response[3], AddressType::IPV6);
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
        let mut writer = BufWriter::new(client);
        let request = SocksRequest::parse_request(&mut reader, &mut writer)
            .await
            .expect("Should parse IPv6 request");
        assert_eq!(request.version, SOCKS5_VERSION);
        assert_eq!(request.command, Command::CONNECT);
        assert_eq!(request.address_type, AddressType::IPV6);
        assert_eq!(
            request.dest_addr,
            std::net::IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1))
        );
        assert_eq!(request.dest_port, 443);
    }

    #[tokio::test]
    async fn test_parse_request_domain_name_valid() {
        let (mut client, server) = tokio::io::duplex(1024);
        let domain = b"google.com";
        let mut data = vec![0x05, 0x01, 0x00, 0x03]; // ATYP_DOMAIN
        data.push(domain.len() as u8); // Domain length
        data.extend_from_slice(domain);
        data.extend_from_slice(&80u16.to_be_bytes()); // Port 80

        client.write_all(&data).await.unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let mut writer = BufWriter::new(client);
        let result = SocksRequest::parse_request(&mut reader, &mut writer).await;
        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(request.version, SOCKS5_VERSION);
        assert_eq!(request.command, Command::CONNECT);
        assert!(!request.dest_addr.is_unspecified());
        assert_eq!(request.dest_port, 80);
    }

    #[tokio::test]
    async fn test_parse_request_domain_empty() {
        let (mut client, server) = tokio::io::duplex(1024);
        client
            .write_all(&[
                0x05, 0x01, 0x00, 0x03, // ATYP_DOMAIN
                0x00, // Domain length (0 - invalid)
                0x00, 0x50, // Port 80
            ])
            .await
            .unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let mut writer = BufWriter::new(client);
        let result = SocksRequest::parse_request(&mut reader, &mut writer).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Empty domain name"));
    }

    #[tokio::test]
    async fn test_parse_request_domain_invalid_utf8() {
        let (mut client, server) = tokio::io::duplex(1024);
        client
            .write_all(&[
                0x05, 0x01, 0x00, 0x03, // ATYP_DOMAIN
                0x04, // Domain length (4)
                0xFF, 0xFE, 0xFD, 0xFC, // Invalid UTF-8
                0x00, 0x50, // Port 80
            ])
            .await
            .unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let (_, dummy_client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(dummy_client);
        let result = SocksRequest::parse_request(&mut reader, &mut writer).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Invalid domain name encoding"));
    }

    #[tokio::test]
    async fn test_parse_request_domain_truncated() {
        let (mut client, server) = tokio::io::duplex(1024);
        client
            .write_all(&[
                0x05, 0x01, 0x00, 0x03, // ATYP_DOMAIN
                0x10, // Domain length (16) but we only provide 4 bytes
                b'e', b'x', b'a', b'm',
            ])
            .await
            .unwrap();
        client.flush().await.unwrap();
        drop(client);

        let mut reader = BufReader::new(server);
        let (_, dummy_client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(dummy_client);
        let result = SocksRequest::parse_request(&mut reader, &mut writer).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
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
        let (_, dummy_client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(dummy_client);
        let request = SocksRequest::parse_request(&mut reader, &mut writer)
            .await
            .expect("Should parse BIND request");
        assert_eq!(request.command, Command::BIND);
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
        let (_, dummy_client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(dummy_client);
        let request = SocksRequest::parse_request(&mut reader, &mut writer)
            .await
            .expect("Should parse UDP_ASSOCIATE request");
        assert_eq!(request.command, Command::UDP_ASSOCIATE);
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
        let (_, dummy_client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(dummy_client);
        let request = SocksRequest::parse_request(&mut reader, &mut writer)
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
        let (_, dummy_client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(dummy_client);
        let result = SocksRequest::parse_request(&mut reader, &mut writer).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Invalid reserved byte"));
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
        let (_, dummy_client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(dummy_client);
        let result = SocksRequest::parse_request(&mut reader, &mut writer).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Invalid SOCKS version: 4"));
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
        let (_, dummy_client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(dummy_client);
        let request = SocksRequest::parse_request(&mut reader, &mut writer)
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
        let (_, dummy_client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(dummy_client);
        let request = SocksRequest::parse_request(&mut reader, &mut writer)
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
        let (_, dummy_client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(dummy_client);
        let result = SocksRequest::parse_request(&mut reader, &mut writer).await;
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
        let (_, dummy_client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(dummy_client);
        let result = SocksRequest::parse_request(&mut reader, &mut writer).await;
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
        let (_, dummy_client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(dummy_client);
        let result = SocksRequest::parse_request(&mut reader, &mut writer).await;
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
        let (_, dummy_client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(dummy_client);
        let result = SocksRequest::parse_request(&mut reader, &mut writer).await;
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
        let (_, dummy_client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(dummy_client);
        let result = SocksRequest::parse_request(&mut reader, &mut writer).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    }

    #[tokio::test]
    async fn test_send_reply_success_ipv4() {
        let (server, mut client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(server);

        let addr_bytes = Ipv4Addr::new(192, 168, 1, 1).octets().to_vec();
        send_reply(
            &mut writer,
            Reply::SUCCESS,
            AddressType::IPV4,
            &addr_bytes,
            3128,
        )
        .await
        .expect("Should send IPv4 reply");
        writer.flush().await.unwrap();

        let mut response = vec![0u8; 4 + 4 + 2];
        client.read_exact(&mut response).await.unwrap();
        assert_eq!(response[0], SOCKS5_VERSION);
        assert_eq!(response[1], Reply::SUCCESS);
        assert_eq!(response[2], RESERVED);
        assert_eq!(response[3], AddressType::IPV4);
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
            send_reply(&mut writer, error_code, AddressType::IPV4, &addr_bytes, 0)
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
