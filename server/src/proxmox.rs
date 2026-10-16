//! Concrete, read-only Proxmox API client. No mutation or passthrough methods.

use reqwest::{header, redirect::Policy, Certificate, Client, StatusCode, Url};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{fmt, time::Duration};

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Configuration,
    Unauthorized,
    Forbidden,
    Timeout,
    Unavailable,
    Provider,
    InvalidResponse,
    ResponseTooLarge,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Configuration => "Invalid Proxmox configuration.",
            Self::Unauthorized => "Proxmox rejected the API token.",
            Self::Forbidden => "The API token lacks permission for this operation.",
            Self::Timeout => "Proxmox request timed out.",
            Self::Unavailable => {
                "Cannot reach Proxmox; check routing, endpoint and trusted TLS certificates."
            }
            Self::Provider => "Proxmox returned an unsuccessful response.",
            Self::InvalidResponse => "Proxmox returned an invalid response.",
            Self::ResponseTooLarge => "Proxmox response exceeded the size limit.",
        })
    }
}

impl std::error::Error for Error {}

/// Authentication and endpoint never appear in Debug output or returned errors.
#[derive(Clone)]
pub struct Proxmox {
    client: Client,
    endpoint: Url,
    authorization: header::HeaderValue,
}

impl fmt::Debug for Proxmox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Proxmox { configuration: [REDACTED] }")
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Version {
    pub version: String,
    pub release: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Node {
    pub node: String,
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VirtualMachine {
    pub vmid: u32,
    pub name: Option<String>,
    pub status: String,
    #[serde(default, deserialize_with = "deserialize_flag")]
    pub template: u8,
}

fn deserialize_flag<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<u8, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Flag {
        Boolean(bool),
        Integer(u8),
    }
    match Flag::deserialize(deserializer)? {
        Flag::Boolean(value) => Ok(u8::from(value)),
        Flag::Integer(value @ 0..=1) => Ok(value),
        _ => Err(serde::de::Error::custom("expected boolean or 0/1")),
    }
}

#[derive(Deserialize)]
struct Envelope<T> {
    data: T,
}

impl Proxmox {
    /// Read only documented runtime variables. No implicit dotenv discovery.
    pub fn from_environment() -> Result<Option<Self>, Error> {
        fn optional(name: &str) -> Result<Option<String>, Error> {
            match std::env::var(name) {
                Ok(value) => Ok(Some(value)),
                Err(std::env::VarError::NotPresent) => Ok(None),
                Err(_) => Err(Error::Configuration),
            }
        }
        let endpoint = optional("POCKETLAB_PROXMOX_URL")?;
        let id = optional("POCKETLAB_PROXMOX_TOKEN_ID")?;
        let secret = optional("POCKETLAB_PROXMOX_TOKEN_SECRET")?;
        let ca_path = optional("POCKETLAB_PROXMOX_CA")?;
        if endpoint.is_none() && id.is_none() && secret.is_none() && ca_path.is_none() {
            return Ok(None);
        }
        let ca = ca_path
            .map(std::fs::read)
            .transpose()
            .map_err(|_| Error::Configuration)?;
        Self::new(
            &endpoint.ok_or(Error::Configuration)?,
            &id.ok_or(Error::Configuration)?,
            &secret.ok_or(Error::Configuration)?,
            ca.as_deref(),
        )
        .map(Some)
    }

    /// Endpoint must be an HTTPS origin, for example https://pve.example:8006.
    /// `ca_pem` adds a trusted CA; certificate and hostname verification stay on.
    pub fn new(
        endpoint: &str,
        token_id: &str,
        token_secret: &str,
        ca_pem: Option<&[u8]>,
    ) -> Result<Self, Error> {
        Self::build(
            endpoint,
            token_id,
            token_secret,
            ca_pem,
            Duration::from_secs(15),
        )
    }

    fn build(
        endpoint: &str,
        token_id: &str,
        token_secret: &str,
        ca_pem: Option<&[u8]>,
        timeout: Duration,
    ) -> Result<Self, Error> {
        let endpoint = Url::parse(endpoint).map_err(|_| Error::Configuration)?;
        if endpoint.scheme() != "https"
            || endpoint.host_str().is_none()
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.path() != "/"
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
        {
            return Err(Error::Configuration);
        }
        let (account, name) = token_id.split_once('!').ok_or(Error::Configuration)?;
        if !account.contains('@')
            || account.starts_with('@')
            || account.ends_with('@')
            || name.is_empty()
            || !token_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"@!._-".contains(&b))
            || name.contains('!')
            || token_secret.is_empty()
            || token_secret.bytes().any(|b| b.is_ascii_whitespace())
        {
            return Err(Error::Configuration);
        }
        let mut authorization =
            header::HeaderValue::from_str(&format!("PVEAPIToken={token_id}={token_secret}"))
                .map_err(|_| Error::Configuration)?;
        authorization.set_sensitive(true);
        let mut builder = Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(timeout);
        if let Some(pem) = ca_pem {
            let certificate = Certificate::from_pem(pem).map_err(|_| Error::Configuration)?;
            builder = builder.add_root_certificate(certificate);
        }
        Ok(Self {
            client: builder.build().map_err(|_| Error::Configuration)?,
            endpoint,
            authorization,
        })
    }

    pub async fn version(&self) -> Result<Version, Error> {
        self.get(&["version"]).await
    }

    pub async fn nodes(&self) -> Result<Vec<Node>, Error> {
        self.get(&["nodes"]).await
    }

    pub async fn virtual_machines(&self, node: &str) -> Result<Vec<VirtualMachine>, Error> {
        if node.is_empty()
            || !node
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".-_".contains(&b))
            || node == "."
            || node == ".."
        {
            return Err(Error::Configuration);
        }
        self.get(&["nodes", node, "qemu"]).await
    }

    pub(crate) async fn template_config(
        &self,
        node: &str,
        vmid: u32,
    ) -> Result<serde_json::Value, Error> {
        self.get_query(
            &["nodes", node, "qemu", &vmid.to_string(), "config"],
            &[("current", "1".to_string())],
        )
        .await
    }

    pub(crate) async fn storage(&self, node: &str) -> Result<Vec<serde_json::Value>, Error> {
        self.get(&["nodes", node, "storage"]).await
    }

    pub(crate) async fn networks(&self, node: &str) -> Result<Vec<serde_json::Value>, Error> {
        self.get(&["nodes", node, "network"]).await
    }

    pub(crate) async fn available_vmid(&self, vmid: u32) -> Result<u32, Error> {
        self.get_query(&["cluster", "nextid"], &[("vmid", vmid.to_string())])
            .await
    }

    async fn get<T: DeserializeOwned>(&self, segments: &[&str]) -> Result<T, Error> {
        self.get_query(segments, &[]).await
    }

    async fn get_query<T: DeserializeOwned>(
        &self,
        segments: &[&str],
        query: &[(&str, String)],
    ) -> Result<T, Error> {
        let mut url = self.endpoint.clone();
        url.path_segments_mut()
            .map_err(|_| Error::Configuration)?
            .extend(["api2", "json"])
            .extend(segments);
        if !query.is_empty() {
            url.query_pairs_mut()
                .extend_pairs(query.iter().map(|(key, value)| (*key, value)));
        }
        let mut response = self
            .client
            .get(url)
            .header(header::AUTHORIZATION, self.authorization.clone())
            .send()
            .await
            .map_err(transport_error)?;
        match response.status() {
            StatusCode::UNAUTHORIZED => return Err(Error::Unauthorized),
            StatusCode::FORBIDDEN => return Err(Error::Forbidden),
            status if !status.is_success() => return Err(Error::Provider),
            _ => {}
        }
        if response
            .content_length()
            .is_some_and(|n| n > MAX_RESPONSE_BYTES as u64)
        {
            return Err(Error::ResponseTooLarge);
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
            if body.len() + chunk.len() > MAX_RESPONSE_BYTES {
                return Err(Error::ResponseTooLarge);
            }
            body.extend_from_slice(&chunk);
        }
        let envelope: Envelope<T> =
            serde_json::from_slice(&body).map_err(|_| Error::InvalidResponse)?;
        Ok(envelope.data)
    }
}

fn transport_error(error: reqwest::Error) -> Error {
    if error.is_timeout() {
        Error::Timeout
    } else {
        Error::Unavailable
    }
}

#[cfg(test)]
mod tests;
