pub mod command;
pub mod handshake;
pub mod request;

use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, BufReader};
use tracing::{debug, error};

pub const SOCKS5_VERSION: u8 = 0x05;
pub const ATYP_IPV4: u8 = 0x01;
pub const ATYP_DOMAIN: u8 = 0x03;
pub const ATYP_IPV6: u8 = 0x04;

pub const REPLY_SUCCESS: u8 = 0x00;

pub const RESERVED: u8 = 0x00;

pub const CONNECT: u8 = 0x01;
pub const BIND: u8 = 0x02;
pub const UDP_ASSOCIATE: u8 = 0x03;

#[derive(Debug)]
pub struct SocksRequest {
    pub version: u8,
    pub command: u8,
    pub reserved: u8,
    pub address_type: u8,
    pub dest_addr: std::net::IpAddr,
    pub dest_port: u16,
}

async fn resolve_domain(domain: &str) -> io::Result<Vec<std::net::SocketAddr>> {
    let addrs: Vec<_> = tokio::net::lookup_host((domain, 0)).await?.collect();
    Ok(addrs)
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

    // TODO: Move this to an enum struct
    let dest_addr = match address_type {
        ATYP_IPV4 => {
            let mut addr = [0u8; 4];
            reader.read_exact(&mut addr).await?;
            std::net::IpAddr::from(addr)
        }
        ATYP_DOMAIN => {
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

            addr
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
