use std::io;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tracing::debug;

/// Should be organized into these steps:
/// 1. Handle handshake/authentication negotation
/// 2. Handle client request (command + destination addr)
/// 2.1 Handle connect request
/// 2.2 Handle bind request
/// 2.3 Handle UDP associate request

pub async fn handle_connection(stream: TcpStream, client_addr: SocketAddr) -> io::Result<()> {
    // TODO: implement connection handling
    let (reader, writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    // 1. handle initial request
    handle_handshake(reader, writer, client_addr).await?;

    Ok(())
}

async fn handle_handshake(
    mut reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    mut writer: BufWriter<tokio::net::tcp::OwnedWriteHalf>,
    client_addr: SocketAddr,
) -> io::Result<()> {
    // TODO: implement handshake handling

    /// Where a given octet must take on a specific value, the
   /// syntax X'hh' is used to denote the value of the single octet in that
   /// field. When the word 'Variable' is used, it indicates that the
   /// corresponding field has a variable length defined either by an
   /// associated (one or two octet) length field, or by a data type field.
   ///
   /// The client connects to the server, and sends a version
   /// identifier/method selection message:

    ///               +----+----------+----------+
    ///               |VER | NMETHODS | METHODS  |
    ///               +----+----------+----------+
    ///               | 1  |    1     | 1 to 255 |
    ///               +----+----------+----------+

   /// The VER field is set to X'05' for this version of the protocol.  The
   /// NMETHODS field contains the number of method identifier octets that
   /// appear in the METHODS field.

   /// The server selects from one of the methods given in METHODS, and
   /// sends a METHOD selection message:

    ///                     +----+--------+
    ///                     |VER | METHOD |
    ///                     +----+--------+
    ///                     | 1  |   1    |
    ///                     +----+--------+

   /// If the selected METHOD is X'FF', none of the methods listed by the
   /// client are acceptable, and the client MUST close the connection.

   /// The values currently defined for METHOD are:

   ///       o  X'00' NO AUTHENTICATION REQUIRED
   ///       o  X'01' GSSAPI
   ///       o  X'02' USERNAME/PASSWORD
   ///       o  X'03' to X'7F' IANA ASSIGNED
   ///       o  X'80' to X'FE' RESERVED FOR PRIVATE METHODS
   ///       o  X'FF' NO ACCEPTABLE METHODS

   /// The client and server then enter a method-specific sub-negotiation.
    debug!("Handling handshake for client {}", client_addr);

    let version = reader.read_u8().await?;
    let nmethods = reader.read_u8().await?;
    debug!("Client {} is using SOCKS version {} with {} methods", client_addr, version, nmethods);

    Ok(())
}
