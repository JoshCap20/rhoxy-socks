pub mod handshake;
pub mod request;
pub mod command;

pub const SOCKS5_VERSION: u8 = 0x05;
pub const ATYP_IPV4: u8 = 0x01;
pub const ATYP_DOMAIN: u8 = 0x03;
pub const ATYP_IPV6: u8 = 0x04;