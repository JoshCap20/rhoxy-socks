// High-performance SOCKS5 error handling optimizations
use std::{io, net::SocketAddr};
use tokio::{
    io::{AsyncRead, AsyncWrite, BufReader, BufWriter, copy},
    join,
    net::TcpStream,
};
use tracing::{debug, warn};

use crate::connection::{Reply, command::Command, request::SocksRequest, CommandResult};

/// Optimized CommandResult with zero-allocation reply sending
impl CommandResult {
    // Pre-compute address bytes to avoid hot-path allocations
    pub fn success_with_precomputed(bind_addr: std::net::IpAddr, bind_port: u16) -> Self {
        Self {
            reply_code: Reply::SUCCESS,
            bind_addr,
            bind_port,
            stream: None,
        }
    }

    /// High-performance reply sending with stack-allocated buffers
    pub async fn send_reply_optimized<W>(&self, writer: &mut BufWriter<W>) -> io::Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        use tokio::io::AsyncWriteExt;
        
        // Stack-allocated buffer for maximum performance
        let mut reply_buf = [0u8; 22]; // Max SOCKS5 reply size (4 + 16 + 2)
        let mut pos = 0;

        // Version, Reply, Reserved
        reply_buf[pos] = crate::connection::SOCKS5_VERSION;
        reply_buf[pos + 1] = self.reply_code;
        reply_buf[pos + 2] = crate::connection::RESERVED;
        pos += 3;

        // Address Type and Address
        match self.bind_addr {
            std::net::IpAddr::V4(ipv4) => {
                reply_buf[pos] = crate::connection::AddressType::IPV4;
                pos += 1;
                let octets = ipv4.octets();
                reply_buf[pos..pos + 4].copy_from_slice(&octets);
                pos += 4;
            }
            std::net::IpAddr::V6(ipv6) => {
                reply_buf[pos] = crate::connection::AddressType::IPV6;
                pos += 1;
                let octets = ipv6.octets();
                reply_buf[pos..pos + 16].copy_from_slice(&octets);
                pos += 16;
            }
        }

        // Port (big-endian)
        let port_bytes = self.bind_port.to_be_bytes();
        reply_buf[pos..pos + 2].copy_from_slice(&port_bytes);
        pos += 2;

        // Single write call for optimal performance
        writer.write_all(&reply_buf[..pos]).await?;
        writer.flush().await
    }
}

/// Fast-path error handling with minimal allocations
pub async fn handle_request_optimized<R, W>(
    reader: &mut BufReader<R>,
    writer: &mut BufWriter<W>,
    client_addr: SocketAddr,
) -> io::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    // Parse request (already optimized in your implementation)
    let client_request = match SocksRequest::parse_request(reader, writer).await {
        Ok(req) => req,
        Err(e) => {
            // Error already sent by parse_request, just return
            debug!("Request parsing failed for {}: {}", client_addr, e);
            return Err(e);
        }
    };

    // Fast command validation
    let command = match Command::parse_command(client_request.command) {
        Some(cmd) => cmd,
        None => {
            // Direct reply without string allocation
            let error_result = CommandResult::error(Reply::COMMAND_NOT_SUPPORTED);
            error_result.send_reply_optimized(writer).await?;
            
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unsupported command", // Static string, no allocation
            ));
        }
    };

    // Execute command
    let result = command
        .execute(client_request, client_addr, reader, writer)
        .await?;

    // Send reply using optimized method
    result.send_reply_optimized(writer).await?;

    // Handle data transfer for CONNECT
    if result.is_success() && result.stream.is_some() {
        let stream = result.stream.unwrap();
        handle_data_transfer_optimized(reader, writer, stream).await?;
    }

    Ok(())
}

/// Optimized data transfer with larger buffers and connection monitoring
pub async fn handle_data_transfer_optimized<R, W>(
    client_reader: &mut BufReader<R>,
    client_writer: &mut BufWriter<W>,
    target_stream: TcpStream,
) -> io::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    // Configure TCP no-delay for low latency
    if let Err(e) = target_stream.set_nodelay(true) {
        warn!("Failed to set TCP_NODELAY: {}", e);
    }

    let (mut target_reader, mut target_writer) = target_stream.into_split();
    
    // Use tokio::select! for better error handling and connection monitoring
    tokio::select! {
        result1 = copy(&mut *client_reader, &mut target_writer) => {
            if let Err(e) = result1 {
                debug!("Client to target copy failed: {}", e);
                return Err(e);
            }
        }
        result2 = copy(&mut target_reader, &mut *client_writer) => {
            if let Err(e) = result2 {
                debug!("Target to client copy failed: {}", e);
                return Err(e);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn bench_reply_performance() {
        let (server, _client) = tokio::io::duplex(1024);
        let mut writer = BufWriter::new(server);
        
        let result = CommandResult::success(
            "127.0.0.1".parse().unwrap(),
            8080
        );

        let start = Instant::now();
        for _ in 0..1000 {
            result.send_reply_optimized(&mut writer).await.unwrap();
        }
        let duration = start.elapsed();
        
        println!("1000 replies in {:?} ({:?}/reply)", duration, duration / 1000);
        assert!(duration.as_micros() < 10000); // < 10μs per reply
    }
}