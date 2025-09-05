use std::io;

use tokio::io::{AsyncWrite, AsyncWriteExt, BufWriter};

use crate::connection::SOCKS5_VERSION;

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

    pub fn handle_methods(&self, client_methods: &[u8], server_methods: &[u8]) -> io::Result<()> {
        match Self::negotiate(client_methods, server_methods) {
            Some(method) => {
                let method = Method::from_u8(method).unwrap();
                method.to_handler(&mut BufWriter::new(Box::new(std::io::stdout()))).await
            }
            None => {
                let response = [SOCKS5_VERSION, Method::NO_ACCEPTABLE_METHODS];
                let mut writer = BufWriter::new(Box::new(std::io::stdout()) as Box<dyn AsyncWrite + Unpin>);
                writer.write_all(&response).await?;
                writer.flush().await?;
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "No acceptable authentication methods",
                ))
            }
        }
    }

    async fn handle_no_auth<W>(&self, writer: &mut BufWriter<W>) -> io::Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        let response = [SOCKS5_VERSION, Method::NO_AUTHENTICATION_REQUIRED];
        writer.write_all(&response).await?;
        writer.flush().await?;

        Ok(())
    }

    fn negotiate(
        client_methods: &[u8],
        server_methods: &[u8],
    ) -> Option<u8> {
        // TODO: Support preferred methods
        // i.e. no auth and username_password should use no auth
        for &method in server_methods {
            if client_methods.contains(&method) {
                return Some(method);
            }
        }
        None
    }

    fn from_u8(value: u8) -> Option<Method> {
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

    async fn to_handler<'a>(&self, writer: &'a mut BufWriter<Box<dyn AsyncWrite + Unpin>>) -> io::Result<()> {
        match self {
            Method::NoAuthenticationRequired => Self::handle_no_auth(self, writer).await,
            _ => unimplemented!(),
        }
    }
}