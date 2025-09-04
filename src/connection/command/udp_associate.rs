use std::{io, net::SocketAddr};
use tokio::io::{AsyncRead, AsyncWrite, BufReader, BufWriter};
use tracing::{debug, error};

use crate::connection::SocksRequest;

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

    error!("[{client_addr}] UDP ASSOCIATE command is not supported");
    return Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "UDP ASSOCIATE request handling not implemented",
    ));
}
