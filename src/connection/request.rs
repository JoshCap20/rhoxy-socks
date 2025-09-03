use std::{io, net::SocketAddr};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
};
use tracing::{debug, error};

#[derive(Debug)]
pub struct SocksRequest {
    pub version: u8,
    pub command: u8,
    pub reserved: u8,
    pub address_type: u8,
    pub dest_addr: std::net::IpAddr,
    pub dest_port: u16,
}

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
    let command = reader.read_u8().await?;
    let reserved = reader.read_u8().await?;
    let address_type = reader.read_u8().await?;

    let dest_addr = match address_type {
        1 => {
            // IPv4
            let mut addr = [0u8; 4];
            reader.read_exact(&mut addr).await?;
            std::net::IpAddr::from(addr)
        }
        3 => {
            error!("Domain name address type not yet supported");
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Domain name resolution not implemented",
            ));
        }
        4 => {
            // IPv6
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
        1 => {
            debug!("Handling CONNECT request");
        }
        2 => {
            debug!("Handling BIND request");
        }
        3 => {
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
