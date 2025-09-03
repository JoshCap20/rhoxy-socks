use std::{io, net::SocketAddr};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
};
use tracing::debug;

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

    // TODO: Handle different address types and commands
    Ok(())
}