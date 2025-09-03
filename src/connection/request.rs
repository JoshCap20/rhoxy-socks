use std::{io, net::SocketAddr};

use tokio::join;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter, copy},
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
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

const RESERVED: u8 = 0x00;

const CONNECT: u8 = 0x01;
const BIND: u8 = 0x02;
const UDP_ASSOCIATE: u8 = 0x03;
const ATYP_IPV4: u8 = 0x01;
const ATYP_DOMAIN: u8 = 0x03;
const ATYP_IPV6: u8 = 0x04;

const REPLY_SUCCESS: u8 = 0x00;

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

    handle_client_request(client_request, reader, writer).await?;

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
    if reserved != RESERVED {
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
    reader: &mut BufReader<OwnedReadHalf>,
    writer: &mut BufWriter<OwnedWriteHalf>,
) -> io::Result<()> {
    match client_request.command {
        CONNECT => {
            handle_connect_command(client_request, reader, writer).await?;
            Ok(())
        }
        BIND => {
            debug!("Handling BIND request");
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "BIND request handling not implemented",
            ))
        }
        UDP_ASSOCIATE => {
            debug!("Handling UDP ASSOCIATE request");
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "UDP ASSOCIATE request handling not implemented",
            ))
        }
        _ => {
            error!("Unsupported command: {}", client_request.command);
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unsupported command",
            ));
        }
    }
}

async fn handle_connect_command(
    client_request: SocksRequest,
    client_reader: &mut BufReader<OwnedReadHalf>,
    client_writer: &mut BufWriter<OwnedWriteHalf>,
) -> io::Result<()> {
    debug!("Handling CONNECT command");

    let target_stream =
        TcpStream::connect((client_request.dest_addr, client_request.dest_port)).await?;
    debug!("Connected to target {}", client_request.dest_addr);

    let destination_addr = target_stream.local_addr()?;
    let destination_port = destination_addr.port();
    let destination_addr_type = if destination_addr.is_ipv4() {
        ATYP_IPV4
    } else {
        ATYP_IPV6
    };
    let destination_addr_as_bytes = match destination_addr.ip() {
        std::net::IpAddr::V4(addr) => addr.octets().to_vec(),
        std::net::IpAddr::V6(addr) => addr.octets().to_vec(),
    };
    debug!(
        "Connected to destination {}:{} address type {}",
        destination_addr, destination_port, destination_addr_type
    );

    send_reply(
        client_writer,
        REPLY_SUCCESS,
        destination_addr_type,
        &destination_addr_as_bytes,
        destination_port,
    )
    .await?;

    let (mut target_reader, mut target_writer) = target_stream.into_split();
    let (client_to_target, target_to_client) = join!(
        copy(&mut *client_reader, &mut target_writer),
        copy(&mut target_reader, &mut *client_writer)
    );

    client_to_target?;
    target_to_client?;
    Ok(())
}

async fn send_reply(
    writer: &mut BufWriter<OwnedWriteHalf>,
    reply_code: u8,
    addr_type: u8,
    addr_bytes: &[u8],
    port: u16,
) -> io::Result<()> {
    writer.write_u8(SOCKS5_VERSION).await?;
    writer.write_u8(reply_code).await?;
    writer.write_u8(RESERVED).await?;
    writer.write_u8(addr_type).await?;
    writer.write_all(addr_bytes).await?;
    writer.write_u16(port).await?;
    writer.flush().await?;
    Ok(())
}
