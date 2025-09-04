use std::{io, net::SocketAddr};

use tokio::io::{AsyncRead, AsyncWrite};

use tokio::io::{BufReader, BufWriter};
use tracing::{debug, error};

use crate::connection::request::SocksRequest;

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
        "[{client_addr}] Handling UDP_ASSOCIATE request: {:?}",
        client_request
    );

    error!("[{client_addr}] UDP_ASSOCIATE command is not supported");
    return Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "UDP_ASSOCIATE request handling not implemented",
    ));
}
