pub mod bind;
pub mod connect;
pub mod udp_associate;

use std::{io, net::SocketAddr};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};

use crate::connection::{RESERVED, SOCKS5_VERSION, SocksRequest};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    Connect = 0x01,
    Bind = 0x02,
    UdpAssociate = 0x03,
}

impl Command {
    pub async fn execute<R, W>(
        &self,
        client_request: SocksRequest,
        client_addr: SocketAddr,
        client_reader: &mut BufReader<R>,
        client_writer: &mut BufWriter<W>,
    ) -> io::Result<()>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        match self {
            Command::Connect => {
                connect::handle_command(client_request, client_addr, client_reader, client_writer)
                    .await?;
            }
            Command::Bind => {
                bind::handle_command(client_request, client_addr, client_reader, client_writer)
                    .await?;
            }
            Command::UdpAssociate => {
                udp_associate::handle_command(
                    client_request,
                    client_addr,
                    client_reader,
                    client_writer,
                )
                .await?;
            }
        }
        Ok(())
    }

    pub fn parse_command(command: u8) -> Option<Command> {
        match command {
            0x01 => Some(Command::Connect),
            0x02 => Some(Command::Bind),
            0x03 => Some(Command::UdpAssociate),
            _ => None,
        }
    }

    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    pub fn name(&self) -> &'static str {
        match self {
            Command::Connect => "CONNECT",
            Command::Bind => "BIND",
            Command::UdpAssociate => "UDP_ASSOCIATE",
        }
    }
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
