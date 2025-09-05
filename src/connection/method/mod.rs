use std::{io, net::SocketAddr};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use tracing::{debug, error, warn};

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

pub struct MethodHandler;

impl MethodHandler {
    pub fn negotiate(client_methods: &[u8], server_methods: &[u8]) -> Option<Method> {
        debug!(
            "Negotiating methods - Client: {:?}, Server: {:?}",
            client_methods, server_methods
        );

        let method_priority = [
            Method::NO_AUTHENTICATION_REQUIRED,
            Method::USERNAME_PASSWORD,
            Method::GSSAPI,
            Method::IANA_ASSIGNED,
            Method::RESERVED_FOR_PRIVATE_METHODS,
        ];

        for &method_code in method_priority
            .iter()
            .map(|m| *m as u8)
            .collect::<Vec<_>>()
            .iter()
        {
            if server_methods.contains(&method_code) && client_methods.contains(&method_code) {
                if let Some(method) = Method::from_u8(method_code) {
                    if method.is_implemented() {
                        debug!(
                            "Negotiated method: {} (0x{:02X})",
                            method.display_name(),
                            method_code
                        );
                        return Some(method);
                    } else {
                        warn!(
                            "Method {} is not implemented, skipping",
                            method.display_name()
                        );
                    }
                }
            }
        }

        warn!("No mutually supported authentication methods found");
        None
    }

    pub async fn handle_client_methods<W>(
        client_methods: &[u8],
        server_methods: &[u8],
        writer: &mut BufWriter<W>,
        client_addr: SocketAddr,
    ) -> io::Result<Method>
    where
        W: AsyncWrite + Unpin,
    {
        debug!(
            "Handling client methods for {}: {:?}",
            client_addr, client_methods
        );

        match Self::negotiate(client_methods, server_methods) {
            Some(method) => {
                debug!(
                    "Selected method {} for client {}",
                    method.display_name(),
                    client_addr
                );

                let response = [SOCKS5_VERSION, method as u8];
                writer.write_all(&response).await?;
                writer.flush().await?;

                Self::authenticate_method(method, writer, client_addr).await?;

                Ok(method)
            }
            None => {
                error!(
                    "No acceptable authentication methods for client {}",
                    client_addr
                );

                let response = [SOCKS5_VERSION, Method::NO_ACCEPTABLE_METHODS];
                writer.write_all(&response).await?;
                writer.flush().await?;

                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "No acceptable authentication methods",
                ))
            }
        }
    }

    async fn authenticate_method<W>(
        method: Method,
        writer: &mut BufWriter<W>,
        client_addr: SocketAddr,
    ) -> io::Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        match method {
            Method::NoAuthenticationRequired => {
                debug!("No authentication required for client {}", client_addr);
                Ok(())
            }
            Method::UsernamePassword => {
                debug!(
                    "Username/password authentication for client {}",
                    client_addr
                );
                Self::handle_username_password_auth(writer, client_addr).await
            }
            Method::Gssapi => {
                debug!("GSSAPI authentication for client {}", client_addr);
                Self::handle_gssapi_auth(writer, client_addr).await
            }
            _ => {
                error!(
                    "Authentication method {} not implemented",
                    method.display_name()
                );
                Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    format!(
                        "Authentication method {} not implemented",
                        method.display_name()
                    ),
                ))
            }
        }
    }

    async fn handle_username_password_auth<W>(
        _writer: &mut BufWriter<W>,
        client_addr: SocketAddr,
    ) -> io::Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        warn!(
            "Username/password authentication not yet implemented for client {}",
            client_addr
        );
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Username/password authentication not implemented",
        ))
    }

    async fn handle_gssapi_auth<W>(
        _writer: &mut BufWriter<W>,
        client_addr: SocketAddr,
    ) -> io::Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        warn!(
            "GSSAPI authentication not yet implemented for client {}",
            client_addr
        );
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "GSSAPI authentication not implemented",
        ))
    }

    pub async fn parse_client_greeting<R>(reader: &mut BufReader<R>) -> io::Result<ClientGreeting>
    where
        R: AsyncRead + Unpin,
    {
        let version = reader.read_u8().await?;
        if version != SOCKS5_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid SOCKS version: expected {}, got {}",
                    SOCKS5_VERSION, version
                ),
            ));
        }

        let nmethods = reader.read_u8().await?;
        if nmethods == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "No authentication methods provided",
            ));
        }

        let mut methods = vec![0u8; nmethods as usize];
        reader.read_exact(&mut methods).await?;

        Ok(ClientGreeting {
            version,
            nmethods,
            methods,
        })
    }

    pub fn validate_client_methods(methods: &[u8]) -> Result<(), String> {
        if methods.is_empty() {
            return Err("No authentication methods provided".to_string());
        }

        if methods.len() > 255 {
            return Err("Too many authentication methods".to_string());
        }

        let mut sorted_methods = methods.to_vec();
        sorted_methods.sort_unstable();
        sorted_methods.dedup();
        if sorted_methods.len() != methods.len() {
            warn!("Client provided duplicate authentication methods");
        }

        for &method in methods {
            if Method::from_u8(method).is_none() {
                warn!("Unknown authentication method: 0x{:02X}", method);
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ClientGreeting {
    pub version: u8,
    pub nmethods: u8,
    pub methods: Vec<u8>,
}

impl ClientGreeting {
    pub fn get_supported_methods(&self) -> Vec<Method> {
        self.methods
            .iter()
            .filter_map(|&m| Method::from_u8(m))
            .collect()
    }

    pub fn supports_method(&self, method: Method) -> bool {
        self.methods.contains(&(method as u8))
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.version != SOCKS5_VERSION {
            return Err(format!("Invalid SOCKS version: {}", self.version));
        }

        if self.nmethods as usize != self.methods.len() {
            return Err(format!(
                "Method count mismatch: declared {}, actual {}",
                self.nmethods,
                self.methods.len()
            ));
        }

        MethodHandler::validate_client_methods(&self.methods)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncWriteExt, duplex};

    #[test]
    fn test_method_from_u8() {
        assert_eq!(
            Method::from_u8(0x00),
            Some(Method::NoAuthenticationRequired)
        );
        assert_eq!(Method::from_u8(0x01), Some(Method::Gssapi));
        assert_eq!(Method::from_u8(0x02), Some(Method::UsernamePassword));
        assert_eq!(Method::from_u8(0xFF), Some(Method::NoAcceptableMethods));
        assert_eq!(Method::from_u8(0x99), None);
    }

    #[test]
    fn test_method_properties() {
        assert!(!Method::NoAuthenticationRequired.requires_auth());
        assert!(Method::UsernamePassword.requires_auth());
        assert!(Method::Gssapi.requires_auth());

        assert!(Method::NoAuthenticationRequired.is_implemented());
        assert!(!Method::UsernamePassword.is_implemented());
        assert!(!Method::Gssapi.is_implemented());
    }

    #[test]
    fn test_method_negotiation() {
        let client_methods = vec![0x00, 0x02];
        let server_methods = vec![0x00];

        let result = MethodHandler::negotiate(&client_methods, &server_methods);
        assert_eq!(result, Some(Method::NoAuthenticationRequired));
    }

    #[test]
    fn test_method_negotiation_no_match() {
        let client_methods = vec![0x01, 0x02];
        let server_methods = vec![0x00];

        let result = MethodHandler::negotiate(&client_methods, &server_methods);
        assert_eq!(result, None);
    }

    #[test]
    fn test_method_negotiation_priority() {
        let client_methods = vec![0x02, 0x00, 0x01];
        let server_methods = vec![0x02, 0x00, 0x01];

        // Should prefer NO_AUTHENTICATION_REQUIRED (0x00) due to priority
        let result = MethodHandler::negotiate(&client_methods, &server_methods);
        assert_eq!(result, Some(Method::NoAuthenticationRequired));
    }

    #[tokio::test]
    async fn test_parse_client_greeting_valid() {
        let (mut client, server) = duplex(1024);

        // Send valid greeting: VER=5, NMETHODS=2, METHODS=[0x00, 0x02]
        client.write_all(&[0x05, 0x02, 0x00, 0x02]).await.unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let greeting = MethodHandler::parse_client_greeting(&mut reader)
            .await
            .expect("Should parse valid greeting");

        assert_eq!(greeting.version, SOCKS5_VERSION);
        assert_eq!(greeting.nmethods, 2);
        assert_eq!(greeting.methods, vec![0x00, 0x02]);
        assert!(greeting.supports_method(Method::NoAuthenticationRequired));
        assert!(greeting.supports_method(Method::UsernamePassword));
    }

    #[tokio::test]
    async fn test_parse_client_greeting_invalid_version() {
        let (mut client, server) = duplex(1024);

        // Send invalid version
        client.write_all(&[0x04, 0x01, 0x00]).await.unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let result = MethodHandler::parse_client_greeting(&mut reader).await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid SOCKS version")
        );
    }

    #[tokio::test]
    async fn test_parse_client_greeting_no_methods() {
        let (mut client, server) = duplex(1024);

        // Send zero methods
        client.write_all(&[0x05, 0x00]).await.unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(server);
        let result = MethodHandler::parse_client_greeting(&mut reader).await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No authentication methods")
        );
    }

    #[test]
    fn test_client_greeting_validation() {
        let greeting = ClientGreeting {
            version: SOCKS5_VERSION,
            nmethods: 2,
            methods: vec![0x00, 0x02],
        };

        assert!(greeting.validate().is_ok());

        let invalid_greeting = ClientGreeting {
            version: 0x04,
            nmethods: 1,
            methods: vec![0x00],
        };

        assert!(invalid_greeting.validate().is_err());
    }

    #[test]
    fn test_validate_client_methods() {
        assert!(MethodHandler::validate_client_methods(&[0x00]).is_ok());
        assert!(MethodHandler::validate_client_methods(&[0x00, 0x02]).is_ok());
        assert!(MethodHandler::validate_client_methods(&[]).is_err());
    }
}
