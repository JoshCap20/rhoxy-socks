use super::*;

#[cfg(test)]
mod command_tests {
    use super::*;

    #[test]
    fn test_command_parse_valid() {
        assert_eq!(Command::parse_command(0x01), Some(Command::Connect));
        assert_eq!(Command::parse_command(0x02), Some(Command::Bind));
        assert_eq!(Command::parse_command(0x03), Some(Command::UdpAssociate));
    }

    #[test]
    fn test_command_parse_invalid() {
        assert_eq!(Command::parse_command(0x00), None);
        assert_eq!(Command::parse_command(0x04), None);
        assert_eq!(Command::parse_command(0xFF), None);
    }

    #[test]
    fn test_command_as_u8() {
        assert_eq!(Command::Connect.as_u8(), 0x01);
        assert_eq!(Command::Bind.as_u8(), 0x02);
        assert_eq!(Command::UdpAssociate.as_u8(), 0x03);
    }

    #[test]
    fn test_command_name() {
        assert_eq!(Command::Connect.name(), "CONNECT");
        assert_eq!(Command::Bind.name(), "BIND");
        assert_eq!(Command::UdpAssociate.name(), "UDP_ASSOCIATE");
    }

    #[test]
    fn test_command_debug() {
        assert!(format!("{:?}", Command::Connect).contains("Connect"));
        assert!(format!("{:?}", Command::Bind).contains("Bind"));
        assert!(format!("{:?}", Command::UdpAssociate).contains("UdpAssociate"));
    }

    #[test]
    fn test_command_equality() {
        assert_eq!(Command::Connect, Command::Connect);
        assert_ne!(Command::Connect, Command::Bind);
        assert_ne!(Command::Bind, Command::UdpAssociate);
    }

    #[test]
    fn test_command_clone() {
        let cmd = Command::Connect;
        let cloned = cmd.clone();
        assert_eq!(cmd, cloned);
    }

    #[test]
    fn test_command_copy() {
        let cmd = Command::Connect;
        let copied = cmd;
        assert_eq!(cmd, copied);
    }

    #[tokio::test]
    async fn test_bind_command_returns_error() {
        use std::net::SocketAddr;
        use tokio::io::{duplex, BufReader, BufWriter};
        use crate::connection::request::SocksRequest;
        use std::net::{IpAddr, Ipv4Addr};

        let (_, server) = duplex(1024);
        let mut reader = BufReader::new(server);
        let (client, _) = duplex(1024);
        let mut writer = BufWriter::new(client);
        
        let request = SocksRequest {
            version: 5,
            command: 0x02,
            reserved: 0,
            address_type: 1,
            dest_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            dest_port: 8080,
        };

        let client_addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let result = Command::Bind.execute(request, client_addr, &mut reader, &mut writer).await;
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
        assert!(err.to_string().contains("BIND request handling not implemented"));
    }

    #[tokio::test]
    async fn test_udp_associate_command_returns_error() {
        use std::net::SocketAddr;
        use tokio::io::{duplex, BufReader, BufWriter};
        use crate::connection::request::SocksRequest;
        use std::net::{IpAddr, Ipv4Addr};

        let (_, server) = duplex(1024);
        let mut reader = BufReader::new(server);
        let (client, _) = duplex(1024);
        let mut writer = BufWriter::new(client);
        
        let request = SocksRequest {
            version: 5,
            command: 0x03,
            reserved: 0,
            address_type: 1,
            dest_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            dest_port: 8080,
        };

        let client_addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let result = Command::UdpAssociate.execute(request, client_addr, &mut reader, &mut writer).await;
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
        assert!(err.to_string().contains("UDP ASSOCIATE request handling not implemented"));
    }
}