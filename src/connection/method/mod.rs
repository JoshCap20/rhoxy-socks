pub mod client_greeting;
pub mod method;
pub mod method_handler;

#[cfg(test)]
mod tests {
    use crate::connection::{
        SOCKS5_VERSION,
        method::{client_greeting::ClientGreeting, method::Method, method_handler::MethodHandler},
    };

    use tokio::io::{AsyncWriteExt, BufReader, duplex};

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
