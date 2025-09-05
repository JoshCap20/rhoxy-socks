use std::{net::{SocketAddr, ToSocketAddrs}, time::Duration};

use clap::Parser;

use crate::connection::Method;

#[derive(Parser, Debug, Clone)]
#[command(version, about = "SOCKS5 proxy", long_about = None)]
pub struct ProxyConfig {
    #[arg(long, default_value = "localhost", help = "Host address to bind to")]
    pub host: String,

    #[arg(short, long, default_value = "1080", help = "Port to listen on")]
    pub port: u16,

    #[arg(long, help = "Enable debug logging")]
    pub verbose: bool,

    #[arg(long, default_value = "1000", help = "Maximum concurrent connections")]
    pub max_connections: usize,

    #[arg(long, default_value = "60", help = "Connection timeout in seconds")]
    pub connection_timeout: u64,

    #[arg(
        long,
        default_value = "32",
        help = "Buffer size for data transfers in KB"
    )]
    pub buffer_size: usize,

    #[arg(
        long,
        default_value = "true",
        help = "Enable TCP_NODELAY for low latency"
    )]
    pub tcp_nodelay: bool,

    // Not implemented yet
    // #[arg(
    //     long,
    //     default_value = "60",
    //     help = "TCP keep-alive timeout in seconds (0 to disable)"
    // )]
    // pub keep_alive: u64,

    #[arg(long, help = "Enable detailed connection metrics")]
    pub metrics: bool,

    // Not implemented yet
    // #[arg(long, help = "Local address to bind outgoing connections to")]
    // pub bind_addr: Option<String>,

    #[arg(
        long,
        default_value = "none",
        help = "Comma-separated list of auth methods: none,userpass,gssapi"
    )]
    pub auth_methods: String,
}

impl ProxyConfig {
    pub fn from_args() -> Self {
        Self::parse()
    }

    pub fn server_addr(&self) -> Result<SocketAddr, std::io::Error> {
        if let Ok(addr) = format!("{}:{}", self.host, self.port).parse::<SocketAddr>() {
            return Ok(addr);
        }

        let addr_str = format!("{}:{}", self.host, self.port);

        match addr_str.to_socket_addrs() {
            Ok(mut addrs) => addrs.next().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                    format!("No addresses found for '{}'", addr_str),
                )
            }),
            Err(e) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Failed to resolve server address '{}': {}", addr_str, e),
            )),
        }
    }

    pub fn connection_timeout_duration(&self) -> Duration {
        Duration::from_secs(self.connection_timeout)
    }

    pub fn keep_alive_duration(&self) -> Option<Duration> {
        // if self.keep_alive > 0 {
        //     Some(Duration::from_secs(self.keep_alive))
        // } else {
        //     None
        // }
        None
    }

    pub fn buffer_size_bytes(&self) -> usize {
        self.buffer_size * 1024
    }

    pub fn bind_address(&self) -> Option<SocketAddr> {
        // self.bind_addr.as_ref().and_then(|addr| addr.parse().ok())
        None
    }

    pub fn supported_auth_methods(&self) -> Vec<u8> {
        let mut methods = Vec::new();

        for method in self.auth_methods.split(',') {
            match method.trim().to_lowercase().as_str() {
                "none" => methods.push(Method::NO_AUTHENTICATION_REQUIRED),
                invalid => {
                    eprintln!("Warning: ignoring invalid auth method '{}'", invalid);
                }
            }
        }

        if methods.is_empty() {
            methods.push(0x00);
        }

        methods
    }

    pub fn tracing_level(&self) -> tracing::Level {
        if self.verbose {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.port == 0 {
            return Err("Port cannot be 0".to_string());
        }

        if self.max_connections == 0 {
            return Err("Max connections must be greater than 0".to_string());
        }

        if self.buffer_size == 0 {
            return Err("Buffer size must be greater than 0".to_string());
        }

        if self.buffer_size > 1024 {
            return Err("Buffer size cannot exceed 1024 KB".to_string());
        }

        // if let Some(ref addr) = self.bind_addr {
        //     addr.parse::<SocketAddr>()
        //         .map_err(|e| format!("Invalid bind address '{}': {}", addr, e))?;
        // }

        let methods = self.supported_auth_methods();
        if methods.is_empty() {
            return Err("At least one authentication method must be supported".to_string());
        }

        Ok(())
    }

    pub fn display_summary(&self) {
        println!("Rhoxy SOCKS5 Proxy Configuration:");
        println!("   Server Address:      {}:{}", self.host, self.port);
        println!("   Max Connections:     {}", self.max_connections);
        println!("   Connection Timeout:  {}s", self.connection_timeout);
        println!("   Buffer Size:         {}KB", self.buffer_size);
        println!("   TCP_NODELAY:         {}", self.tcp_nodelay);
        // println!(
        //     "   Keep-Alive:          {}s",
        //     if self.keep_alive > 0 {
        //         self.keep_alive.to_string()
        //     } else {
        //         "disabled".to_string()
        //     }
        // );
        println!("   Auth Methods:        {}", self.auth_methods);
        println!("   Metrics Enabled:     {}", self.metrics);
        println!("   Debug Logging:       {}", self.verbose);

        // if let Some(ref addr) = self.bind_addr {
        //     println!("   Bind Address:        {}", addr);
        // }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub buffer_size: usize,
    pub tcp_nodelay: bool,
    pub keep_alive: Option<Duration>,
    pub connection_timeout: Duration,
    pub bind_addr: Option<SocketAddr>,
    pub metrics_enabled: bool,
}

impl From<&ProxyConfig> for ConnectionConfig {
    fn from(config: &ProxyConfig) -> Self {
        Self {
            buffer_size: config.buffer_size_bytes(),
            tcp_nodelay: config.tcp_nodelay,
            keep_alive: config.keep_alive_duration(),
            connection_timeout: config.connection_timeout_duration(),
            bind_addr: config.bind_address(),
            metrics_enabled: config.metrics,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_validation() {
        let config = ProxyConfig {
            host: "localhost".to_string(),
            port: 1080,
            verbose: false,
            max_connections: 1000,
            connection_timeout: 30,
            buffer_size: 32,
            tcp_nodelay: true,
            // keep_alive: 60,
            metrics: false,
            // bind_addr: None,
            auth_methods: "none".to_string(),
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_port() {
        let config = ProxyConfig {
            host: "localhost".to_string(),
            port: 0,
            verbose: false,
            max_connections: 1000,
            connection_timeout: 30,
            buffer_size: 32,
            tcp_nodelay: true,
            // keep_alive: 60,
            metrics: false,
            // bind_addr: None,
            auth_methods: "none".to_string(),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_auth_methods_parsing() {
        let config = ProxyConfig {
            host: "localhost".to_string(),
            port: 1080,
            verbose: false,
            max_connections: 1000,
            connection_timeout: 30,
            buffer_size: 32,
            tcp_nodelay: true,
            // keep_alive: 60,
            metrics: false,
            // bind_addr: None,
            auth_methods: "none".to_string(),
        };

        let methods = config.supported_auth_methods();
        assert!(methods.contains(&Method::NO_AUTHENTICATION_REQUIRED));
    }

    #[test]
    fn test_connection_config_conversion() {
        let proxy_config = ProxyConfig {
            host: "localhost".to_string(),
            port: 1080,
            verbose: false,
            max_connections: 1000,
            connection_timeout: 30,
            buffer_size: 32,
            tcp_nodelay: true,
            // keep_alive: 60,
            metrics: false,
            // bind_addr: None,
            auth_methods: "none".to_string(),
        };

        let conn_config = ConnectionConfig::from(&proxy_config);
        assert_eq!(conn_config.buffer_size, 32 * 1024);
        assert_eq!(conn_config.connection_timeout, Duration::from_secs(30));
        // assert_eq!(conn_config.keep_alive, Some(Duration::from_secs(60)));
    }

    #[test]
    fn test_server_addr_parsing() {
        let config = ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            verbose: false,
            max_connections: 1000,
            connection_timeout: 30,
            buffer_size: 32,
            tcp_nodelay: true,
            // keep_alive: 60,
            metrics: false,
            // bind_addr: None,
            auth_methods: "none".to_string(),
        };

        let addr = config.server_addr().unwrap();
        assert_eq!(addr.port(), 8080);
    }
}
