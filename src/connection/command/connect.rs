use std::{io, net::SocketAddr};

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::join;
use tokio::{
    io::{BufReader, BufWriter, copy},
    net::TcpStream,
};
use tracing::debug;

use crate::connection::command::send_reply;
use crate::connection::request::SocksRequest;
use crate::connection::{ATYP_IPV4, ATYP_IPV6, REPLY_SUCCESS};

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
        "[{client_addr}] Handling CONNECT request: {:?}",
        client_request
    );

    let target_stream =
        TcpStream::connect((client_request.dest_addr, client_request.dest_port)).await?;
    debug!(
        "[{client_addr}] Connected to target {}:{}",
        client_request.dest_addr, client_request.dest_port
    );

    let destination_addr = target_stream.local_addr()?;
    let destination_port = destination_addr.port();
    let destination_addr_type = if destination_addr.is_ipv4() {
        ATYP_IPV4
    } else {
        ATYP_IPV6
    };
    let destination_addr_as_bytes = match destination_addr.ip() {
        std::net::IpAddr::V4(addr) => addr.octets().to_vec(),
        std::net::IpAddr::V6(addr) => addr.octets().to_vec(),
    };

    send_reply(
        client_writer,
        REPLY_SUCCESS,
        destination_addr_type,
        &destination_addr_as_bytes,
        destination_port,
    )
    .await?;

    let (mut target_reader, mut target_writer) = target_stream.into_split();
    let (client_to_target, target_to_client) = join!(
        copy(&mut *client_reader, &mut target_writer),
        copy(&mut target_reader, &mut *client_writer)
    );

    client_to_target?;
    target_to_client?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::connection::{RESERVED, SOCKS5_VERSION};

    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};

    #[tokio::test]
    async fn test_send_reply_ipv4() {
        let (server, mut client) = duplex(1024);
        let mut writer = BufWriter::new(server);

        let addr_bytes = Ipv4Addr::new(192, 168, 1, 1).octets().to_vec();
        send_reply(&mut writer, REPLY_SUCCESS, ATYP_IPV4, &addr_bytes, 3128)
            .await
            .expect("Should send IPv4 reply");
        writer.flush().await.unwrap();

        let mut response = vec![0u8; 10];
        client.read_exact(&mut response).await.unwrap();
        assert_eq!(response[0], SOCKS5_VERSION);
        assert_eq!(response[1], REPLY_SUCCESS);
        assert_eq!(response[2], RESERVED);
        assert_eq!(response[3], ATYP_IPV4);
        assert_eq!(&response[4..8], &addr_bytes);
        assert_eq!(&response[8..10], 3128u16.to_be_bytes());
    }

    #[tokio::test]
    async fn test_send_reply_ipv6() {
        let (server, mut client) = duplex(1024);
        let mut writer = BufWriter::new(server);

        let addr_bytes = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1).octets().to_vec();
        send_reply(&mut writer, REPLY_SUCCESS, ATYP_IPV6, &addr_bytes, 8080)
            .await
            .expect("Should send IPv6 reply");
        writer.flush().await.unwrap();

        let mut response = vec![0u8; 22];
        client.read_exact(&mut response).await.unwrap();
        assert_eq!(response[0], SOCKS5_VERSION);
        assert_eq!(response[1], REPLY_SUCCESS);
        assert_eq!(response[2], RESERVED);
        assert_eq!(response[3], ATYP_IPV6);
        assert_eq!(&response[4..20], &addr_bytes);
        assert_eq!(&response[20..22], 8080u16.to_be_bytes());
    }

    #[tokio::test]
    async fn test_send_reply_error_codes() {
        let error_codes = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

        for &error_code in &error_codes {
            let (server, mut client) = duplex(1024);
            let mut writer = BufWriter::new(server);

            let addr_bytes = vec![127, 0, 0, 1];
            send_reply(&mut writer, error_code, ATYP_IPV4, &addr_bytes, 0)
                .await
                .expect("Should send error reply");
            writer.flush().await.unwrap();

            let mut response = vec![0u8; 10];
            client.read_exact(&mut response).await.unwrap();
            assert_eq!(response[0], SOCKS5_VERSION);
            assert_eq!(response[1], error_code);
            assert_eq!(response[2], RESERVED);
            assert_eq!(response[3], ATYP_IPV4);
        }
    }

    #[tokio::test]
    async fn test_send_reply_port_boundaries() {
        // Test port 0
        let (server, mut client) = duplex(1024);
        let mut writer = BufWriter::new(server);
        let addr_bytes = vec![127, 0, 0, 1];

        send_reply(&mut writer, REPLY_SUCCESS, ATYP_IPV4, &addr_bytes, 0)
            .await
            .expect("Should send reply with port 0");
        writer.flush().await.unwrap();

        let mut response = vec![0u8; 10];
        client.read_exact(&mut response).await.unwrap();
        assert_eq!(&response[8..10], 0u16.to_be_bytes());
        drop(writer);
        drop(client);

        // Test port 65535
        let (server, mut client) = duplex(1024);
        let mut writer = BufWriter::new(server);

        send_reply(&mut writer, REPLY_SUCCESS, ATYP_IPV4, &addr_bytes, 65535)
            .await
            .expect("Should send reply with port 65535");
        writer.flush().await.unwrap();

        let mut response = vec![0u8; 10];
        client.read_exact(&mut response).await.unwrap();
        assert_eq!(&response[8..10], 65535u16.to_be_bytes());
    }
}
