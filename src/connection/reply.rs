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
}
