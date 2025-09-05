use std::{io, net::SocketAddr};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, BufReader, BufWriter};
use tracing::{debug, error};

use crate::connection::{
    AddressType, RESERVED, SOCKS5_VERSION, SocksError, command::Command,
    handler::handle_data_transfer, reply::Reply, send_error_reply, send_socks_error_reply,
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

        Self::handle_client_request(client_request, client_addr, reader, writer, tcp_nodelay)
            .await?;

        Ok(())
    }

    async fn handle_client_request<R, W>(
        client_request: SocksRequest,
        client_addr: SocketAddr,
        reader: &mut BufReader<R>,
        writer: &mut BufWriter<W>,
        tcp_nodelay: bool,
    ) -> io::Result<()>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
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
            .execute(client_request, client_addr, reader, writer)
            .await?;

        result.send_reply(writer).await?;

        // If successful and has a stream (CONNECT command), handle data transfer
        if result.is_success() && result.stream.is_some() {
            let stream = result.stream.unwrap();
            handle_data_transfer(reader, writer, stream, tcp_nodelay).await?;
        }

        Ok(())
    }

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
