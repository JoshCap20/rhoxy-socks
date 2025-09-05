use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, BufReader, BufWriter};
use tracing::error;

use crate::connection::{
    AddressType, RESERVED, SOCKS5_VERSION, SocksError, send_socks_error_reply,
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
    pub async fn parse_request<R, W>(
        reader: &mut BufReader<R>,
        writer: &mut BufWriter<W>,
    ) -> io::Result<SocksRequest>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let version = reader
            .read_u8()
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::UnexpectedEof, "Failed to read version"))?;

        let command = reader
            .read_u8()
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::UnexpectedEof, "Failed to read command"))?;

        let reserved = reader.read_u8().await.map_err(|e| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "Failed to read reserved byte")
        })?;

        let address_type = reader.read_u8().await.map_err(|e| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "Failed to read address type")
        })?;

        let dest_addr = match AddressType::parse(reader, address_type).await {
            Ok(addr) => addr,
            Err(socks_error) => {
                error!("Failed to parse address: {:?}", socks_error);
                let _ = send_socks_error_reply(writer, &socks_error).await;
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
            let _ = send_socks_error_reply(writer, &socks_error).await;
            return Err(socks_error.to_io_error());
        }

        if reserved != RESERVED {
            error!(
                "Invalid reserved byte: expected {}, got {}",
                RESERVED, reserved
            );
            let socks_error = SocksError::InvalidReservedByte(reserved);
            let _ = send_socks_error_reply(writer, &socks_error).await;
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
}
