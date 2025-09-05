#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Method {
    NoAuthenticationRequired = 0x00,
    Gssapi = 0x01,
    UsernamePassword = 0x02,
    IanaAssigned = 0x03,
    ReservedForPrivateMethods = 0x80,
    NoAcceptableMethods = 0xFF,
}

impl Method {
    pub const NO_AUTHENTICATION_REQUIRED: u8 = Self::NoAuthenticationRequired as u8;
    pub const GSSAPI: u8 = Self::Gssapi as u8;
    pub const USERNAME_PASSWORD: u8 = Self::UsernamePassword as u8;
    pub const IANA_ASSIGNED: u8 = Self::IanaAssigned as u8;
    pub const RESERVED_FOR_PRIVATE_METHODS: u8 = Self::ReservedForPrivateMethods as u8;
    pub const NO_ACCEPTABLE_METHODS: u8 = Self::NoAcceptableMethods as u8;
}