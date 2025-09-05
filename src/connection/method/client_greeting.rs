use crate::connection::{
    SOCKS5_VERSION,
    method::{method::Method, method_handler::MethodHandler},
};

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
