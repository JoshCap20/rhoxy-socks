use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, BufReader, BufWriter};
use tracing::error;

use crate::connection::{send_error_reply, map_error_to_reply, AddressType, Reply, RESERVED, SOCKS5_VERSION};

#[derive(Debug)]
pub struct SocksRequest {
    pub version: u8,
    pub command: u8,
    pub reserved: u8,
    pub address_type: u8,
    pub dest_addr: std::net::IpAddr,
    pub dest_port: u16,
}

impl SocksRequest {
    pub async fn parse_request<R, W>(reader: &mut BufReader<R>, writer: &mut BufWriter<W>) -> io::Result<SocksRequest>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        // Step 1: Parse the entire request first without sending any error responses
        let version = reader.read_u8().await.map_err(|e| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "Failed to read version")
        })?;

        let command = reader.read_u8().await.map_err(|e| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "Failed to read command")
        })?;

        let reserved = reader.read_u8().await.map_err(|e| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "Failed to read reserved byte")
        })?;

        let address_type = reader.read_u8().await.map_err(|e| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "Failed to read address type")
        })?;

        // Try to parse the address without sending error responses
        let dest_addr = match AddressType::parse(reader, writer, address_type).await {
            Ok(addr) => addr,
            Err(e) => {
                error!("Failed to parse address: {}", e);
                // Send appropriate error response based on the specific failure
                let error_code = map_error_to_reply(&e);
                // Only send error response if writer flush succeeds (connection is still open)
                let _ = send_error_reply(writer, error_code).await;
                return Err(e);
            }
        };

        let dest_port = reader.read_u16().await.map_err(|e| {
            let err = io::Error::new(io::ErrorKind::UnexpectedEof, "Failed to read port");
            error!("Failed to read port: {}", e);
            err
        })?;

        // Step 2: Validate the parsed request and send appropriate error responses
        if version != SOCKS5_VERSION {
            error!("Invalid SOCKS version: expected {}, got {}", SOCKS5_VERSION, version);
            let _ = send_error_reply(writer, Reply::GENERAL_FAILURE).await;
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Expected SOCKS version {}, got {}", SOCKS5_VERSION, version),
            ));
        }

        if reserved != RESERVED {
            error!("Invalid reserved byte: expected {}, got {}", RESERVED, reserved);
            let _ = send_error_reply(writer, Reply::GENERAL_FAILURE).await;
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Reserved byte must be {}, got {}", RESERVED, reserved),
            ));
        }

        // Step 3: Return successful parse result
        Ok(SocksRequest {
            version,
            command,
            reserved,
            address_type,
            dest_addr,
            dest_port,
        })
    }
}
