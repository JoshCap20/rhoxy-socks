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

    pub fn from_u8(value: u8) -> Option<Method> {
        match value {
            Self::NO_AUTHENTICATION_REQUIRED => Some(Method::NoAuthenticationRequired),
            Self::GSSAPI => Some(Method::Gssapi),
            Self::USERNAME_PASSWORD => Some(Method::UsernamePassword),
            Self::IANA_ASSIGNED => Some(Method::IanaAssigned),
            Self::RESERVED_FOR_PRIVATE_METHODS => Some(Method::ReservedForPrivateMethods),
            Self::NO_ACCEPTABLE_METHODS => Some(Method::NoAcceptableMethods),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Method::NoAuthenticationRequired => "No Authentication Required",
            Method::Gssapi => "GSSAPI",
            Method::UsernamePassword => "Username/Password",
            Method::IanaAssigned => "IANA Assigned",
            Method::ReservedForPrivateMethods => "Reserved for Private Methods",
            Method::NoAcceptableMethods => "No Acceptable Methods",
        }
    }

    pub fn requires_auth(&self) -> bool {
        match self {
            Method::NoAuthenticationRequired => false,
            Method::Gssapi | Method::UsernamePassword => true,
            Method::IanaAssigned | Method::ReservedForPrivateMethods => true,
            Method::NoAcceptableMethods => false,
        }
    }

    pub fn is_implemented(&self) -> bool {
        matches!(self, Method::NoAuthenticationRequired)
    }
}
