use std::{fmt, net::SocketAddr};

/// Until authentication exists, this service can only bind IPv4 loopback.
#[derive(Debug, Clone, Copy)]
pub struct Config {
    port: u16,
}

impl Config {
    /// An absent port uses 8080. Empty, zero and invalid ports are rejected.
    pub fn from_port(port: Option<&str>) -> Result<Self, InvalidPort> {
        let port = match port {
            None => 8080,
            Some(value) => value.parse::<u16>().map_err(|_| InvalidPort)?,
        };
        if port == 0 {
            return Err(InvalidPort);
        }
        Ok(Self { port })
    }

    pub fn address(self) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], self.port))
    }
}

#[derive(Debug)]
pub struct InvalidPort;

impl fmt::Display for InvalidPort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("VMANYWARE_PORT must be an integer between 1 and 65535")
    }
}

impl std::error::Error for InvalidPort {}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn default_and_custom_ports_always_bind_loopback() {
        for (input, expected) in [(None, 8080), (Some("9000"), 9000)] {
            let address = Config::from_port(input).unwrap().address();
            assert!(address.ip().is_loopback());
            assert_eq!(address.port(), expected);
        }
    }

    #[test]
    fn rejects_invalid_ports_without_echoing_input() {
        for input in ["", "0", "65536", "-1", "localhost:8080", "secret"] {
            let error = Config::from_port(Some(input)).unwrap_err();
            assert_eq!(
                error.to_string(),
                "VMANYWARE_PORT must be an integer between 1 and 65535"
            );
        }
    }
}
