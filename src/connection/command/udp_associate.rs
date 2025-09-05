use std::{io, net::SocketAddr};
use tokio::io::{AsyncRead, AsyncWrite, BufReader, BufWriter};
use tracing::{debug, error};

use crate::connection::{Reply, command::CommandResult, request::SocksRequest};

pub async fn handle_command<R, W>(
    client_request: SocksRequest,
    client_addr: SocketAddr,
    _client_reader: &mut BufReader<R>,
    _client_writer: &mut BufWriter<W>,
) -> io::Result<CommandResult>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    debug!(
        "[{client_addr}] Handling UDP ASSOCIATE request: {:?}",
        client_request
    );

    error!("[{client_addr}] UDP ASSOCIATE command is not supported");
    Ok(CommandResult::error(Reply::COMMAND_NOT_SUPPORTED))
}
