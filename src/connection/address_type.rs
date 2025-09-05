use tokio::io::{AsyncRead, AsyncReadExt, BufReader};

use crate::connection::{error::SocksError, resolve_domain};

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

    pub async fn parse<R>(
        reader: &mut BufReader<R>,
        atyp: u8,
    ) -> Result<std::net::IpAddr, SocksError>
    where
        R: AsyncRead + Unpin,
    {
        match AddressType::from_u8(atyp) {
            Some(AddressType::IPv4) => Self::parse_ipv4(reader).await,
            Some(AddressType::DomainName) => Self::parse_domain_name(reader).await,
            Some(AddressType::IPv6) => Self::parse_ipv6(reader).await,
            None => Err(SocksError::UnsupportedAddressType(atyp)),
        }
    }

    async fn parse_ipv4<R>(reader: &mut BufReader<R>) -> Result<std::net::IpAddr, SocksError>
    where
        R: AsyncRead + Unpin,
    {
        let mut addr = [0u8; 4];
        reader
            .read_exact(&mut addr)
            .await
            .map_err(|e| SocksError::IoError(e.kind()))?;
        Ok(std::net::IpAddr::from(addr))
    }

    async fn parse_ipv6<R>(reader: &mut BufReader<R>) -> Result<std::net::IpAddr, SocksError>
    where
        R: AsyncRead + Unpin,
    {
        let mut addr = [0u8; 16];
        reader
            .read_exact(&mut addr)
            .await
            .map_err(|e| SocksError::IoError(e.kind()))?;
        Ok(std::net::IpAddr::from(addr))
    }

    async fn parse_domain_name<R>(reader: &mut BufReader<R>) -> Result<std::net::IpAddr, SocksError>
    where
        R: AsyncRead + Unpin,
    {
        let domain_len = reader
            .read_u8()
            .await
            .map_err(|e| SocksError::IoError(e.kind()))? as usize;
        if domain_len == 0 {
            return Err(SocksError::EmptyDomainName);
        }

        let mut domain = vec![0u8; domain_len];
        reader
            .read_exact(&mut domain)
            .await
            .map_err(|e| SocksError::IoError(e.kind()))?;

        let domain_str =
            String::from_utf8(domain).map_err(|_| SocksError::InvalidDomainNameEncoding)?;

        let resolved_addrs = resolve_domain(&domain_str)
            .await
            .map_err(|_| SocksError::DnsResolutionFailed)?;

        let addr = resolved_addrs
            .first()
            .ok_or(SocksError::NoAddressesResolved)?
            .ip();

        Ok(addr)
    }
}
