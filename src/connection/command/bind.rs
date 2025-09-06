use std::{io, net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncWrite, BufReader, BufWriter},
    net::TcpListener,
    time::timeout,
};
use tracing::{debug, warn};

use crate::connection::{command::CommandResult, reply::Reply, request::SocksRequest};

pub async fn handle_command<R, W>(
    client_request: SocksRequest,
    client_addr: SocketAddr,
    _client_reader: &mut BufReader<R>,
    client_writer: &mut BufWriter<W>,
) -> io::Result<CommandResult>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    debug!(
        "[{client_addr}] Handling BIND request: {:?}",
        client_request
    );

    let listener = match TcpListener::bind("0.0.0.0:0").await {
        Ok(listener) => listener,
        Err(e) => {
            debug!("[{client_addr}] Failed to create bind socket: {}", e);
            return Ok(CommandResult::error(Reply::GENERAL_FAILURE));
        }
    };

    let bound_addr = listener.local_addr()?;
    debug!("[{client_addr}] BIND socket created at {}", bound_addr);

    // Send first reply with bound address and port
    let first_reply = CommandResult::success(bound_addr.ip(), bound_addr.port());
    first_reply.send_reply(client_writer).await?;
    debug!(
        "[{client_addr}] Sent first BIND reply with bound address {}",
        bound_addr
    );

    let connection_result = timeout(Duration::from_secs(30), listener.accept()).await;

    match connection_result {
        Ok(Ok((stream, connecting_addr))) => {
            debug!(
                "[{client_addr}] BIND accepted connection from {}",
                connecting_addr
            );

            // Verify the connecting address matches the requested destination
            // According to RFC, the SOCKS server should use DST.ADDR and DST.PORT for evaluation
            if connecting_addr.ip() != client_request.dest_addr {
                warn!(
                    "[{client_addr}] BIND connection from {} doesn't match expected destination {}",
                    connecting_addr.ip(),
                    client_request.dest_addr
                );
                // Send second reply with connection refused
                let second_reply = CommandResult::error(Reply::CONNECTION_REFUSED);
                second_reply.send_reply(client_writer).await?;
                return Ok(second_reply);
            }

            // Send second reply with connecting host address and port
            let second_reply = CommandResult::success(connecting_addr.ip(), connecting_addr.port());
            second_reply.send_reply(client_writer).await?;
            debug!(
                "[{client_addr}] Sent second BIND reply with connecting address {}",
                connecting_addr
            );

            Ok(second_reply)
        }
        Ok(Err(e)) => {
            debug!("[{client_addr}] BIND accept failed: {}", e);
            let second_reply = CommandResult::error(Reply::GENERAL_FAILURE);
            second_reply.send_reply(client_writer).await?;
            Ok(second_reply)
        }
        Err(_) => {
            debug!("[{client_addr}] BIND timeout waiting for connection");
            let second_reply = CommandResult::error(Reply::TTL_EXPIRED);
            second_reply.send_reply(client_writer).await?;
            Ok(second_reply)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::{AddressType, command::Command};
    use std::net::{IpAddr, Ipv4Addr};
    use tokio::{io::BufReader, time::sleep};

    fn create_test_request() -> SocksRequest {
        SocksRequest {
            version: 0x05,
            command: Command::BIND as u8,
            reserved: 0x00,
            address_type: AddressType::IPV4,
            dest_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            dest_port: 8080,
        }
    }

    #[tokio::test]
    async fn test_bind_command_timeout() {
        let request = create_test_request();
        let client_addr = "127.0.0.1:12345".parse().unwrap();

        let (client_read, client_write) = tokio::io::duplex(1024);
        let mut reader = BufReader::new(client_read);
        let mut writer = tokio::io::BufWriter::new(client_write);

        // This should timeout since no connection will be made
        let result = timeout(
            Duration::from_millis(100),
            handle_command(request, client_addr, &mut reader, &mut writer),
        )
        .await;

        // Should timeout
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_bind_socket_creation_success() {
        let request = create_test_request();
        let client_addr = "127.0.0.1:12345".parse().unwrap();

        let (client_read, client_write) = tokio::io::duplex(1024);
        let mut reader = BufReader::new(client_read);
        let mut writer = tokio::io::BufWriter::new(client_write);

        // Start the bind command in a task
        let handle = tokio::spawn(async move {
            handle_command(request, client_addr, &mut reader, &mut writer).await
        });

        // Give it a moment to create the socket and send first reply
        sleep(Duration::from_millis(10)).await;

        // Cancel the task
        handle.abort();
    }

    #[tokio::test]
    async fn test_bind_socket_creation() {
        let request = create_test_request();
        let client_addr = "127.0.0.1:12345".parse().unwrap();

        let (client_read, mut client_write) = tokio::io::duplex(1024);
        let mut reader = BufReader::new(client_read);
        let mut writer = tokio::io::BufWriter::new(&mut client_write);

        // Test that a bind socket can be created (will timeout waiting for connection)
        let result = timeout(
            Duration::from_millis(100),
            handle_command(request, client_addr, &mut reader, &mut writer),
        )
        .await;

        // Should timeout because we didn't send anyone to connect
        assert!(result.is_err());
    }
}
