//! In-memory HTTPS connection; no token persistence or generic request passthrough.
#[path = "remote_connection/create.rs"]
pub(crate) mod create;
#[path = "remote_connection/deletion.rs"]
pub(crate) mod deletion;
#[path = "remote_connection/deletion_recovery.rs"]
pub(crate) mod deletion_recovery;
#[path = "remote_connection/instance_detail.rs"]
pub(crate) mod instance_detail;
#[path = "remote_connection/instance_status.rs"]
pub(crate) mod instance_status;
#[path = "remote_connection/lifecycle.rs"]
pub(crate) mod lifecycle;
#[path = "remote_connection/lifecycle_recovery.rs"]
pub(crate) mod lifecycle_recovery;
#[path = "remote_connection/recreation.rs"]
pub(crate) mod recreation;
#[path = "remote_connection/recreation_recovery.rs"]
pub(crate) mod recreation_recovery;
#[path = "remote_connection/restore_recovery.rs"]
pub(crate) mod restore_recovery;
#[path = "remote_connection/snapshot_create.rs"]
pub(crate) mod snapshot_create;
#[path = "remote_connection/snapshot_recovery.rs"]
pub(crate) mod snapshot_recovery;
#[path = "remote_connection/snapshot_restore.rs"]
pub(crate) mod snapshot_restore;
#[path = "remote_connection/snapshots.rs"]
pub(crate) mod snapshots;
use reqwest::{
    header::{HeaderValue, AUTHORIZATION},
    redirect::Policy,
    Client, Url,
};
use serde::{Deserialize, Serialize};
use std::{sync::Mutex, time::Duration};
const RATE_LIMITED: &str = "The server is busy or limiting requests. Wait at least one second before retrying. For confirmed actions, retry only the same request key or read the known operation.";
const INVALID:&str="Enter an HTTPS server address without credentials, path, query or fragment, and a valid access token.";
fn connection_ca(pem: Option<&str>) -> Result<Option<reqwest::Certificate>, String> {
    let Some(pem) = pem.map(str::trim).filter(|pem| !pem.is_empty()) else {
        return Ok(None);
    };
    let invalid = "Enter one trusted public CA certificate in PEM format (maximum 16 KiB).";
    if pem.len() > 16 * 1024
        || !pem.starts_with("-----BEGIN CERTIFICATE-----")
        || !pem.ends_with("-----END CERTIFICATE-----")
        || pem.matches("-----BEGIN ").count() != 1
        || pem.contains("PRIVATE KEY")
    {
        return Err(invalid.into());
    }
    reqwest::Certificate::from_pem(pem.as_bytes())
        .map(Some)
        .map_err(|_| invalid.into())
}
#[derive(Clone)]
struct Profile {
    endpoint: Url,
    authorization: HeaderValue,
    client: Client,
}
impl std::fmt::Debug for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RemoteProfile { [REDACTED] }")
    }
}
impl Profile {
    fn new(endpoint: &str, token: &str, ca: Option<reqwest::Certificate>) -> Result<Self, String> {
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
        let mut authorization =
            HeaderValue::from_str(&format!("Bearer {token}")).map_err(|_| INVALID)?;
        authorization.set_sensitive(true);
        let mut builder = Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(Policy::none())
            .retry(reqwest::retry::never())
            .timeout(Duration::from_secs(15));
        if let Some(ca) = ca {
            // Android cannot merge an extra root into its platform verifier.
            // An explicit private CA scopes this profile to that root only;
            // WebPKI still verifies certificate signatures, dates and hostname.
            builder = builder.tls_certs_only([ca]);
        }
        let client = builder
            .build()
            .map_err(|_| "Cannot initialize the secure connection.")?;
        Ok(Self {
            endpoint,
            authorization,
            client,
        })
    }
    async fn get<T: serde::de::DeserializeOwned>(&self, path: ReadPath<'_>) -> Result<T, String> {
        let url = self.endpoint.join(&path.path()).map_err(|_| INVALID)?;
        let mut response = self
            .client
            .get(url)
            .header(AUTHORIZATION, self.authorization.clone())
            .send()
            .await
            .map_err(|_| {
                "Cannot reach the server. Check its address, network and TLS certificate."
            })?;
        if response.status().as_u16() == 429 {
            return Err(RATE_LIMITED.into());
        }
        if response.status().as_u16() == 401 {
            return Err("The server rejected the access token.".into());
        }
        if !response.status().is_success() {
            return Err(match path {
                ReadPath::Health => "The server rejected the connection check.",
                ReadPath::Environment(_) => "Cannot read the saved instance. Check its registration and server availability.",
                ReadPath::Lifecycle(_) => "Cannot review lifecycle prerequisites. Check target, provider and server access.",
                ReadPath::LifecycleHistory(_) | ReadPath::LifecycleOperation(_, _) => "Cannot read saved lifecycle evidence. Check the instance, operation and server journal.",
                ReadPath::InstanceStatus(_) => "Cannot observe this instance. Check its registration, provider scope, access and availability.",
                ReadPath::Images => {
                    "Cannot load templates. Check the server catalog and access configuration."
                }
                ReadPath::Snapshots(_) => "Cannot load snapshots. Check target, provider permissions and server availability.",
                ReadPath::Environments => {
                    "Cannot load environments. Check server availability and access."
                }
            }
            .into());
        }
        if response
            .headers()
            .get("x-vmanyware-authenticated")
            .and_then(|v| v.to_str().ok())
            != Some("bearer")
        {
            return Err("The server does not confirm access-token authentication. Check its configuration or version.".into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "The connection check was interrupted.")?
        {
            if bytes.len() + chunk.len() > path.limit() {
                return Err("The server returned an invalid connection response.".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| "The server returned an invalid response.".into())
    }
    async fn check(&self) -> Result<(), String> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Health {
            status: String,
        }
        let health: Health = self.get(ReadPath::Health).await?;
        if health.status != "ok" {
            return Err("The server is not ready.".into());
        }
        Ok(())
    }
}
#[derive(Clone, Copy)]
enum ReadPath<'a> {
    Health,
    Images,
    Environments,
    Snapshots(&'a str),
    Environment(&'a str),
    Lifecycle(&'a str),
    InstanceStatus(&'a str),
    LifecycleHistory(&'a str),
    LifecycleOperation(&'a str, &'a str),
}
impl ReadPath<'_> {
    fn path(self) -> String {
        match self {
            Self::Health => "api/v1/health".into(),
            Self::Environment(id) => format!("api/v1/environments/{id}"),
            Self::Lifecycle(id) => format!("api/v1/environments/{id}/lifecycle-preflight"),
            Self::InstanceStatus(id) => format!("api/v1/environments/{id}/status"),
            Self::LifecycleHistory(id) => format!("api/v1/environments/{id}/lifecycle-operations"),
            Self::LifecycleOperation(id, operation) => {
                format!("api/v1/environments/{id}/lifecycle-operations/{operation}")
            }
            Self::Images => "api/v1/images".into(),
            Self::Environments => "api/v1/environments".into(),
            Self::Snapshots(id) => format!("api/v1/environments/{id}/snapshots"),
        }
    }
    fn limit(self) -> usize {
        match self {
            Self::Health => 1024,
            Self::InstanceStatus(_) => 4096,
            Self::Snapshots(_) => 2 * 1024 * 1024,
            _ => 256 * 1024,
        }
    }
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteImage {
    image: String,
    node: String,
    template_vmid: u32,
    template_version: String,
    #[serde(rename = "sourceDiskGiB")]
    source_disk_gib: u32,
    guest_user: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteCatalog {
    api_version: String,
    kind: String,
    images: Vec<RemoteImage>,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteEnvironment {
    id: String,
    definition_name: String,
    provider_reference: RemoteProvider,
    status: RemoteEnvironmentStatus,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum RemoteEnvironmentStatus {
    Registered,
    Deleted,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoteProvider {
    provider: String,
    scope_id: String,
    node: String,
    vmid: u32,
}
fn identifier(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
fn hex(value: &str, size: usize) -> bool {
    value.len() == size
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
#[derive(Default)]
struct ConnectionState {
    generation: u64,
    profile: Option<Profile>,
    prepared: Option<create::Prepared>,
    recreation: recreation::RecreationState,
    snapshot_creation: snapshot_create::CreationState,
    snapshot_restore: snapshot_restore::RestoreState,
    deletion: deletion::DeletionState,
    lifecycle: lifecycle::LifecycleState,
}
#[derive(Default)]
pub struct RemoteConnection(Mutex<ConnectionState>);
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStatus {
    pub connection_id: u64,
    pub endpoint: String,
}
impl RemoteConnection {
    fn snapshot(&self, id: u64) -> Result<Profile, String> {
        let state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != id {
            return Err("The connection changed. Connect again.".into());
        }
        state
            .profile
            .clone()
            .ok_or_else(|| "Connect to a server first.".into())
    }
    fn current(&self, id: u64) -> Result<(), String> {
        let state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != id || state.profile.is_none() {
            return Err("The connection changed. Discarded an outdated response.".into());
        }
        Ok(())
    }
    async fn images(&self, id: u64) -> Result<Vec<RemoteImage>, String> {
        let profile = self.snapshot(id)?;
        let result: Result<RemoteCatalog, String> = profile.get(ReadPath::Images).await;
        self.current(id)?;
        let catalog = result?;
        if catalog.api_version != "vmanyware.dev/v1"
            || catalog.kind != "TemplateCatalog"
            || catalog.images.len() > 128
            || catalog.images.iter().any(|i| {
                !identifier(&i.image, 64)
                    || !identifier(&i.node, 63)
                    || !(100..=999999999).contains(&i.template_vmid)
                    || i.template_version.is_empty()
                    || i.template_version.len() > 128
                    || !(4..=4096).contains(&i.source_disk_gib)
                    || !identifier(&i.guest_user, 32)
            })
        {
            return Err("The server returned an invalid template catalog.".into());
        }
        Ok(catalog.images)
    }
    async fn environments(&self, id: u64) -> Result<Vec<RemoteEnvironment>, String> {
        let profile = self.snapshot(id)?;
        let result: Result<Vec<RemoteEnvironment>, String> =
            profile.get(ReadPath::Environments).await;
        self.current(id)?;
        let environments = result?;
        if environments.len() > 100
            || environments.iter().any(|e| {
                !hex(&e.id, 32)
                    || !identifier(&e.definition_name, 63)
                    || e.provider_reference.provider != "proxmox"
                    || !hex(&e.provider_reference.scope_id, 64)
                    || !identifier(&e.provider_reference.node, 63)
                    || !(100..=999999999).contains(&e.provider_reference.vmid)
            })
        {
            return Err("The server returned an invalid environment list.".into());
        }
        Ok(environments)
    }
    fn begin(&self) -> Result<u64, String> {
        let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        state.generation = state
            .generation
            .checked_add(1)
            .ok_or("Connection state unavailable.")?;
        state.profile = None;
        state.prepared = None;
        state.recreation = recreation::RecreationState::default();
        state.snapshot_creation = snapshot_create::CreationState::default();
        state.snapshot_restore = snapshot_restore::RestoreState::default();
        state.deletion = deletion::DeletionState::default();
        state.lifecycle = lifecycle::LifecycleState::default();
        Ok(state.generation)
    }
    fn complete(&self, generation: u64, profile: Profile) -> Result<ConnectionStatus, String> {
        let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != generation {
            return Err("The connection changed. Check the current server again.".into());
        }
        let status = ConnectionStatus {
            connection_id: generation,
            endpoint: profile.endpoint.as_str().into(),
        };
        state.profile = Some(profile);
        Ok(status)
    }
}
#[tauri::command]
pub async fn connect_remote(
    state: tauri::State<'_, RemoteConnection>,
    endpoint: String,
    token: String,
    ca_pem: Option<String>,
) -> Result<ConnectionStatus, String> {
    let generation = state.begin()?;
    let profile = Profile::new(&endpoint, &token, connection_ca(ca_pem.as_deref())?)?;
    drop(token);
    profile.check().await?;
    state.complete(generation, profile)
}
#[tauri::command]
pub fn disconnect_remote(state: tauri::State<'_, RemoteConnection>) -> Result<(), String> {
    state.begin().map(|_| ())
}

#[tauri::command]
pub async fn read_remote_images(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
) -> Result<Vec<RemoteImage>, String> {
    state.images(connection_id).await
}
#[tauri::command]
pub async fn read_remote_environments(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
) -> Result<Vec<RemoteEnvironment>, String> {
    state.environments(connection_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    const TOKEN: &str = "fixture-only-access-token-0123456789abcdef";
    pub(super) async fn fixture(
        status: &str,
        body: &str,
        headers: &str,
    ) -> (
        String,
        reqwest::Certificate,
        tokio::task::JoinHandle<String>,
    ) {
        fixture_with_gate(status, body, headers, None).await
    }
    pub(super) async fn fixture_with_gate(
        status: &str,
        body: &str,
        headers: &str,
        gate: Option<(
            tokio::sync::oneshot::Sender<()>,
            tokio::sync::oneshot::Receiver<()>,
        )>,
    ) -> (
        String,
        reqwest::Certificate,
        tokio::task::JoinHandle<String>,
    ) {
        let key = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let ca = reqwest::Certificate::from_pem(key.cert.pem().as_bytes()).unwrap();
        let config = tokio_rustls::rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![key.cert.der().clone()],
                tokio_rustls::rustls::pki_types::PrivatePkcs8KeyDer::from(
                    key.signing_key.serialize_der(),
                )
                .into(),
            )
            .unwrap();
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!(
            "https://localhost:{}",
            listener.local_addr().unwrap().port()
        );
        let reply = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}",
            body.len()
        );
        let task = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let Ok(mut stream) = acceptor.accept(socket).await else {
                return String::new();
            };
            let mut request = Vec::new();
            let mut chunk = [0; 1024];
            while !request.windows(4).any(|v| v == b"\r\n\r\n") {
                let n = stream.read(&mut chunk).await.unwrap();
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..n]);
            }
            if let Some(end) = request.windows(4).position(|v| v == b"\r\n\r\n") {
                let length = String::from_utf8_lossy(&request[..end])
                    .lines()
                    .find_map(|line| {
                        let (key, value) = line.split_once(':')?;
                        if key.eq_ignore_ascii_case("content-length") {
                            value.trim().parse::<usize>().ok()
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);
                while request.len() < end + 4 + length && request.len() < 65536 {
                    let n = stream.read(&mut chunk).await.unwrap();
                    if n == 0 {
                        break;
                    }
                    request.extend_from_slice(&chunk[..n]);
                }
            }
            if let Some((seen, release)) = gate {
                let _ = seen.send(());
                let _ = release.await;
            }
            let _ = stream.write_all(reply.as_bytes()).await;
            String::from_utf8(request).unwrap()
        });
        (endpoint, ca, task)
    }
    #[test]
    fn connection_ca_requires_one_bounded_public_certificate() {
        assert!(connection_ca(None).unwrap().is_none());
        assert!(connection_ca(Some("  ")).unwrap().is_none());
        let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()])
            .unwrap()
            .cert
            .pem();
        assert!(connection_ca(Some(&certificate)).unwrap().is_some());
        for invalid in [
            "not a certificate".to_owned(),
            "-----BEGIN PRIVATE KEY-----\nfixture\n-----END PRIVATE KEY-----".to_owned(),
            format!("{certificate}{certificate}"),
            format!(
                "-----BEGIN CERTIFICATE-----{}-----END CERTIFICATE-----",
                "A".repeat(16 * 1024)
            ),
        ] {
            let error = connection_ca(Some(&invalid)).unwrap_err();
            assert!(!error.contains(&invalid));
            assert!(error.contains("public CA"));
        }
    }

    #[tokio::test]
    async fn verified_https_sends_token_only_to_fixed_health_path_and_keeps_it_out_of_status() {
        let (endpoint, ca, task) = fixture(
            "200 OK",
            r#"{"status":"ok"}"#,
            "x-vmanyware-authenticated: bearer\r\n",
        )
        .await;
        let profile = Profile::new(&endpoint, TOKEN, Some(ca)).unwrap();
        profile.check().await.unwrap();
        let request = task.await.unwrap();
        assert!(request.starts_with("GET /api/v1/health "));
        assert!(request
            .to_ascii_lowercase()
            .contains(&format!("authorization: bearer {TOKEN}")));
        assert!(!format!("{profile:?}").contains(TOKEN));
        let manager = RemoteConnection::default();
        let generation = manager.begin().unwrap();
        let status = manager.complete(generation, profile).unwrap();
        assert!(!serde_json::to_string(&status).unwrap().contains(TOKEN));
        manager.begin().unwrap();
        assert!(manager.0.lock().unwrap().profile.is_none());
    }
    #[tokio::test]
    async fn redirects_untrusted_tls_and_rejected_tokens_never_establish_a_connection() {
        for (status, body, headers, expected) in [
            ("200 OK", r#"{"status":"ok"}"#, "", "authentication"),
            (
                "401 Unauthorized",
                "private-provider-error",
                "",
                "access token",
            ),
            (
                "302 Found",
                "",
                "Location: https://other.invalid/\r\n",
                "rejected",
            ),
            (
                "200 OK",
                r#"{"status":"ok","token":"private"}"#,
                "x-vmanyware-authenticated: bearer\r\n",
                "invalid",
            ),
        ] {
            let (endpoint, ca, task) = fixture(status, body, headers).await;
            let profile = Profile::new(&endpoint, TOKEN, Some(ca)).unwrap();
            let error = profile.check().await.unwrap_err();
            assert!(error.contains(expected));
            assert!(!error.contains("private"));
            task.await.unwrap();
        }
        let (endpoint, _ca, task) = fixture("200 OK", r#"{"status":"ok"}"#, "").await;
        assert!(Profile::new(&endpoint, TOKEN, None)
            .unwrap()
            .check()
            .await
            .is_err());
        assert!(task.await.unwrap().is_empty());
    }
    #[test]
    fn invalid_addresses_are_rejected_and_stale_checks_cannot_replace_current_connection() {
        for endpoint in [
            "http://localhost:8080",
            "https://user:password@example.test",
            "https://example.test/api",
            "https://example.test?token=private",
            "https://example.test/#fragment",
        ] {
            assert_eq!(Profile::new(endpoint, TOKEN, None).unwrap_err(), INVALID);
        }
        let manager = RemoteConnection::default();
        let stale = manager.begin().unwrap();
        let current = manager.begin().unwrap();
        assert!(manager
            .complete(
                stale,
                Profile::new("https://example.test", TOKEN, None).unwrap()
            )
            .is_err());
        assert!(manager
            .complete(
                current,
                Profile::new("https://example.test", TOKEN, None).unwrap()
            )
            .is_ok());
    }
    const AUTH: &str = "x-vmanyware-authenticated: bearer\r\n";
    fn catalog_body() -> String {
        serde_json::json!({"apiVersion":"vmanyware.dev/v1","kind":"TemplateCatalog","images":[{"image":"debian-13","node":"fixture-node","templateVmid":9000,"templateVersion":"fixture-v1","sourceDiskGiB":8,"guestUser":"debian"}]}).to_string()
    }
    fn environments_body() -> String {
        serde_json::json!([{"id":"a".repeat(32),"definitionName":"remote-lab","providerReference":{"provider":"proxmox","scopeId":"b".repeat(64),"node":"fixture-node","vmid":202},"status":"registered"}]).to_string()
    }
    #[tokio::test]
    async fn remote_workspace_reads_only_fixed_scoped_authenticated_paths() {
        for (images, body, path) in [
            (true, catalog_body(), "/api/v1/images"),
            (false, environments_body(), "/api/v1/environments"),
            (
                false,
                environments_body().replace("registered", "deleted"),
                "/api/v1/environments",
            ),
        ] {
            let (endpoint, ca, task) = fixture("200 OK", &body, AUTH).await;
            let manager = RemoteConnection::default();
            let id = manager.begin().unwrap();
            manager
                .complete(id, Profile::new(&endpoint, TOKEN, Some(ca)).unwrap())
                .unwrap();
            if images {
                let list = manager.images(id).await.unwrap();
                assert_eq!(list[0].image, "debian-13");
                assert_eq!(list[0].template_vmid, 9000);
            } else {
                let list = manager.environments(id).await.unwrap();
                assert_eq!(list[0].definition_name, "remote-lab");
                assert_eq!(list[0].provider_reference.vmid, 202);
            }
            let request = task.await.unwrap();
            assert!(request.starts_with(&format!("GET {path} ")));
            assert!(request
                .to_ascii_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}")));
            manager.begin().unwrap();
            assert!(manager.snapshot(id).is_err());
        }
    }
    #[tokio::test]
    async fn remote_workspace_rejects_malformed_data_or_authentication_without_raw_errors() {
        for (images, status, body, headers) in [
            (
                true,
                "200 OK",
                catalog_body().replace("vmanyware.dev/v1", "unexpected"),
                AUTH,
            ),
            (
                false,
                "200 OK",
                environments_body().replace("registered", "ready"),
                AUTH,
            ),
            (true, "401 Unauthorized", "private-raw-error".into(), ""),
            (false, "200 OK", environments_body(), ""),
        ] {
            let (endpoint, ca, task) = fixture(status, &body, headers).await;
            let manager = RemoteConnection::default();
            let id = manager.begin().unwrap();
            manager
                .complete(id, Profile::new(&endpoint, TOKEN, Some(ca)).unwrap())
                .unwrap();
            let error = if images {
                manager.images(id).await.err()
            } else {
                manager.environments(id).await.err()
            }
            .unwrap();
            assert!(!error.contains("private-raw-error"));
            assert!(!error.contains(TOKEN));
            task.await.unwrap();
        }
    }
    #[tokio::test]
    async fn remote_workspace_discards_an_inflight_response_after_disconnect() {
        let (seen, received) = tokio::sync::oneshot::channel();
        let (release, gate) = tokio::sync::oneshot::channel();
        let (endpoint, ca, task) =
            fixture_with_gate("200 OK", &environments_body(), AUTH, Some((seen, gate))).await;
        let manager = Arc::new(RemoteConnection::default());
        let id = manager.begin().unwrap();
        manager
            .complete(id, Profile::new(&endpoint, TOKEN, Some(ca)).unwrap())
            .unwrap();
        let active = manager.clone();
        let read = tokio::spawn(async move { active.environments(id).await });
        tokio::time::timeout(Duration::from_secs(3), received)
            .await
            .unwrap()
            .unwrap();
        manager.begin().unwrap();
        release.send(()).unwrap();
        let error = read.await.unwrap().err().unwrap();
        assert!(error.contains("outdated"));
        task.await.unwrap();
    }
}

#[cfg(test)]
mod capacity_tests {
    use super::*;
    #[tokio::test]
    async fn request_capacity_errors_cover_all_remote_decoders_without_auth_marker_or_body_echo() {
        for marked in [false, true] {
            for decoder in 0..4 {
                let headers = if marked {
                    "x-vmanyware-authenticated: bearer\r\nRetry-After: 1\r\n"
                } else {
                    "Retry-After: private-proxy-value\r\n"
                };
                let (endpoint, ca, task) =
                    tests::fixture("429 Too Many Requests", "PRIVATE_SERVER_BODY", headers).await;
                let profile = Profile::new(
                    &endpoint,
                    "fixture-only-access-token-0123456789abcdef",
                    Some(ca),
                )
                .unwrap();
                let request = profile
                    .client
                    .get(profile.endpoint.join("api/v1/health").unwrap());
                let error = match decoder {
                    0 => profile
                        .get::<serde_json::Value>(ReadPath::Health)
                        .await
                        .unwrap_err(),
                    1 => profile
                        .create_json::<serde_json::Value>(request)
                        .await
                        .unwrap_err(),
                    2 => profile
                        .snapshot_json::<serde_json::Value>(request, false)
                        .await
                        .unwrap_err(),
                    _ => profile
                        .deletion_json::<serde_json::Value>(request, false)
                        .await
                        .unwrap_err(),
                };
                assert_eq!(error, RATE_LIMITED);
                assert!(
                    !error.contains("PRIVATE_SERVER_BODY")
                        && !error.contains("private-proxy-value")
                );
                assert!(task.await.unwrap().starts_with("GET /api/v1/health "));
            }
        }
    }
}
