use std::{io, net::SocketAddr};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use tracing::{debug, error, warn};

use crate::connection::{
    SOCKS5_VERSION,
    method::{client_greeting::ClientGreeting, method::Method},
};

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
