use std::{io, net::SocketAddr};

use tokio::{
    io::{AsyncReadExt, BufReader, BufWriter},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
};
use tracing::debug;

pub async fn perform_handshake(
    mut reader: BufReader<OwnedReadHalf>,
    mut writer: BufWriter<OwnedWriteHalf>,
    client_addr: SocketAddr,
) -> io::Result<()> {
    debug!("Performing handshake for client {}", client_addr);

    let version = reader.read_u8().await?;
    let nmethods = reader.read_u8().await?;
    debug!(
        "Client {} is using SOCKS version {} with {} methods",
        client_addr, version, nmethods
    );

    let mut methods: Vec<u8> = vec![0; nmethods as usize];
    reader.read_exact(&mut methods).await?;
    debug!(
        "Client {} supports the following authentication methods: {:?}",
        client_addr, methods
    );

    Ok(())
}
