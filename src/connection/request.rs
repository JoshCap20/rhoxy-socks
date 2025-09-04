use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, BufReader};
use tracing::error;

use crate::connection::{AddressType, RESERVED, SOCKS5_VERSION};

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
    pub async fn parse_request<R>(reader: &mut BufReader<R>) -> io::Result<SocksRequest>
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
        let dest_addr = AddressType::parse(reader, address_type).await?;
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
}
