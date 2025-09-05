pub mod address_type;
pub mod command;
pub mod error;
pub mod handler;
pub mod handshake;
pub mod method;
pub mod reply;
pub mod request;

use std::io;
use tokio::io::{AsyncWrite, AsyncWriteExt, BufWriter};

use crate::connection::{address_type::AddressType, error::SocksError};

pub const SOCKS5_VERSION: u8 = 0x05;
pub const RESERVED: u8 = 0x00;
// For errors prior to established connection (in which case command returns the host, port)
// these are used for connection errors (i.e. dns failure in domain name translation)
pub const ERROR_ADDR: [u8; 4] = [0, 0, 0, 0];
pub const ERROR_PORT: u16 = 0;

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
    // TODO: Single call with stack-allocated buffer
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
    // CommandResult error reply does not use this
    // but socks request parsing failures do
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
