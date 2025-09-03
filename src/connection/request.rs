use std::{io, net::SocketAddr};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
};
use tracing::{debug, error};

use crate::connection::SOCKS5_VERSION;

#[derive(Debug)]
pub struct SocksRequest {
    pub version: u8,
    pub command: u8,
    pub reserved: u8,
    pub address_type: u8,
    pub dest_addr: std::net::IpAddr,
    pub dest_port: u16,
}

const CONNECT: u8 = 0x01;
const BIND: u8 = 0x02;
const UDP_ASSOCIATE: u8 = 0x03;
const ATYP_IPV4: u8 = 0x01;
const ATYP_DOMAIN: u8 = 0x03;
const ATYP_IPV6: u8 = 0x04;

pub async fn handle_request(
    reader: &mut BufReader<OwnedReadHalf>,
    writer: &mut BufWriter<OwnedWriteHalf>,
    client_addr: SocketAddr,
) -> io::Result<()> {
    debug!("Handling request from {}", client_addr);

    let client_request = parse_request(reader).await?;
    debug!(
        "Parsed client request from {}: {:?}",
        client_addr, client_request
    );

    handle_client_request(client_request, writer).await?;

    Ok(())
}

async fn parse_request(reader: &mut BufReader<OwnedReadHalf>) -> io::Result<SocksRequest> {
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
    if reserved != 0x00 {
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

async fn handle_client_request(
    client_request: SocksRequest,
    writer: &mut BufWriter<OwnedWriteHalf>,
) -> io::Result<()> {
    match client_request.command {
        CONNECT => {
            debug!("Handling CONNECT request");
        }
        BIND => {
            debug!("Handling BIND request");
        }
        UDP_ASSOCIATE => {
            debug!("Handling UDP ASSOCIATE request");
        }
        _ => {
            error!("Unsupported command: {}", client_request.command);
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unsupported command",
            ));
        }
    }

    Ok(())
}
