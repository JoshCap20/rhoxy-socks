pub mod command;
pub mod handler;
pub mod handshake;
pub mod request;

use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, BufReader};
use tracing::error;

pub const SOCKS5_VERSION: u8 = 0x05;
pub const REPLY_SUCCESS: u8 = 0x00;
pub const RESERVED: u8 = 0x00;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AddressType {
    IPv4 = 0x01,
    DomainName = 0x03,
    IPv6 = 0x04,
}

impl AddressType {
    pub const IPV4: u8 = Self::IPv4 as u8;
    pub const DOMAIN_NAME: u8 = Self::DomainName as u8;
    pub const IPV6: u8 = Self::IPv6 as u8;

    pub fn from_u8(value: u8) -> Option<AddressType> {
        match value {
            Self::IPV4 => Some(AddressType::IPv4),
            Self::DOMAIN_NAME => Some(AddressType::DomainName),
            Self::IPV6 => Some(AddressType::IPv6),
            _ => None,
        }
    }

    pub async fn parse<R>(reader: &mut BufReader<R>, atyp: u8) -> io::Result<std::net::IpAddr>
    where
        R: AsyncRead + Unpin,
    {
        match AddressType::from_u8(atyp) {
            Some(AddressType::IPv4) => Self::parse_ipv4(reader).await,
            Some(AddressType::DomainName) => Self::parse_domain_name(reader).await,
            Some(AddressType::IPv6) => Self::parse_ipv6(reader).await,
            None => {
                error!("Unsupported address type: {}", atyp);
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Unsupported address type",
                ))
            }
        }
    }

    async fn parse_ipv4<R>(reader: &mut BufReader<R>) -> io::Result<std::net::IpAddr>
    where
        R: AsyncRead + Unpin,
    {
        let mut addr = [0u8; 4];
        reader.read_exact(&mut addr).await?;
        Ok(std::net::IpAddr::from(addr))
    }

    async fn parse_ipv6<R>(reader: &mut BufReader<R>) -> io::Result<std::net::IpAddr>
    where
        R: AsyncRead + Unpin,
    {
        let mut addr = [0u8; 16];
        reader.read_exact(&mut addr).await?;
        Ok(std::net::IpAddr::from(addr))
    }

    async fn parse_domain_name<R>(reader: &mut BufReader<R>) -> io::Result<std::net::IpAddr>
    where
        R: AsyncRead + Unpin,
    {
        let domain_len = reader.read_u8().await? as usize;
        if domain_len == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Empty domain name",
            ));
        }

        let mut domain = vec![0u8; domain_len];
        reader.read_exact(&mut domain).await?;

        let domain_str = String::from_utf8(domain).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "Invalid domain name encoding")
        })?;

        let resolved_addrs = resolve_domain(&domain_str).await.map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("DNS resolution failed for {}: {}", domain_str, e),
            )
        })?;

        let addr = resolved_addrs
            .get(0)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "No addresses resolved for domain",
                )
            })?
            .ip();

        Ok(addr)
    }
}

async fn resolve_domain(domain: &str) -> io::Result<Vec<std::net::SocketAddr>> {
    let addrs: Vec<_> = tokio::net::lookup_host((domain, 0)).await?.collect();
    Ok(addrs)
}
