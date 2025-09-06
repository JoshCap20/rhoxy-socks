#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Reply {
    Success = 0x00,
    GeneralFailure = 0x01,
    ConnectionNotAllowed = 0x02,
    NetworkUnreachable = 0x03,
    HostUnreachable = 0x04,
    ConnectionRefused = 0x05,
    TtlExpired = 0x06,
    CommandNotSupported = 0x07,
    AddressTypeNotSupported = 0x08,
}

impl Reply {
    pub const SUCCESS: u8 = Self::Success as u8;
    pub const GENERAL_FAILURE: u8 = Self::GeneralFailure as u8;
    pub const CONNECTION_NOT_ALLOWED: u8 = Self::ConnectionNotAllowed as u8;
    pub const NETWORK_UNREACHABLE: u8 = Self::NetworkUnreachable as u8;
    pub const HOST_UNREACHABLE: u8 = Self::HostUnreachable as u8;
    pub const CONNECTION_REFUSED: u8 = Self::ConnectionRefused as u8;
    pub const TTL_EXPIRED: u8 = Self::TtlExpired as u8;
    pub const COMMAND_NOT_SUPPORTED: u8 = Self::CommandNotSupported as u8;
    pub const ADDRESS_TYPE_NOT_SUPPORTED: u8 = Self::AddressTypeNotSupported as u8;

    pub fn from_u8(value: u8) -> Option<Reply> {
        match value {
            Self::SUCCESS => Some(Reply::Success),
            Self::GENERAL_FAILURE => Some(Reply::GeneralFailure),
            Self::CONNECTION_NOT_ALLOWED => Some(Reply::ConnectionNotAllowed),
            Self::NETWORK_UNREACHABLE => Some(Reply::NetworkUnreachable),
            Self::HOST_UNREACHABLE => Some(Reply::HostUnreachable),
            Self::CONNECTION_REFUSED => Some(Reply::ConnectionRefused),
            Self::TTL_EXPIRED => Some(Reply::TtlExpired),
            Self::COMMAND_NOT_SUPPORTED => Some(Reply::CommandNotSupported),
            Self::ADDRESS_TYPE_NOT_SUPPORTED => Some(Reply::AddressTypeNotSupported),
            _ => None,
        }
    }

    pub fn is_success(self) -> bool {
        matches!(self, Reply::Success)
    }

    pub fn is_error(self) -> bool {
        !self.is_success()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reply_constants() {
        assert_eq!(Reply::SUCCESS, 0x00);
        assert_eq!(Reply::GENERAL_FAILURE, 0x01);
        assert_eq!(Reply::CONNECTION_NOT_ALLOWED, 0x02);
        assert_eq!(Reply::NETWORK_UNREACHABLE, 0x03);
        assert_eq!(Reply::HOST_UNREACHABLE, 0x04);
        assert_eq!(Reply::CONNECTION_REFUSED, 0x05);
        assert_eq!(Reply::TTL_EXPIRED, 0x06);
        assert_eq!(Reply::COMMAND_NOT_SUPPORTED, 0x07);
        assert_eq!(Reply::ADDRESS_TYPE_NOT_SUPPORTED, 0x08);
    }

    #[test]
    fn test_reply_enum_values() {
        assert_eq!(Reply::Success as u8, 0x00);
        assert_eq!(Reply::GeneralFailure as u8, 0x01);
        assert_eq!(Reply::ConnectionNotAllowed as u8, 0x02);
        assert_eq!(Reply::NetworkUnreachable as u8, 0x03);
        assert_eq!(Reply::HostUnreachable as u8, 0x04);
        assert_eq!(Reply::ConnectionRefused as u8, 0x05);
        assert_eq!(Reply::TtlExpired as u8, 0x06);
        assert_eq!(Reply::CommandNotSupported as u8, 0x07);
        assert_eq!(Reply::AddressTypeNotSupported as u8, 0x08);
    }

    #[test]
    fn test_from_u8_valid() {
        assert_eq!(Reply::from_u8(0x00), Some(Reply::Success));
        assert_eq!(Reply::from_u8(0x01), Some(Reply::GeneralFailure));
        assert_eq!(Reply::from_u8(0x02), Some(Reply::ConnectionNotAllowed));
        assert_eq!(Reply::from_u8(0x03), Some(Reply::NetworkUnreachable));
        assert_eq!(Reply::from_u8(0x04), Some(Reply::HostUnreachable));
        assert_eq!(Reply::from_u8(0x05), Some(Reply::ConnectionRefused));
        assert_eq!(Reply::from_u8(0x06), Some(Reply::TtlExpired));
        assert_eq!(Reply::from_u8(0x07), Some(Reply::CommandNotSupported));
        assert_eq!(Reply::from_u8(0x08), Some(Reply::AddressTypeNotSupported));
    }

    #[test]
    fn test_from_u8_invalid() {
        assert_eq!(Reply::from_u8(0x09), None);
        assert_eq!(Reply::from_u8(0xFF), None);
        assert_eq!(Reply::from_u8(0x10), None);
        assert_eq!(Reply::from_u8(0x80), None);
    }

    #[test]
    fn test_is_success() {
        assert!(Reply::Success.is_success());
        assert!(!Reply::GeneralFailure.is_success());
        assert!(!Reply::ConnectionNotAllowed.is_success());
        assert!(!Reply::NetworkUnreachable.is_success());
        assert!(!Reply::HostUnreachable.is_success());
        assert!(!Reply::ConnectionRefused.is_success());
        assert!(!Reply::TtlExpired.is_success());
        assert!(!Reply::CommandNotSupported.is_success());
        assert!(!Reply::AddressTypeNotSupported.is_success());
    }

    #[test]
    fn test_is_error() {
        assert!(!Reply::Success.is_error());
        assert!(Reply::GeneralFailure.is_error());
        assert!(Reply::ConnectionNotAllowed.is_error());
        assert!(Reply::NetworkUnreachable.is_error());
        assert!(Reply::HostUnreachable.is_error());
        assert!(Reply::ConnectionRefused.is_error());
        assert!(Reply::TtlExpired.is_error());
        assert!(Reply::CommandNotSupported.is_error());
        assert!(Reply::AddressTypeNotSupported.is_error());
    }

    #[test]
    fn test_reply_equality() {
        assert_eq!(Reply::Success, Reply::Success);
        assert_ne!(Reply::Success, Reply::GeneralFailure);
        assert_ne!(Reply::ConnectionRefused, Reply::NetworkUnreachable);
    }

    #[test]
    fn test_reply_debug() {
        let reply = Reply::Success;
        let debug_str = format!("{:?}", reply);
        assert_eq!(debug_str, "Success");
        
        let reply = Reply::ConnectionRefused;
        let debug_str = format!("{:?}", reply);
        assert_eq!(debug_str, "ConnectionRefused");
    }

    #[test]
    fn test_reply_clone() {
        let original = Reply::NetworkUnreachable;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_reply_copy() {
        let original = Reply::HostUnreachable;
        let copied = original;
        assert_eq!(original, copied);
    }

    #[test]
    fn test_all_reply_codes_roundtrip() {
        let replies = [
            Reply::Success,
            Reply::GeneralFailure,
            Reply::ConnectionNotAllowed,
            Reply::NetworkUnreachable,
            Reply::HostUnreachable,
            Reply::ConnectionRefused,
            Reply::TtlExpired,
            Reply::CommandNotSupported,
            Reply::AddressTypeNotSupported,
        ];

        for reply in replies {
            let value = reply as u8;
            let converted = Reply::from_u8(value).expect("Should convert back successfully");
            assert_eq!(reply, converted);
        }
    }
}
