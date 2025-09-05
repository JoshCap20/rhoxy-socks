pub mod command;
pub mod handler;
pub mod handshake;
pub mod request;
pub mod error;

use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};

pub const SOCKS5_VERSION: u8 = 0x05;
pub const RESERVED: u8 = 0x00;
// Since socks5 still requires dest.addr and port lets use 0.0.0.0:0 for now
// may want to set when error occurs in command though/post established connection
pub const ERROR_ADDR: [u8; 4] = [0, 0, 0, 0];
pub const ERROR_PORT: u16 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Method {
    NoAuthenticationRequired = 0x00,
    Gssapi = 0x01,
    UsernamePassword = 0x02,
    IanaAssigned = 0x03,
    ReservedForPrivateMethods = 0x80,
    NoAcceptableMethods = 0xFF,
}

impl Method {
    pub const NO_AUTHENTICATION_REQUIRED: u8 = Self::NoAuthenticationRequired as u8;
    pub const GSSAPI: u8 = Self::Gssapi as u8;
    pub const USERNAME_PASSWORD: u8 = Self::UsernamePassword as u8;
    pub const IANA_ASSIGNED: u8 = Self::IanaAssigned as u8;
    pub const RESERVED_FOR_PRIVATE_METHODS: u8 = Self::ReservedForPrivateMethods as u8;
    pub const NO_ACCEPTABLE_METHODS: u8 = Self::NoAcceptableMethods as u8;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Reply {
    Success = 0x00,
    GeneralFailure = 0x01,
    ConnectionNotAllowed = 0x02,
    NetworkUnreachable = 0x03,
    HostUnreachable = 0x04,
    ConnectionRefused = 0x05,
    TtlExpired = 0x06,
    CommandNotSupported = 0x07,
    AddressTypeNotSupported = 0x08,
}

impl Reply {
    pub const SUCCESS: u8 = Self::Success as u8;
    pub const GENERAL_FAILURE: u8 = Self::GeneralFailure as u8;
    pub const CONNECTION_NOT_ALLOWED: u8 = Self::ConnectionNotAllowed as u8;
    pub const NETWORK_UNREACHABLE: u8 = Self::NetworkUnreachable as u8;
    pub const HOST_UNREACHABLE: u8 = Self::HostUnreachable as u8;
    pub const CONNECTION_REFUSED: u8 = Self::ConnectionRefused as u8;
    pub const TTL_EXPIRED: u8 = Self::TtlExpired as u8;
    pub const COMMAND_NOT_SUPPORTED: u8 = Self::CommandNotSupported as u8;
    pub const ADDRESS_TYPE_NOT_SUPPORTED: u8 = Self::AddressTypeNotSupported as u8;
}

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
            .get(0)
            .ok_or(SocksError::NoAddressesResolved)?
            .ip();

        Ok(addr)
    }
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
