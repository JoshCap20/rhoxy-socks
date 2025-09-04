use std::{io, net::SocketAddr};

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::join;
use tokio::{
    io::{AsyncWriteExt, BufReader, BufWriter, copy},
    net::TcpStream,
};
use tracing::{debug};

use crate::connection::request::SocksRequest;
use crate::connection::{ATYP_IPV4, ATYP_IPV6};

pub async fn handle_connect_command<R, W>(
    client_request: SocksRequest,
    client_addr: SocketAddr,
    client_reader: &mut BufReader<R>,
    client_writer: &mut BufWriter<W>,
) -> io::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    debug!(
        "[{client_addr}] Handling CONNECT request: {:?}",
        client_request
    );

    let target_stream =
        TcpStream::connect((client_request.dest_addr, client_request.dest_port)).await?;
    debug!(
        "[{client_addr}] Connected to target {}:{}",
        client_request.dest_addr, client_request.dest_port
    );

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

async fn send_reply<W>(
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