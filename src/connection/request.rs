use std::{io, net::SocketAddr};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
};
use tracing::{debug, error};

pub async fn handle_request(
    mut reader: BufReader<OwnedReadHalf>,
    mut writer: BufWriter<OwnedWriteHalf>,
    client_addr: SocketAddr,
) -> io::Result<()> {
///    The SOCKS request is formed as follows:
///        +----+-----+-------+------+----------+----------+
///        |VER | CMD |  RSV  | ATYP | DST.ADDR | DST.PORT |
///        +----+-----+-------+------+----------+----------+
///        | 1  |  1  | X'00' |  1   | Variable |    2     |
///        +----+-----+-------+------+----------+----------+
///
///     Where:
///          o  VER    protocol version: X'05'
///          o  CMD
///             o  CONNECT X'01'
///             o  BIND X'02'
///             o  UDP ASSOCIATE X'03'
///          o  RSV    RESERVED
///          o  ATYP   address type of following address
///             o  IP V4 address: X'01'
///             o  DOMAINNAME: X'03'
///             o  IP V6 address: X'04'
///          o  DST.ADDR       desired destination address
///          o  DST.PORT desired destination port in network octet
///             order
    debug!("Handling request from {}", client_addr);

    let version = reader.read_u8().await?;
    let command = reader.read_u8().await?;
    let reserved = reader.read_u8().await?;
    let address_type = reader.read_u8().await?;

    debug!(
        "Client {} sent request: version={}, command={}, reserved={}, address_type={}",
        client_addr, version, command, reserved, address_type
    );

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
    Ok(())
}