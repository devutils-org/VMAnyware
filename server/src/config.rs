use std::{fmt, net::SocketAddr};

/// The service binds IPv4 loopback; remote TLS termination is configured separately.
#[derive(Debug, Clone, Copy)]
pub struct Config {
    port: u16,
    lifecycle_enabled: bool,
    provisioning_enabled: bool,
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
        Ok(Self {
            port,
            lifecycle_enabled: false,
            provisioning_enabled: false,
        })
    }

    /// Lifecycle execution is opt-in. Only exact lowercase true/false are accepted.
    pub fn with_lifecycle(mut self, value: Option<&str>) -> Result<Self, InvalidLifecycle> {
        self.lifecycle_enabled = match value {
            None | Some("false") => false,
            Some("true") => true,
            _ => return Err(InvalidLifecycle),
        };
        Ok(self)
    }

    pub fn with_provisioning(mut self, value: Option<&str>) -> Result<Self, InvalidProvisioning> {
        self.provisioning_enabled = match value {
            None | Some("false") => false,
            Some("true") => true,
            _ => return Err(InvalidProvisioning),
        };
        Ok(self)
    }
    pub fn provisioning_enabled(self) -> bool {
        self.provisioning_enabled
    }

    pub fn lifecycle_enabled(self) -> bool {
        self.lifecycle_enabled
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

#[derive(Debug)]
pub struct InvalidLifecycle;
impl fmt::Display for InvalidLifecycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("VMANYWARE_LIFECYCLE_ENABLED must be true or false")
    }
}
impl std::error::Error for InvalidLifecycle {}

#[derive(Debug)]
pub struct InvalidProvisioning;
impl fmt::Display for InvalidProvisioning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("VMANYWARE_PROVISIONING_ENABLED must be true or false")
    }
}
impl std::error::Error for InvalidProvisioning {}

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
    fn lifecycle_requires_explicit_exact_opt_in() {
        let config = Config::from_port(None).unwrap();
        assert!(!config.lifecycle_enabled());
        for (input, enabled) in [(None, false), (Some("false"), false), (Some("true"), true)] {
            let configured = config.with_lifecycle(input).unwrap();
            assert_eq!(configured.lifecycle_enabled(), enabled);
            assert!(configured.address().ip().is_loopback());
        }
        for input in [
            "",
            "1",
            "0",
            "TRUE",
            "yes",
            " true",
            "false\n",
            "private-input",
        ] {
            assert_eq!(
                config.with_lifecycle(Some(input)).unwrap_err().to_string(),
                "VMANYWARE_LIFECYCLE_ENABLED must be true or false"
            );
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
