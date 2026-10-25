//! Fixed authenticated GETs only. No provider credentials, redirects or write requests.
use crate::model::*;
use reqwest::{
    header::{HeaderValue, AUTHORIZATION},
    Client, Url,
};
use serde::de::DeserializeOwned;
use std::{collections::HashSet, time::Duration};
use zeroize::Zeroizing;

pub type Read<T> = Result<T, String>;
const INVALID: &str =
    "Enter an HTTPS server origin and a 32–256 character access token (letters, digits, - or _).";
const BAD_DATA: &str = "The server returned invalid or unrelated data. Refresh to retry.";

pub struct Api {
    endpoint: Url,
    authorization: HeaderValue,
    client: Client,
}
impl Api {
    pub fn new(endpoint: &str, token: &str, ca_pem: Option<&[u8]>) -> Read<Self> {
        let endpoint = Url::parse(endpoint).map_err(|_| INVALID)?;
        if endpoint.scheme() != "https"
            || endpoint.host_str().is_none()
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.path() != "/"
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || !(32..=256).contains(&token.len())
            || !token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        {
            return Err(INVALID.into());
        }
        let bearer = Zeroizing::new(format!("Bearer {token}"));
        let mut authorization = HeaderValue::from_str(&bearer).map_err(|_| INVALID)?;
        authorization.set_sensitive(true);
        let mut builder = Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(Duration::from_secs(15));
        if let Some(pem) = ca_pem {
            let invalid = "Use one public CA certificate in PEM format, at most 16 KiB.";
            let text = std::str::from_utf8(pem).map_err(|_| invalid)?.trim();
            if pem.len() > 16 * 1024
                || !text.starts_with("-----BEGIN CERTIFICATE-----")
                || !text.ends_with("-----END CERTIFICATE-----")
                || text.matches("-----BEGIN ").count() != 1
            {
                return Err(invalid.into());
            }
            let ca = reqwest::Certificate::from_pem(pem).map_err(|_| invalid)?;
            builder = builder.tls_certs_only([ca]);
        }
        Ok(Self {
            endpoint,
            authorization,
            client: builder.build().map_err(|_| "Cannot initialize HTTPS.")?,
        })
    }

    async fn get<T: DeserializeOwned>(&self, path: &str, limit: usize) -> Read<T> {
        let url = self.endpoint.join(path).map_err(|_| BAD_DATA)?;
        let mut response = self
            .client
            .get(url)
            .header(AUTHORIZATION, self.authorization.clone())
            .send()
            .await
            .map_err(|_| "Cannot reach the server. Check the network, address and TLS trust.")?;
        match response.status().as_u16() {
            401 => return Err("Access token rejected. Press c to reconnect.".into()),
            403 => return Err("Access refused by the server.".into()),
            404 => return Err("This record is unavailable on the server.".into()),
            429 => return Err("Server busy: wait before refreshing again.".into()),
            200 => (),
            _ => return Err("The server could not complete this read. Refresh to retry.".into()),
        }
        if response
            .headers()
            .get("x-vmanyware-authenticated")
            .and_then(|v| v.to_str().ok())
            != Some("bearer")
        {
            return Err(
                "Server did not confirm bearer authentication. Check its configuration.".into(),
            );
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Server response interrupted.")?
        {
            if bytes.len() + chunk.len() > limit {
                return Err(BAD_DATA.into());
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| BAD_DATA.into())
    }

    pub async fn check(&self) -> Read<()> {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Health {
            status: String,
        }
        let health: Health = self.get("api/v1/health", 1024).await?;
        if health.status != "ok" {
            return Err("Server is not ready.".into());
        }
        Ok(())
    }

    pub async fn environments(&self) -> Read<Vec<Environment>> {
        let items: Vec<Environment> = self.get("api/v1/environments", 256 * 1024).await?;
        let mut ids = HashSet::new();
        if items.len() > 100 || items.iter().any(|e| !e.valid() || !ids.insert(&e.id)) {
            return Err(BAD_DATA.into());
        }
        Ok(items)
    }
    pub async fn operations(&self) -> Read<Vec<Operation>> {
        let items: Vec<Operation> = self.get("api/v1/operations", 256 * 1024).await?;
        let mut ids = HashSet::new();
        if items.len() > 100 || items.iter().any(|op| !op.valid() || !ids.insert(&op.id)) {
            return Err(BAD_DATA.into());
        }
        Ok(items)
    }
    pub async fn detail(&self, e: &Environment) -> Read<Detail> {
        if !e.valid() {
            return Err(BAD_DATA.into());
        }
        let detail: Detail = self
            .get(&format!("api/v1/environments/{}", e.id), 256 * 1024)
            .await?;
        if !detail.matches(e) {
            return Err(BAD_DATA.into());
        }
        Ok(detail)
    }
    pub async fn power(&self, e: &Environment) -> Read<Power> {
        if !e.valid() || e.status != Association::Registered {
            return Err("No live VM associated with this record.".into());
        }
        let power: Power = self
            .get(&format!("api/v1/environments/{}/status", e.id), 4096)
            .await?;
        if !power.matches(e) {
            return Err(BAD_DATA.into());
        }
        Ok(power)
    }
    pub async fn progress(&self, op: &Operation, environments: &[Environment]) -> Read<Progress> {
        if !op.valid() || !op.provisioning() {
            return Err(BAD_DATA.into());
        }
        let e = environments
            .iter()
            .find(|e| Some(&e.id) == op.instance_id.as_ref())
            .ok_or("The operation's environment is outside the recent list.")?;
        let progress: Progress = self
            .get(
                &format!("api/v1/operations/{}/provisioning", op.id),
                16 * 1024,
            )
            .await?;
        // The worker can advance between history and progress reads. Progress has
        // its own recorded status; do not require equality with the older summary.
        if !progress.matches(op, e) {
            return Err(BAD_DATA.into());
        }
        Ok(progress)
    }
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod tests;
