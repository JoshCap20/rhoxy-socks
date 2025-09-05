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
            SocksError::DnsResolutionFailed => io::Error::other("DNS resolution failed"),
            SocksError::NoAddressesResolved => io::Error::other("No addresses resolved for domain"),
            SocksError::ConnectionFailed(kind) => io::Error::new(*kind, "Connection failed"),
            SocksError::InvalidData => io::Error::new(io::ErrorKind::InvalidData, "Invalid data"),
            SocksError::IoError(kind) => io::Error::new(*kind, "IO error"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_socks_error_clone() {
        let error = SocksError::InvalidVersion(4);
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    #[test]
    fn test_socks_error_debug() {
        let error = SocksError::InvalidVersion(4);
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("InvalidVersion"));
        assert!(debug_str.contains("4"));
    }

    #[test]
    fn test_socks_error_partial_eq() {
        assert_eq!(SocksError::InvalidVersion(4), SocksError::InvalidVersion(4));
        assert_ne!(SocksError::InvalidVersion(4), SocksError::InvalidVersion(5));
        assert_ne!(SocksError::InvalidVersion(4), SocksError::EmptyDomainName);
    }

    mod to_reply_code_tests {
        use super::*;

        #[test]
        fn test_invalid_version_to_reply_code() {
            let error = SocksError::InvalidVersion(4);
            assert_eq!(error.to_reply_code(), Reply::GENERAL_FAILURE);
        }

        #[test]
        fn test_invalid_reserved_byte_to_reply_code() {
            let error = SocksError::InvalidReservedByte(0xFF);
            assert_eq!(error.to_reply_code(), Reply::GENERAL_FAILURE);
        }

        #[test]
        fn test_unsupported_address_type_to_reply_code() {
            let error = SocksError::UnsupportedAddressType(0xFF);
            assert_eq!(error.to_reply_code(), Reply::ADDRESS_TYPE_NOT_SUPPORTED);
        }

        #[test]
        fn test_unsupported_command_to_reply_code() {
            let error = SocksError::UnsupportedCommand(0xFF);
            assert_eq!(error.to_reply_code(), Reply::COMMAND_NOT_SUPPORTED);
        }

        #[test]
        fn test_empty_domain_name_to_reply_code() {
            let error = SocksError::EmptyDomainName;
            assert_eq!(error.to_reply_code(), Reply::GENERAL_FAILURE);
        }

        #[test]
        fn test_invalid_domain_name_encoding_to_reply_code() {
            let error = SocksError::InvalidDomainNameEncoding;
            assert_eq!(error.to_reply_code(), Reply::GENERAL_FAILURE);
        }

        #[test]
        fn test_dns_resolution_failed_to_reply_code() {
            let error = SocksError::DnsResolutionFailed;
            assert_eq!(error.to_reply_code(), Reply::HOST_UNREACHABLE);
        }

        #[test]
        fn test_no_addresses_resolved_to_reply_code() {
            let error = SocksError::NoAddressesResolved;
            assert_eq!(error.to_reply_code(), Reply::HOST_UNREACHABLE);
        }

        #[test]
        fn test_connection_failed_connection_refused_to_reply_code() {
            let error = SocksError::ConnectionFailed(io::ErrorKind::ConnectionRefused);
            assert_eq!(error.to_reply_code(), Reply::CONNECTION_REFUSED);
        }

        #[test]
        fn test_connection_failed_timed_out_to_reply_code() {
            let error = SocksError::ConnectionFailed(io::ErrorKind::TimedOut);
            assert_eq!(error.to_reply_code(), Reply::HOST_UNREACHABLE);
        }

        #[test]
        fn test_connection_failed_addr_not_available_to_reply_code() {
            let error = SocksError::ConnectionFailed(io::ErrorKind::AddrNotAvailable);
            assert_eq!(error.to_reply_code(), Reply::HOST_UNREACHABLE);
        }

        #[test]
        fn test_connection_failed_network_unreachable_to_reply_code() {
            let error = SocksError::ConnectionFailed(io::ErrorKind::NetworkUnreachable);
            assert_eq!(error.to_reply_code(), Reply::NETWORK_UNREACHABLE);
        }

        #[test]
        fn test_connection_failed_permission_denied_to_reply_code() {
            let error = SocksError::ConnectionFailed(io::ErrorKind::PermissionDenied);
            assert_eq!(error.to_reply_code(), Reply::CONNECTION_NOT_ALLOWED);
        }

        #[test]
        fn test_connection_failed_other_error_kinds_to_reply_code() {
            let error_kinds = [
                io::ErrorKind::NotFound,
                io::ErrorKind::InvalidInput,
                io::ErrorKind::InvalidData,
                io::ErrorKind::BrokenPipe,
                io::ErrorKind::AlreadyExists,
            ];

            for kind in error_kinds.iter() {
                let error = SocksError::ConnectionFailed(*kind);
                assert_eq!(error.to_reply_code(), Reply::GENERAL_FAILURE);
            }
        }

        #[test]
        fn test_invalid_data_to_reply_code() {
            let error = SocksError::InvalidData;
            assert_eq!(error.to_reply_code(), Reply::GENERAL_FAILURE);
        }

        #[test]
        fn test_io_error_to_reply_code() {
            let error = SocksError::IoError(io::ErrorKind::UnexpectedEof);
            assert_eq!(error.to_reply_code(), Reply::GENERAL_FAILURE);
        }
    }

    mod to_io_error_tests {
        use super::*;

        #[test]
        fn test_invalid_version_to_io_error() {
            let error = SocksError::InvalidVersion(4);
            let io_error = error.to_io_error();
            assert_eq!(io_error.kind(), io::ErrorKind::InvalidData);
            assert!(io_error.to_string().contains("Invalid SOCKS version: 4"));
        }

        #[test]
        fn test_invalid_reserved_byte_to_io_error() {
            let error = SocksError::InvalidReservedByte(0xFF);
            let io_error = error.to_io_error();
            assert_eq!(io_error.kind(), io::ErrorKind::InvalidData);
            assert!(io_error.to_string().contains("Invalid reserved byte: 255"));
        }

        #[test]
        fn test_unsupported_address_type_to_io_error() {
            let error = SocksError::UnsupportedAddressType(0xFF);
            let io_error = error.to_io_error();
            assert_eq!(io_error.kind(), io::ErrorKind::InvalidData);
            assert!(
                io_error
                    .to_string()
                    .contains("Unsupported address type: 255")
            );
        }

        #[test]
        fn test_unsupported_command_to_io_error() {
            let error = SocksError::UnsupportedCommand(0xFF);
            let io_error = error.to_io_error();
            assert_eq!(io_error.kind(), io::ErrorKind::InvalidData);
            assert!(io_error.to_string().contains("Unsupported command: 255"));
        }

        #[test]
        fn test_empty_domain_name_to_io_error() {
            let error = SocksError::EmptyDomainName;
            let io_error = error.to_io_error();
            assert_eq!(io_error.kind(), io::ErrorKind::InvalidData);
            assert!(io_error.to_string().contains("Empty domain name"));
        }

        #[test]
        fn test_invalid_domain_name_encoding_to_io_error() {
            let error = SocksError::InvalidDomainNameEncoding;
            let io_error = error.to_io_error();
            assert_eq!(io_error.kind(), io::ErrorKind::InvalidData);
            assert!(
                io_error
                    .to_string()
                    .contains("Invalid domain name encoding")
            );
        }

        #[test]
        fn test_dns_resolution_failed_to_io_error() {
            let error = SocksError::DnsResolutionFailed;
            let io_error = error.to_io_error();
            assert_eq!(io_error.kind(), io::ErrorKind::Other);
            assert!(io_error.to_string().contains("DNS resolution failed"));
        }

        #[test]
        fn test_no_addresses_resolved_to_io_error() {
            let error = SocksError::NoAddressesResolved;
            let io_error = error.to_io_error();
            assert_eq!(io_error.kind(), io::ErrorKind::Other);
            assert!(
                io_error
                    .to_string()
                    .contains("No addresses resolved for domain")
            );
        }

        #[test]
        fn test_connection_failed_to_io_error() {
            let original_kind = io::ErrorKind::ConnectionRefused;
            let error = SocksError::ConnectionFailed(original_kind);
            let io_error = error.to_io_error();
            assert_eq!(io_error.kind(), original_kind);
            assert!(io_error.to_string().contains("Connection failed"));
        }

        #[test]
        fn test_invalid_data_to_io_error() {
            let error = SocksError::InvalidData;
            let io_error = error.to_io_error();
            assert_eq!(io_error.kind(), io::ErrorKind::InvalidData);
            assert!(io_error.to_string().contains("Invalid data"));
        }

        #[test]
        fn test_io_error_to_io_error() {
            let original_kind = io::ErrorKind::UnexpectedEof;
            let error = SocksError::IoError(original_kind);
            let io_error = error.to_io_error();
            assert_eq!(io_error.kind(), original_kind);
            assert!(io_error.to_string().contains("IO error"));
        }

        #[test]
        fn test_different_error_kinds_preserved() {
            let error_kinds = [
                io::ErrorKind::NotFound,
                io::ErrorKind::PermissionDenied,
                io::ErrorKind::ConnectionRefused,
                io::ErrorKind::TimedOut,
                io::ErrorKind::InvalidInput,
                io::ErrorKind::InvalidData,
                io::ErrorKind::UnexpectedEof,
            ];

            for &kind in &error_kinds {
                let socks_error = SocksError::IoError(kind);
                let io_error = socks_error.to_io_error();
                assert_eq!(io_error.kind(), kind);

                let connection_error = SocksError::ConnectionFailed(kind);
                let connection_io_error = connection_error.to_io_error();
                assert_eq!(connection_io_error.kind(), kind);
            }
        }
    }

    mod comprehensive_tests {
        use super::*;

        #[test]
        fn test_all_error_variants_exist() {
            // Ensure all variants can be constructed and are distinct
            let errors = vec![
                SocksError::InvalidVersion(4),
                SocksError::InvalidReservedByte(0xFF),
                SocksError::UnsupportedAddressType(0xFF),
                SocksError::UnsupportedCommand(0xFF),
                SocksError::EmptyDomainName,
                SocksError::InvalidDomainNameEncoding,
                SocksError::DnsResolutionFailed,
                SocksError::NoAddressesResolved,
                SocksError::ConnectionFailed(io::ErrorKind::ConnectionRefused),
                SocksError::InvalidData,
                SocksError::IoError(io::ErrorKind::UnexpectedEof),
            ];

            // Each error should be unique
            for (i, error1) in errors.iter().enumerate() {
                for (j, error2) in errors.iter().enumerate() {
                    if i == j {
                        assert_eq!(error1, error2);
                    } else {
                        assert_ne!(error1, error2);
                    }
                }
            }
        }

        #[test]
        fn test_error_conversion_roundtrip_compatibility() {
            let errors = vec![
                SocksError::InvalidVersion(4),
                SocksError::InvalidReservedByte(0xFF),
                SocksError::UnsupportedAddressType(0xFF),
                SocksError::UnsupportedCommand(0xFF),
                SocksError::EmptyDomainName,
                SocksError::InvalidDomainNameEncoding,
                SocksError::DnsResolutionFailed,
                SocksError::NoAddressesResolved,
                SocksError::ConnectionFailed(io::ErrorKind::ConnectionRefused),
                SocksError::InvalidData,
                SocksError::IoError(io::ErrorKind::UnexpectedEof),
            ];

            for error in errors {
                // Each error should convert to a valid reply code
                let reply_code = error.to_reply_code();
                assert!(reply_code <= 0x08); // Valid SOCKS5 reply codes are 0x00-0x08

                // Each error should convert to a valid io::Error
                let io_error = error.to_io_error();
                assert!(!io_error.to_string().is_empty());
            }
        }

        #[test]
        fn test_boundary_values() {
            // Test boundary values for numeric variants
            let boundary_tests = vec![
                (SocksError::InvalidVersion(0), Reply::GENERAL_FAILURE),
                (SocksError::InvalidVersion(255), Reply::GENERAL_FAILURE),
                (SocksError::InvalidReservedByte(0), Reply::GENERAL_FAILURE),
                (SocksError::InvalidReservedByte(255), Reply::GENERAL_FAILURE),
                (
                    SocksError::UnsupportedAddressType(0),
                    Reply::ADDRESS_TYPE_NOT_SUPPORTED,
                ),
                (
                    SocksError::UnsupportedAddressType(255),
                    Reply::ADDRESS_TYPE_NOT_SUPPORTED,
                ),
                (
                    SocksError::UnsupportedCommand(0),
                    Reply::COMMAND_NOT_SUPPORTED,
                ),
                (
                    SocksError::UnsupportedCommand(255),
                    Reply::COMMAND_NOT_SUPPORTED,
                ),
            ];

            for (error, expected_reply) in boundary_tests {
                assert_eq!(error.to_reply_code(), expected_reply);
                // Ensure io::Error conversion works for boundary values
                let io_error = error.to_io_error();
                assert!(!io_error.to_string().is_empty());
            }
        }

        #[test]
        fn test_error_messages_are_descriptive() {
            let test_cases = vec![
                (
                    SocksError::InvalidVersion(4),
                    vec!["Invalid", "SOCKS", "version", "4"],
                ),
                (
                    SocksError::InvalidReservedByte(255),
                    vec!["Invalid", "reserved", "byte", "255"],
                ),
                (
                    SocksError::UnsupportedAddressType(10),
                    vec!["Unsupported", "address", "type", "10"],
                ),
                (
                    SocksError::UnsupportedCommand(99),
                    vec!["Unsupported", "command", "99"],
                ),
                (SocksError::EmptyDomainName, vec!["Empty", "domain", "name"]),
                (
                    SocksError::InvalidDomainNameEncoding,
                    vec!["Invalid", "domain", "name", "encoding"],
                ),
                (
                    SocksError::DnsResolutionFailed,
                    vec!["DNS", "resolution", "failed"],
                ),
                (
                    SocksError::NoAddressesResolved,
                    vec!["No", "addresses", "resolved"],
                ),
                (
                    SocksError::ConnectionFailed(io::ErrorKind::ConnectionRefused),
                    vec!["Connection", "failed"],
                ),
                (SocksError::InvalidData, vec!["Invalid", "data"]),
                (
                    SocksError::IoError(io::ErrorKind::UnexpectedEof),
                    vec!["IO", "error"],
                ),
            ];

            for (error, keywords) in test_cases {
                let message = error.to_io_error().to_string();
                for keyword in keywords {
                    assert!(
                        message.to_lowercase().contains(&keyword.to_lowercase()),
                        "Error message '{}' should contain keyword '{}'",
                        message,
                        keyword
                    );
                }
            }
        }
    }
}
