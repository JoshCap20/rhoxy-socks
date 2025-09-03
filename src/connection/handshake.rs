use std::{io, net::SocketAddr};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
};
use tracing::{debug, error};

use crate::connection::SOCKS5_VERSION;

#[derive(Debug)]
pub struct HandshakeRequest {
    pub version: u8,
    pub nmethods: u8,
    pub methods: Vec<u8>,
}

const NO_AUTHENTICATION_REQUIRED: u8 = 0x00;

pub async fn perform_handshake(
    reader: &mut BufReader<OwnedReadHalf>,
    writer: &mut BufWriter<OwnedWriteHalf>,
    client_addr: SocketAddr,
) -> io::Result<()> {
    debug!("Performing handshake for client {}", client_addr);

    let handshake_request = parse_client_greeting(reader).await?;
    debug!(
        "Parsed client greeting for {}: {:?}",
        client_addr, handshake_request
    );

    handle_client_greeting(&handshake_request, writer).await?;
    debug!("Completed handshake for client {}", client_addr);

    Ok(())
}

async fn parse_client_greeting(
    reader: &mut BufReader<OwnedReadHalf>,
) -> io::Result<HandshakeRequest> {
    let version = reader.read_u8().await?;
    if version != SOCKS5_VERSION {
        error!("Invalid SOCKS version: {}", version);
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Expected SOCKS version {}, got {}", SOCKS5_VERSION, version),
        ));
    }
    let nmethods = reader.read_u8().await?;

    let mut methods: Vec<u8> = vec![0; nmethods as usize];
    reader.read_exact(&mut methods).await?;

    Ok(HandshakeRequest {
        version,
        nmethods,
        methods,
    })
}

async fn handle_client_greeting(
    handshake_request: &HandshakeRequest,
    writer: &mut BufWriter<OwnedWriteHalf>,
) -> io::Result<()> {
    /// TODO: Support all authentication methods
    ///           o  X'01' GSSAPI
    ///           o  X'02' USERNAME/PASSWORD
    ///           o  X'03' to X'7F' IANA ASSIGNED
    ///           o  X'80' to X'FE' RESERVED FOR PRIVATE METHODS
    ///           o  X'FF' NO ACCEPTABLE METHODS
    let response = [SOCKS5_VERSION, NO_AUTHENTICATION_REQUIRED];
    writer.write_all(&response).await?;
    writer.flush().await?;

    Ok(())
}
