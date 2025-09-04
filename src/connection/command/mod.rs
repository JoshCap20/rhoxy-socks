use std::{io, net::SocketAddr};

use tokio::io::{AsyncRead, AsyncWrite, BufReader, BufWriter};

use crate::connection::request::SocksRequest;

pub mod connect;

pub enum Command {
    CONNECT = 0x01,
    BIND = 0x02,
    UDP_ASSOCIATE = 0x03,
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
            Command::CONNECT => {
                connect::handle_connect_command(
                    client_request,
                    client_addr,
                    client_reader,
                    client_writer,
                )
                .await?;
            }
            Command::BIND => {
                // Handle bind command
            }
            Command::UDP_ASSOCIATE => {
                // Handle UDP associate command
            }
        }
        Ok(())
    }
}
