//! Validated network endpoint configuration.
use std::net::SocketAddr;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsMode {
    Disabled,
    Required,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkError {
    InvalidAddress,
    InvalidTls,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkEndpoint {
    pub address: SocketAddr,
    pub http2: bool,
    pub tls: TlsMode,
}
impl NetworkEndpoint {
    pub fn new(address: impl AsRef<str>) -> Result<Self, NetworkError> {
        let address = address
            .as_ref()
            .parse()
            .map_err(|_| NetworkError::InvalidAddress)?;
        Ok(Self {
            address,
            http2: false,
            tls: TlsMode::Disabled,
        })
    }
    pub fn http2(mut self, enabled: bool) -> Self {
        self.http2 = enabled;
        self
    }
    pub fn tls(mut self, mode: TlsMode) -> Result<Self, NetworkError> {
        if mode == TlsMode::Required && self.address.port() == 0 {
            return Err(NetworkError::InvalidTls);
        }
        self.tls = mode;
        Ok(self)
    }
    pub fn scheme(&self) -> &'static str {
        match self.tls {
            TlsMode::Disabled => "http",
            TlsMode::Required => "https",
        }
    }
    pub fn describe(&self) -> String {
        format!(
            "{}://{}{}",
            self.scheme(),
            self.address,
            if self.http2 { "?http2=1" } else { "" }
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_deterministic_endpoint() {
        let e = NetworkEndpoint::new("127.0.0.1:8443")
            .unwrap()
            .http2(true)
            .tls(TlsMode::Required)
            .unwrap();
        assert_eq!(e.describe(), "https://127.0.0.1:8443?http2=1")
    }
    #[test]
    fn validates_address_and_tls() {
        assert!(NetworkEndpoint::new("bad").is_err());
        assert!(NetworkEndpoint::new("127.0.0.1:0")
            .unwrap()
            .tls(TlsMode::Required)
            .is_err())
    }
}
