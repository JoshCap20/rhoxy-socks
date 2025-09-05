
use rhoxy_socks::connection::method::method::Method;
use rhoxy_socks::{connection::SOCKS5_VERSION, handle_connection, config::ConnectionConfig};
use std::net::Ipv6Addr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task;
use tokio::time::timeout;

fn default_test_config() -> ConnectionConfig {
    ConnectionConfig {
        buffer_size: 32 * 1024,
        tcp_nodelay: true,
        keep_alive: Some(std::time::Duration::from_secs(60)),
        connection_timeout: std::time::Duration::from_secs(30),
        bind_addr: None,
        metrics_enabled: false,
        supported_auth_methods: vec![Method::NO_AUTHENTICATION_REQUIRED],
    }
}

#[tokio::test]
async fn test_full_socks5_connect_ipv4() {
    // Spawn a mock target server (echo server on localhost:0)
    let target_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target_addr = target_listener.local_addr().unwrap();
    let target_handle = task::spawn(async move {
        if let Ok((mut socket, _)) = target_listener.accept().await {
            let mut buf = [0u8; 1024];
            let n = socket.read(&mut buf).await.unwrap();
            socket.write_all(&buf[..n]).await.unwrap();
        }
    });

    // Spawn SOCKS server on localhost:0
    let socks_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let socks_addr = socks_listener.local_addr().unwrap();
    let socks_handle = task::spawn(async move {
        let (socket, client_addr) = socks_listener.accept().await.unwrap();
        handle_connection(socket, client_addr, default_test_config()).await.unwrap();
    });

    // Client: Connect to SOCKS, do handshake
    let mut client = TcpStream::connect(socks_addr).await.unwrap();

    // Handshake: Send VER=0x05, NMETHODS=1, METHODS=0x00
    client.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    client.flush().await.unwrap();
    let mut response = [0u8; 2];
    client.read_exact(&mut response).await.unwrap();
    assert_eq!(response, [SOCKS5_VERSION, 0x00]);

    // Request: CONNECT to target (ATYP=0x01, ADDR=127.0.0.1, PORT=target.port)
    let addr_bytes = [127, 0, 0, 1];
    let port_bytes = target_addr.port().to_be_bytes();
    let mut request = vec![0x05, 0x01, 0x00, 0x01];
    request.extend_from_slice(&addr_bytes);
    request.extend_from_slice(&port_bytes);
    client.write_all(&request).await.unwrap();
    client.flush().await.unwrap();

    // Read reply: Should be success (REP=0x00)
    let mut reply = vec![0u8; 10];
    client.read_exact(&mut reply).await.unwrap();
    assert_eq!(reply[0], SOCKS5_VERSION);
    assert_eq!(reply[1], 0x00);

    // Relay data: Send "hello", expect echo
    client.write_all(b"hello").await.unwrap();
    client.flush().await.unwrap();
    let mut buf = [0u8; 5];
    client.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"hello");

    // Cleanup
    drop(client);
    socks_handle.await.unwrap();
    target_handle.await.unwrap();
}

#[tokio::test]
async fn test_full_socks5_connect_ipv6() {
    // Spawn a mock target server (echo server on [::1]:0)
    let target_listener = TcpListener::bind("[::1]:0").await.unwrap();
    let target_addr = target_listener.local_addr().unwrap();
    let target_handle = task::spawn(async move {
        if let Ok((mut socket, _)) = target_listener.accept().await {
            let mut buf = [0u8; 1024];
            let n = socket.read(&mut buf).await.unwrap();
            socket.write_all(&buf[..n]).await.unwrap();
        }
    });

    // Spawn SOCKS server on [::1]:0
    let socks_listener = TcpListener::bind("[::1]:0").await.unwrap();
    let socks_addr = socks_listener.local_addr().unwrap();
    let socks_handle = task::spawn(async move {
        let (socket, client_addr) = socks_listener.accept().await.unwrap();
        handle_connection(socket, client_addr, default_test_config()).await.unwrap();
    });

    // Client: Connect to SOCKS, do handshake
    let mut client = TcpStream::connect(socks_addr).await.unwrap();

    // Handshake: Send VER=0x05, NMETHODS=1, METHODS=0x00
    client.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    client.flush().await.unwrap();
    let mut response = [0u8; 2];
    client.read_exact(&mut response).await.unwrap();
    assert_eq!(response, [SOCKS5_VERSION, 0x00]);

    // Request: CONNECT to IPv6 target (ATYP=0x04, ADDR=::1, PORT=target.port)
    let addr_bytes = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1).octets();
    let port_bytes = target_addr.port().to_be_bytes();
    let mut request = vec![0x05, 0x01, 0x00, 0x04];
    request.extend_from_slice(&addr_bytes);
    request.extend_from_slice(&port_bytes);
    client.write_all(&request).await.unwrap();
    client.flush().await.unwrap();

    // Read reply: Should be success (REP=0x00)
    let mut reply = vec![0u8; 4 + 16 + 2];
    client.read_exact(&mut reply).await.unwrap();
    assert_eq!(reply[0], SOCKS5_VERSION);
    assert_eq!(reply[1], 0x00);

    // Relay data: Send "hello ipv6", expect echo
    client.write_all(b"hello ipv6").await.unwrap();
    client.flush().await.unwrap();
    let mut buf = [0u8; 10];
    client.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"hello ipv6");

    // Cleanup
    drop(client);
    socks_handle.await.unwrap();
    target_handle.await.unwrap();
}

#[tokio::test]
async fn test_connection_refused() {
    // Spawn SOCKS server
    let socks_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let socks_addr = socks_listener.local_addr().unwrap();
    let socks_handle = task::spawn(async move {
        let (socket, client_addr) = socks_listener.accept().await.unwrap();
        let _ = handle_connection(socket, client_addr, default_test_config()).await;
    });

    // Client: Connect to SOCKS, do handshake
    let mut client = TcpStream::connect(socks_addr).await.unwrap();

    // Handshake
    client.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    client.flush().await.unwrap();
    let mut response = [0u8; 2];
    client.read_exact(&mut response).await.unwrap();
    assert_eq!(response, [SOCKS5_VERSION, 0x00]);

    // Request: CONNECT to unreachable port (127.0.0.1:1 should be refused)
    let mut request = vec![0x05, 0x01, 0x00, 0x01];
    request.extend_from_slice(&[127, 0, 0, 1]);
    request.extend_from_slice(&1u16.to_be_bytes());
    client.write_all(&request).await.unwrap();
    client.flush().await.unwrap();

    // Should get connection refused or network unreachable error
    let mut reply = vec![0u8; 10];
    let result = timeout(Duration::from_secs(5), client.read_exact(&mut reply)).await;

    // Connection should either close or return error reply
    match result {
        Ok(Ok(_)) => {
            assert_eq!(reply[0], SOCKS5_VERSION);
            assert_ne!(reply[1], 0x00); // Should not be success
        }
        _ => {
            // Connection closed, which is also acceptable behavior
        }
    }

    drop(client);
    let _ = socks_handle.await;
}

#[tokio::test]
async fn test_concurrent_connections() {
    use std::sync::Arc;
    use tokio::sync::Barrier;

    // Spawn a mock target server that handles multiple connections
    let target_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target_addr = target_listener.local_addr().unwrap();
    let target_handle = task::spawn(async move {
        for _ in 0..3 {
            if let Ok((mut socket, _)) = target_listener.accept().await {
                task::spawn(async move {
                    let mut buf = [0u8; 1024];
                    if let Ok(n) = socket.read(&mut buf).await {
                        let _ = socket.write_all(&buf[..n]).await;
                    }
                });
            }
        }
    });

    // Spawn SOCKS server
    let socks_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let socks_addr = socks_listener.local_addr().unwrap();
    let socks_handle = task::spawn(async move {
        for _ in 0..3 {
            if let Ok((socket, client_addr)) = socks_listener.accept().await {
                task::spawn(async move {
                    let _ = handle_connection(socket, client_addr, default_test_config()).await;
                });
            }
        }
    });

    let barrier = Arc::new(Barrier::new(3));
    let mut client_handles = Vec::new();

    // Launch 3 concurrent clients
    for i in 0..3 {
        let barrier = barrier.clone();
        let message = format!("client{}", i);
        let handle = task::spawn(async move {
            barrier.wait().await;

            let mut client = TcpStream::connect(socks_addr).await.unwrap();

            // Handshake
            client.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
            client.flush().await.unwrap();
            let mut response = [0u8; 2];
            client.read_exact(&mut response).await.unwrap();
            assert_eq!(response, [SOCKS5_VERSION, 0x00]);

            // Request
            let addr_bytes = [127, 0, 0, 1];
            let port_bytes = target_addr.port().to_be_bytes();
            let mut request = vec![0x05, 0x01, 0x00, 0x01];
            request.extend_from_slice(&addr_bytes);
            request.extend_from_slice(&port_bytes);
            client.write_all(&request).await.unwrap();
            client.flush().await.unwrap();

            // Read reply
            let mut reply = vec![0u8; 10];
            client.read_exact(&mut reply).await.unwrap();
            assert_eq!(reply[0], SOCKS5_VERSION);
            assert_eq!(reply[1], 0x00);

            // Send unique message
            client.write_all(message.as_bytes()).await.unwrap();
            client.flush().await.unwrap();
            let mut buf = vec![0u8; message.len()];
            client.read_exact(&mut buf).await.unwrap();
            assert_eq!(buf, message.as_bytes());

            drop(client);
        });
        client_handles.push(handle);
    }

    // Wait for all clients to complete
    for handle in client_handles {
        handle.await.unwrap();
    }

    socks_handle.await.unwrap();
    target_handle.await.unwrap();
}

#[tokio::test]
async fn test_client_disconnect_during_handshake() {
    let socks_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let socks_addr = socks_listener.local_addr().unwrap();

    let socks_handle = task::spawn(async move {
        let (socket, client_addr) = socks_listener.accept().await.unwrap();
        let result = handle_connection(socket, client_addr, default_test_config()).await;
        assert!(result.is_err()); // Should fail due to client disconnect
    });

    let mut client = TcpStream::connect(socks_addr).await.unwrap();
    // Send partial handshake then disconnect
    client.write_all(&[0x05]).await.unwrap();
    drop(client); // Disconnect immediately

    socks_handle.await.unwrap();
}

#[tokio::test]
async fn test_unsupported_bind_command() {
    let socks_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let socks_addr = socks_listener.local_addr().unwrap();
    let socks_handle = task::spawn(async move {
        let (socket, client_addr) = socks_listener.accept().await.unwrap();
        let _ = handle_connection(socket, client_addr, default_test_config()).await;
    });

    let mut client = TcpStream::connect(socks_addr).await.unwrap();

    // Handshake
    client.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    client.flush().await.unwrap();
    let mut response = [0u8; 2];
    client.read_exact(&mut response).await.unwrap();
    assert_eq!(response, [SOCKS5_VERSION, 0x00]);

    // BIND request (unsupported)
    let mut request = vec![0x05, 0x02, 0x00, 0x01]; // BIND command
    request.extend_from_slice(&[127, 0, 0, 1]);
    request.extend_from_slice(&8080u16.to_be_bytes());
    client.write_all(&request).await.unwrap();
    client.flush().await.unwrap();

    // Should get connection closed or error reply
    let mut reply = vec![0u8; 10];
    let result = timeout(Duration::from_secs(2), client.read_exact(&mut reply)).await;

    match result {
        Ok(Ok(_)) => {
            // Got a reply - should be an error code
            assert_eq!(reply[0], SOCKS5_VERSION);
            assert_ne!(reply[1], 0x00); // Should not be success
        }
        _ => {
            // Connection closed, which is acceptable for unsupported commands
        }
    }

    drop(client);
    let _ = socks_handle.await;
}

#[tokio::test]
async fn test_unsupported_udp_associate_command() {
    let socks_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let socks_addr = socks_listener.local_addr().unwrap();
    let socks_handle = task::spawn(async move {
        let (socket, client_addr) = socks_listener.accept().await.unwrap();
        let _ = handle_connection(socket, client_addr, default_test_config()).await;
    });

    let mut client = TcpStream::connect(socks_addr).await.unwrap();

    // Handshake
    client.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    client.flush().await.unwrap();
    let mut response = [0u8; 2];
    client.read_exact(&mut response).await.unwrap();
    assert_eq!(response, [SOCKS5_VERSION, 0x00]);

    // UDP_ASSOCIATE request (unsupported)
    let mut request = vec![0x05, 0x03, 0x00, 0x01]; // UDP_ASSOCIATE command
    request.extend_from_slice(&[127, 0, 0, 1]);
    request.extend_from_slice(&8080u16.to_be_bytes());
    client.write_all(&request).await.unwrap();
    client.flush().await.unwrap();

    // Should get connection closed or error reply
    let mut reply = vec![0u8; 10];
    let result = timeout(Duration::from_secs(2), client.read_exact(&mut reply)).await;

    match result {
        Ok(Ok(_)) => {
            assert_eq!(reply[0], SOCKS5_VERSION);
            assert_ne!(reply[1], 0x00); // Should not be success
        }
        _ => {
            // Connection closed, which is acceptable for unsupported commands
        }
    }

    drop(client);
    let _ = socks_handle.await;
}

#[tokio::test]
async fn test_malformed_handshake_too_few_bytes() {
    let socks_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let socks_addr = socks_listener.local_addr().unwrap();
    let socks_handle = task::spawn(async move {
        let (socket, client_addr) = socks_listener.accept().await.unwrap();
        let result = handle_connection(socket, client_addr, default_test_config()).await;
        assert!(result.is_err()); // Should fail due to malformed handshake
    });

    let mut client = TcpStream::connect(socks_addr).await.unwrap();
    // Send incomplete handshake (missing methods)
    client.write_all(&[0x05, 0x01]).await.unwrap();
    client.flush().await.unwrap();
    drop(client);

    socks_handle.await.unwrap();
}

#[tokio::test]
async fn test_malformed_request_invalid_address_type() {
    let socks_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let socks_addr = socks_listener.local_addr().unwrap();
    let socks_handle = task::spawn(async move {
        let (socket, client_addr) = socks_listener.accept().await.unwrap();
        let _ = handle_connection(socket, client_addr, default_test_config()).await;
    });

    let mut client = TcpStream::connect(socks_addr).await.unwrap();

    // Handshake
    client.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    client.flush().await.unwrap();
    let mut response = [0u8; 2];
    client.read_exact(&mut response).await.unwrap();
    assert_eq!(response, [SOCKS5_VERSION, 0x00]);

    // Request with invalid address type (0x99)
    let mut request = vec![0x05, 0x01, 0x00, 0x99]; // Invalid ATYP
    request.extend_from_slice(&[127, 0, 0, 1]);
    request.extend_from_slice(&8080u16.to_be_bytes());
    client.write_all(&request).await.unwrap();
    client.flush().await.unwrap();

    // Should get error or connection close
    let mut reply = vec![0u8; 10];
    let result = timeout(Duration::from_secs(2), client.read_exact(&mut reply)).await;

    match result {
        Ok(Ok(_)) => {
            assert_eq!(reply[0], SOCKS5_VERSION);
            assert_ne!(reply[1], 0x00); // Should not be success
        }
        _ => {
            // Connection closed is acceptable for invalid requests
        }
    }

    drop(client);
    let _ = socks_handle.await;
}

#[tokio::test]
async fn test_invalid_socks_version_in_handshake() {
    let socks_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let socks_addr = socks_listener.local_addr().unwrap();
    let socks_handle = task::spawn(async move {
        let (socket, client_addr) = socks_listener.accept().await.unwrap();
        let result = handle_connection(socket, client_addr, default_test_config()).await;
        assert!(result.is_err()); // Should fail due to invalid version
    });

    let mut client = TcpStream::connect(socks_addr).await.unwrap();
    // Send SOCKS4 version instead of SOCKS5
    client.write_all(&[0x04, 0x01, 0x00]).await.unwrap();
    client.flush().await.unwrap();
    drop(client);

    socks_handle.await.unwrap();
}

#[tokio::test]
async fn test_invalid_socks_version_in_request() {
    let socks_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let socks_addr = socks_listener.local_addr().unwrap();
    let socks_handle = task::spawn(async move {
        let (socket, client_addr) = socks_listener.accept().await.unwrap();
        let _ = handle_connection(socket, client_addr, default_test_config()).await;
    });

    let mut client = TcpStream::connect(socks_addr).await.unwrap();

    // Valid handshake
    client.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    client.flush().await.unwrap();
    let mut response = [0u8; 2];
    client.read_exact(&mut response).await.unwrap();
    assert_eq!(response, [SOCKS5_VERSION, 0x00]);

    // Request with wrong version
    let mut request = vec![0x04, 0x01, 0x00, 0x01]; // SOCKS4 version in request
    request.extend_from_slice(&[127, 0, 0, 1]);
    request.extend_from_slice(&8080u16.to_be_bytes());
    client.write_all(&request).await.unwrap();
    client.flush().await.unwrap();

    // Should get error or connection close
    let mut reply = vec![0u8; 10];
    let result = timeout(Duration::from_secs(2), client.read_exact(&mut reply)).await;

    match result {
        Ok(Ok(_)) => {
            assert_eq!(reply[0], SOCKS5_VERSION);
            assert_ne!(reply[1], 0x00);
        }
        _ => {
            // Connection closed is acceptable
        }
    }

    drop(client);
    let _ = socks_handle.await;
}

#[tokio::test]
async fn test_client_disconnect_during_request() {
    let socks_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let socks_addr = socks_listener.local_addr().unwrap();
    let socks_handle = task::spawn(async move {
        let (socket, client_addr) = socks_listener.accept().await.unwrap();
        let result = handle_connection(socket, client_addr, default_test_config()).await;
        assert!(result.is_err()); // Should fail due to client disconnect
    });

    let mut client = TcpStream::connect(socks_addr).await.unwrap();

    // Complete handshake
    client.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    client.flush().await.unwrap();
    let mut response = [0u8; 2];
    client.read_exact(&mut response).await.unwrap();
    assert_eq!(response, [SOCKS5_VERSION, 0x00]);

    // Send partial request then disconnect
    client.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    drop(client); // Disconnect before sending complete request

    socks_handle.await.unwrap();
}

#[tokio::test]
async fn test_zero_byte_transfer() {
    // Echo server that immediately closes after accepting
    let target_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target_addr = target_listener.local_addr().unwrap();
    let target_handle = task::spawn(async move {
        if let Ok((socket, _)) = target_listener.accept().await {
            drop(socket); // Close immediately without reading/writing
        }
    });

    let socks_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let socks_addr = socks_listener.local_addr().unwrap();
    let socks_handle = task::spawn(async move {
        let (socket, client_addr) = socks_listener.accept().await.unwrap();
        let _ = handle_connection(socket, client_addr, default_test_config()).await;
    });

    let mut client = TcpStream::connect(socks_addr).await.unwrap();

    // Handshake
    client.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    client.flush().await.unwrap();
    let mut response = [0u8; 2];
    client.read_exact(&mut response).await.unwrap();
    assert_eq!(response, [SOCKS5_VERSION, 0x00]);

    // Request
    let addr_bytes = [127, 0, 0, 1];
    let port_bytes = target_addr.port().to_be_bytes();
    let mut request = vec![0x05, 0x01, 0x00, 0x01];
    request.extend_from_slice(&addr_bytes);
    request.extend_from_slice(&port_bytes);
    client.write_all(&request).await.unwrap();
    client.flush().await.unwrap();

    // Read reply
    let mut reply = vec![0u8; 10];
    client.read_exact(&mut reply).await.unwrap();
    assert_eq!(reply[0], SOCKS5_VERSION);
    assert_eq!(reply[1], 0x00);

    // Try to read from connection - should close quickly
    let mut buf = [0u8; 1];
    let result = timeout(Duration::from_secs(2), client.read(&mut buf)).await;

    match result {
        Ok(Ok(0)) => {
            // Connection closed as expected
        }
        Ok(Err(_)) => {
            // Error is also acceptable
        }
        Err(_) => {
            // Timeout is acceptable too
        }
        _ => {}
    }

    drop(client);
    let _ = socks_handle.await;
    target_handle.await.unwrap();
}
