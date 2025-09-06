pub mod bind;
pub mod connect;
pub mod udp_associate;

use std::{io, net::SocketAddr};
use tokio::io::{AsyncRead, AsyncWrite, BufReader, BufWriter};

use crate::connection::{
    AddressType, ERROR_ADDR, ERROR_PORT, error::SocksError, reply::Reply, request::SocksRequest,
    send_reply,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    Connect = 0x01,
    Bind = 0x02,
    UdpAssociate = 0x03,
}

impl Command {
    pub const CONNECT: u8 = Self::Connect as u8;
    pub const BIND: u8 = Self::Bind as u8;
    pub const UDP_ASSOCIATE: u8 = Self::UdpAssociate as u8;

    pub async fn execute<R, W>(
        &self,
        client_request: SocksRequest,
        client_addr: SocketAddr,
        client_reader: &mut BufReader<R>,
        client_writer: &mut BufWriter<W>,
        tcp_nodelay: bool,
    ) -> io::Result<CommandResult>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        match self {
            Command::Connect => {
                connect::handle_command(
                    client_request,
                    client_addr,
                    client_reader,
                    client_writer,
                    tcp_nodelay,
                )
                .await
            }
            Command::Bind => {
                bind::handle_command(client_request, client_addr, client_reader, client_writer)
                    .await
            }
            Command::UdpAssociate => {
                udp_associate::handle_command(
                    client_request,
                    client_addr,
                    client_reader,
                    client_writer,
                )
                .await
            }
        }
    }

    pub fn parse_command(command: u8) -> Option<Command> {
        match command {
            0x01 => Some(Command::Connect),
            0x02 => Some(Command::Bind),
            0x03 => Some(Command::UdpAssociate),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Command::Connect => "CONNECT",
            Command::Bind => "BIND",
            Command::UdpAssociate => "UDP_ASSOCIATE",
        }
    }
}

#[derive(Debug)]
pub struct CommandResult {
    pub reply_code: u8,
    pub bind_addr: std::net::IpAddr,
    pub bind_port: u16
}

impl CommandResult {
    pub fn success(bind_addr: std::net::IpAddr, bind_port: u16) -> Self {
        Self {
            reply_code: Reply::SUCCESS,
            bind_addr,
            bind_port
        }
    }

    pub fn error(reply_code: u8) -> Self {
        Self {
            reply_code,
            bind_addr: std::net::IpAddr::from(ERROR_ADDR),
            bind_port: ERROR_PORT,
        }
    }

    pub fn from_socks_error(socks_error: &SocksError) -> Self {
        Self::error(socks_error.to_reply_code())
    }

    pub async fn send_reply<W>(&self, writer: &mut BufWriter<W>) -> io::Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        match self.bind_addr {
            std::net::IpAddr::V4(ipv4) => {
                let addr_bytes = ipv4.octets();
                send_reply(
                    writer,
                    self.reply_code,
                    AddressType::IPV4,
                    &addr_bytes,
                    self.bind_port,
                )
                .await
            }
            std::net::IpAddr::V6(ipv6) => {
                let addr_bytes = ipv6.octets();
                send_reply(
                    writer,
                    self.reply_code,
                    AddressType::IPV6,
                    &addr_bytes,
                    self.bind_port,
                )
                .await
            }
        }
    }

    pub fn is_success(&self) -> bool {
        self.reply_code == Reply::SUCCESS
    }

    pub fn is_error(&self) -> bool {
        !self.is_success()
    }

    pub fn reply_code(&self) -> u8 {
        self.reply_code
    }
}

#[cfg(test)]
mod tests {
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
}
