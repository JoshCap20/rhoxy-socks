use tokio::io::{AsyncRead, AsyncReadExt, BufReader};

use crate::connection::{error::SocksError, resolve_domain};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AddressType {
    IPv4 = 0x01,
    DomainName = 0x03,
    IPv6 = 0x04,
}

impl AddressType {
    pub const IPV4: u8 = Self::IPv4 as u8;
    pub const DOMAIN_NAME: u8 = Self::DomainName as u8;
    pub const IPV6: u8 = Self::IPv6 as u8;

    pub fn from_u8(value: u8) -> Option<AddressType> {
        match value {
            Self::IPV4 => Some(AddressType::IPv4),
            Self::DOMAIN_NAME => Some(AddressType::DomainName),
            Self::IPV6 => Some(AddressType::IPv6),
            _ => None,
        }
    }

    pub async fn parse<R>(
        reader: &mut BufReader<R>,
        atyp: u8,
    ) -> Result<std::net::IpAddr, SocksError>
    where
        R: AsyncRead + Unpin,
    {
        match AddressType::from_u8(atyp) {
            Some(AddressType::IPv4) => Self::parse_ipv4(reader).await,
            Some(AddressType::DomainName) => Self::parse_domain_name(reader).await,
            Some(AddressType::IPv6) => Self::parse_ipv6(reader).await,
            None => Err(SocksError::UnsupportedAddressType(atyp)),
        }
    }

    async fn parse_ipv4<R>(reader: &mut BufReader<R>) -> Result<std::net::IpAddr, SocksError>
    where
        R: AsyncRead + Unpin,
    {
        let mut addr = [0u8; 4];
        reader
            .read_exact(&mut addr)
            .await
            .map_err(|e| SocksError::IoError(e.kind()))?;
        Ok(std::net::IpAddr::from(addr))
    }

    async fn parse_ipv6<R>(reader: &mut BufReader<R>) -> Result<std::net::IpAddr, SocksError>
    where
        R: AsyncRead + Unpin,
    {
        let mut addr = [0u8; 16];
        reader
            .read_exact(&mut addr)
            .await
            .map_err(|e| SocksError::IoError(e.kind()))?;
        Ok(std::net::IpAddr::from(addr))
    }

    async fn parse_domain_name<R>(reader: &mut BufReader<R>) -> Result<std::net::IpAddr, SocksError>
    where
        R: AsyncRead + Unpin,
    {
        let domain_len = reader
            .read_u8()
            .await
            .map_err(|e| SocksError::IoError(e.kind()))? as usize;
        if domain_len == 0 {
            return Err(SocksError::EmptyDomainName);
        }

        let mut domain = vec![0u8; domain_len];
        reader
            .read_exact(&mut domain)
            .await
            .map_err(|e| SocksError::IoError(e.kind()))?;

        let domain_str =
            String::from_utf8(domain).map_err(|_| SocksError::InvalidDomainNameEncoding)?;

        let resolved_addrs = resolve_domain(&domain_str)
            .await
            .map_err(|_| SocksError::DnsResolutionFailed)?;

        let addr = resolved_addrs
            .first()
            .ok_or(SocksError::NoAddressesResolved)?
            .ip();

        Ok(addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use tokio::io::BufReader;

    #[test]
    fn test_address_type_from_u8() {
        assert_eq!(AddressType::from_u8(0x01), Some(AddressType::IPv4));
        assert_eq!(AddressType::from_u8(0x03), Some(AddressType::DomainName));
        assert_eq!(AddressType::from_u8(0x04), Some(AddressType::IPv6));
        assert_eq!(AddressType::from_u8(0x00), None);
        assert_eq!(AddressType::from_u8(0x02), None);
        assert_eq!(AddressType::from_u8(0x05), None);
        assert_eq!(AddressType::from_u8(0xFF), None);
    }

    #[test]
    fn test_address_type_constants() {
        assert_eq!(AddressType::IPV4, 0x01);
        assert_eq!(AddressType::DOMAIN_NAME, 0x03);
        assert_eq!(AddressType::IPV6, 0x04);
    }

    #[test]
    fn test_address_type_equality() {
        assert_eq!(AddressType::IPv4, AddressType::IPv4);
        assert_ne!(AddressType::IPv4, AddressType::IPv6);
        assert_ne!(AddressType::IPv4, AddressType::DomainName);
    }

    #[tokio::test]
    async fn test_parse_ipv4_valid() {
        let data = vec![127, 0, 0, 1];
        let mut reader = BufReader::new(data.as_slice());

        let result = AddressType::parse(&mut reader, AddressType::IPV4).await;
        assert!(result.is_ok());

        let addr = result.unwrap();
        assert_eq!(addr, std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    }

    #[tokio::test]
    async fn test_parse_ipv4_incomplete_data() {
        let data = vec![127, 0, 0]; // Missing one byte
        let mut reader = BufReader::new(data.as_slice());

        let result = AddressType::parse(&mut reader, AddressType::IPV4).await;
        assert!(result.is_err());

        if let Err(SocksError::IoError(kind)) = result {
            assert_eq!(kind, std::io::ErrorKind::UnexpectedEof);
        } else {
            panic!("Expected IoError with UnexpectedEof");
        }
    }

    #[tokio::test]
    async fn test_parse_ipv6_valid() {
        let data = vec![
            0x20, 0x01, 0x0d, 0xb8, 0x85, 0xa3, 0x00, 0x00, 0x00, 0x00, 0x8a, 0x2e, 0x03, 0x70,
            0x73, 0x34,
        ];
        let mut reader = BufReader::new(data.as_slice());

        let result = AddressType::parse(&mut reader, AddressType::IPV6).await;
        assert!(result.is_ok());

        let addr = result.unwrap();
        if let std::net::IpAddr::V6(ipv6_addr) = addr {
            assert_eq!(
                ipv6_addr.segments(),
                [
                    0x2001, 0x0db8, 0x85a3, 0x0000, 0x0000, 0x8a2e, 0x0370, 0x7334
                ]
            );
        } else {
            panic!("Expected IPv6 address");
        }
    }

    #[tokio::test]
    async fn test_parse_ipv6_incomplete_data() {
        let data = vec![0x20, 0x01, 0x0d, 0xb8]; // Only 4 bytes instead of 16
        let mut reader = BufReader::new(data.as_slice());

        let result = AddressType::parse(&mut reader, AddressType::IPV6).await;
        assert!(result.is_err());

        if let Err(SocksError::IoError(kind)) = result {
            assert_eq!(kind, std::io::ErrorKind::UnexpectedEof);
        } else {
            panic!("Expected IoError with UnexpectedEof");
        }
    }

    #[tokio::test]
    async fn test_parse_domain_name_empty() {
        let data = vec![0]; // Domain length = 0
        let mut reader = BufReader::new(data.as_slice());

        let result = AddressType::parse(&mut reader, AddressType::DOMAIN_NAME).await;
        assert!(result.is_err());

        if let Err(SocksError::EmptyDomainName) = result {
            // Expected
        } else {
            panic!("Expected EmptyDomainName error");
        }
    }

    #[tokio::test]
    async fn test_parse_domain_name_invalid_utf8() {
        let data = vec![3, 0xFF, 0xFE, 0xFD]; // Invalid UTF-8 sequence
        let mut reader = BufReader::new(data.as_slice());

        let result = AddressType::parse(&mut reader, AddressType::DOMAIN_NAME).await;
        assert!(result.is_err());

        if let Err(SocksError::InvalidDomainNameEncoding) = result {
            // Expected
        } else {
            panic!("Expected InvalidDomainNameEncoding error");
        }
    }

    #[tokio::test]
    async fn test_parse_domain_name_incomplete_length() {
        let data = vec![]; // No domain length byte
        let mut reader = BufReader::new(data.as_slice());

        let result = AddressType::parse(&mut reader, AddressType::DOMAIN_NAME).await;
        assert!(result.is_err());

        if let Err(SocksError::IoError(kind)) = result {
            assert_eq!(kind, std::io::ErrorKind::UnexpectedEof);
        } else {
            panic!("Expected IoError with UnexpectedEof");
        }
    }

    #[tokio::test]
    async fn test_parse_domain_name_incomplete_data() {
        let data = vec![5, b'h', b'e']; // Claims 5 bytes but only provides 2
        let mut reader = BufReader::new(data.as_slice());

        let result = AddressType::parse(&mut reader, AddressType::DOMAIN_NAME).await;
        assert!(result.is_err());

        if let Err(SocksError::IoError(kind)) = result {
            assert_eq!(kind, std::io::ErrorKind::UnexpectedEof);
        } else {
            panic!("Expected IoError with UnexpectedEof");
        }
    }

    #[tokio::test]
    async fn test_parse_unsupported_address_type() {
        let data = vec![127, 0, 0, 1];
        let mut reader = BufReader::new(data.as_slice());

        let result = AddressType::parse(&mut reader, 0x99).await; // Invalid ATYP
        assert!(result.is_err());

        if let Err(SocksError::UnsupportedAddressType(atyp)) = result {
            assert_eq!(atyp, 0x99);
        } else {
            panic!("Expected UnsupportedAddressType error");
        }
    }

    #[test]
    fn test_address_type_debug() {
        let ipv4 = AddressType::IPv4;
        let debug_str = format!("{:?}", ipv4);
        assert_eq!(debug_str, "IPv4");
    }

    #[test]
    fn test_address_type_clone() {
        let original = AddressType::IPv6;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_address_type_copy() {
        let original = AddressType::DomainName;
        let copied = original;
        assert_eq!(original, copied);
    }
}
