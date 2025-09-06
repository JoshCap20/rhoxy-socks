use std::{io, net::SocketAddr};
use tokio::io::{AsyncRead, AsyncWrite, BufReader, BufWriter};
use tracing::{debug, error};

use crate::connection::{command::CommandResult, reply::Reply, request::SocksRequest};

pub async fn handle_command<R, W>(
    client_request: SocksRequest,
    client_addr: SocketAddr,
    _client_reader: &mut BufReader<R>,
    _client_writer: &mut BufWriter<W>,
) -> io::Result<CommandResult>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    debug!(
        "[{client_addr}] Handling BIND request: {:?}",
        client_request
    );

    error!("[{client_addr}] BIND command is not supported");
    Ok(CommandResult::error(Reply::COMMAND_NOT_SUPPORTED))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::{AddressType, command::Command};
    use std::net::{IpAddr, Ipv4Addr};
    use tokio::io::BufReader;

    fn create_test_request() -> SocksRequest {
        SocksRequest {
            version: 0x05,
            command: Command::BIND as u8,
            reserved: 0x00,
            address_type: AddressType::IPV4,
            dest_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            dest_port: 8080,
        }
    }

    #[tokio::test]
    async fn test_bind_command_not_supported() {
        let request = create_test_request();
        let client_addr = "127.0.0.1:12345".parse().unwrap();

        let (client_read, _client_write) = tokio::io::duplex(1024);
        let mut reader = BufReader::new(client_read);
        let mut writer = tokio::io::BufWriter::new(tokio::io::sink());

        let result = handle_command(request, client_addr, &mut reader, &mut writer).await;

        assert!(result.is_ok());
        let command_result = result.unwrap();
        assert!(command_result.is_error());
        assert_eq!(command_result.reply_code(), Reply::COMMAND_NOT_SUPPORTED);
    }

    #[tokio::test]
    async fn test_bind_with_different_address_types() {
        let test_cases = [
            (AddressType::IPv4, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))),
            (AddressType::IPv6, IpAddr::V6("::1".parse().unwrap())),
        ];

        for (addr_type, dest_addr) in test_cases {
            let mut request = create_test_request();
            request.address_type = addr_type as u8;
            request.dest_addr = dest_addr;

            let client_addr = "127.0.0.1:12345".parse().unwrap();

            let (client_read, _client_write) = tokio::io::duplex(1024);
            let mut reader = BufReader::new(client_read);
            let mut writer = tokio::io::BufWriter::new(tokio::io::sink());

            let result = handle_command(request, client_addr, &mut reader, &mut writer).await;

            assert!(result.is_ok());
            let command_result = result.unwrap();
            assert!(command_result.is_error());
            assert_eq!(command_result.reply_code(), Reply::COMMAND_NOT_SUPPORTED);
        }
    }

    #[tokio::test]
    async fn test_bind_with_different_ports() {
        let test_ports = [80, 443, 8080, 1234, 65535];

        for port in test_ports {
            let mut request = create_test_request();
            request.dest_port = port;

            let client_addr = "127.0.0.1:12345".parse().unwrap();

            let (client_read, _client_write) = tokio::io::duplex(1024);
            let mut reader = BufReader::new(client_read);
            let mut writer = tokio::io::BufWriter::new(tokio::io::sink());

            let result = handle_command(request, client_addr, &mut reader, &mut writer).await;

            assert!(result.is_ok());
            let command_result = result.unwrap();
            assert!(command_result.is_error());
            assert_eq!(command_result.reply_code(), Reply::COMMAND_NOT_SUPPORTED);
        }
    }
}
