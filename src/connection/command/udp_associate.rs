use std::{io, net::SocketAddr};
use tokio::io::{AsyncRead, AsyncWrite, BufReader, BufWriter};
use tracing::{debug, error};

use crate::connection::{
    AddressType, ERROR_ADDR, ERROR_PORT, Reply, request::SocksRequest, send_reply,
};

pub async fn handle_command<R, W>(
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
        "[{client_addr}] Handling UDP ASSOCIATE request: {:?}",
        client_request
    );

    send_reply(
        client_writer,
        Reply::COMMAND_NOT_SUPPORTED,
        AddressType::IPV4,
        &ERROR_ADDR,
        ERROR_PORT,
    )
    .await?;

    error!("[{client_addr}] UDP ASSOCIATE command is not supported");
    return Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "UDP ASSOCIATE request handling not implemented",
    ));
}
