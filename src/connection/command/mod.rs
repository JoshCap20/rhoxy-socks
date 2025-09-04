use std::{io, net::SocketAddr};

use crate::connection::request::SocksRequest;
use tokio::io::{AsyncRead, AsyncWrite, BufReader, BufWriter};
use tracing::error;

pub mod connect;

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
                connect::handle_connect_command(
                    client_request,
                    client_addr,
                    client_reader,
                    client_writer,
                )
                .await?;
            }
            Command::Bind => {
                error!("[{client_addr}] BIND command is not supported");
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "BIND request handling not implemented",
                ));
            }
            Command::UdpAssociate => {
                error!("[{client_addr}] UDP_ASSOCIATE command is not supported");
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "UDP ASSOCIATE request handling not implemented",
                ));
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
