use std::io;

use crate::connection::Reply;

#[derive(Debug, Clone, PartialEq)]
pub enum SocksError {
    InvalidVersion(u8),
    InvalidReservedByte(u8),
    UnsupportedAddressType(u8),
    UnsupportedCommand(u8),
    EmptyDomainName,
    InvalidDomainNameEncoding,
    DnsResolutionFailed,
    NoAddressesResolved,
    ConnectionFailed(io::ErrorKind),
    InvalidData,
    IoError(io::ErrorKind),
}

impl SocksError {
    pub fn to_reply_code(&self) -> u8 {
        match self {
            SocksError::InvalidVersion(_) => Reply::GENERAL_FAILURE,
            SocksError::InvalidReservedByte(_) => Reply::GENERAL_FAILURE,
            SocksError::UnsupportedAddressType(_) => Reply::ADDRESS_TYPE_NOT_SUPPORTED,
            SocksError::UnsupportedCommand(_) => Reply::COMMAND_NOT_SUPPORTED,
            SocksError::EmptyDomainName => Reply::GENERAL_FAILURE,
            SocksError::InvalidDomainNameEncoding => Reply::GENERAL_FAILURE,
            SocksError::DnsResolutionFailed => Reply::HOST_UNREACHABLE,
            SocksError::NoAddressesResolved => Reply::HOST_UNREACHABLE,
            SocksError::ConnectionFailed(kind) => match kind {
                io::ErrorKind::ConnectionRefused => Reply::CONNECTION_REFUSED,
                io::ErrorKind::TimedOut => Reply::HOST_UNREACHABLE,
                io::ErrorKind::AddrNotAvailable => Reply::HOST_UNREACHABLE,
                io::ErrorKind::NetworkUnreachable => Reply::NETWORK_UNREACHABLE,
                io::ErrorKind::PermissionDenied => Reply::CONNECTION_NOT_ALLOWED,
                _ => Reply::GENERAL_FAILURE,
            },
            SocksError::InvalidData => Reply::GENERAL_FAILURE,
            SocksError::IoError(_) => Reply::GENERAL_FAILURE,
        }
    }

    pub fn to_io_error(&self) -> io::Error {
        match self {
            SocksError::InvalidVersion(v) => io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid SOCKS version: {}", v),
            ),
            SocksError::InvalidReservedByte(b) => io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid reserved byte: {}", b),
            ),
            SocksError::UnsupportedAddressType(t) => io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unsupported address type: {}", t),
            ),
            SocksError::UnsupportedCommand(c) => io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unsupported command: {}", c),
            ),
            SocksError::EmptyDomainName => {
                io::Error::new(io::ErrorKind::InvalidData, "Empty domain name")
            }
            SocksError::InvalidDomainNameEncoding => {
                io::Error::new(io::ErrorKind::InvalidData, "Invalid domain name encoding")
            }
            SocksError::DnsResolutionFailed => {
                io::Error::new(io::ErrorKind::Other, "DNS resolution failed")
            }
            SocksError::NoAddressesResolved => {
                io::Error::new(io::ErrorKind::Other, "No addresses resolved for domain")
            }
            SocksError::ConnectionFailed(kind) => io::Error::new(*kind, "Connection failed"),
            SocksError::InvalidData => io::Error::new(io::ErrorKind::InvalidData, "Invalid data"),
            SocksError::IoError(kind) => io::Error::new(*kind, "IO error"),
        }
    }
}
