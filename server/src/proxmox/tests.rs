use super::{Error, Proxmox, MAX_RESPONSE_BYTES};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::mpsc,
    task::JoinHandle,
};
use tokio_rustls::{rustls, TlsAcceptor};

struct Fixture {
    endpoint: String,
    ca: Vec<u8>,
    requests: mpsc::UnboundedReceiver<String>,
    task: JoinHandle<()>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Fixture {
    async fn new(status: &str, body: &str, headers: &str, delay: Duration) -> Self {
        Self::routes(status, &[("*", body)], headers, delay).await
    }

    async fn routes(status: &str, routes: &[(&str, &str)], headers: &str, delay: Duration) -> Self {
        let key = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let ca = key.cert.pem().into_bytes();
        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![key.cert.der().clone()],
                rustls::pki_types::PrivatePkcs8KeyDer::from(key.signing_key.serialize_der()).into(),
            )
            .unwrap();
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!(
            "https://localhost:{}",
            listener.local_addr().unwrap().port()
        );
        let responses: Vec<(String, String)> = routes.iter().map(|(path, body)| (
            path.to_string(),
            format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}", body.len())
        )).collect();
        let (sender, requests) = mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                let Ok(mut stream) = acceptor.accept(socket).await else {
                    continue;
                };
                let mut request = Vec::new();
                let mut bytes = [0; 1024];
                while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                    let Ok(n) = stream.read(&mut bytes).await else {
                        break;
                    };
                    if n == 0 || request.len() > 8192 {
                        break;
                    }
                    request.extend_from_slice(&bytes[..n]);
                }
                let request = String::from_utf8_lossy(&request).into_owned();
                let path = request.split_whitespace().nth(1).unwrap_or("");
                let response = responses
                    .iter()
                    .find(|(route, _)| route == path || route == "*")
                    .map(|(_, body)| body.as_str())
                    .unwrap_or(
                        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    );
                let _ = sender.send(request.clone());
                tokio::time::sleep(delay).await;
                let _ = stream.write_all(response.as_bytes()).await;
                let _ = stream.shutdown().await;
            }
        });
        Self {
            endpoint,
            ca,
            requests,
            task,
        }
    }

    fn client(&self) -> Proxmox {
        Proxmox::new(
            &self.endpoint,
            "reader@pve!pocketlab",
            "fake-test-secret",
            Some(&self.ca),
        )
        .unwrap()
    }
}

#[tokio::test]
async fn api_inventory_maps_visible_templates_and_skips_offline_nodes() {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let fixture = Fixture::routes("200 OK", &[
        ("/api2/json/version", r#"{"data":{"version":"9.0.1","release":"9.0"}}"#),
        ("/api2/json/nodes", r#"{"data":[{"node":"offline","status":"offline"},{"node":"pve-1","status":"online"}]}"#),
        ("/api2/json/nodes/pve-1/qemu", r#"{"data":[{"vmid":9000,"name":"debian","status":"stopped","template":true},{"vmid":101,"status":"running","template":false}]}"#),
    ], "", Duration::ZERO).await;
    let response = crate::router_with_provider(Some(fixture.client()))
        .oneshot(
            Request::builder()
                .uri("/api/v1/provider/inventory")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
    assert_eq!(
        value,
        serde_json::json!({
            "provider":"proxmox", "version":"9.0.1",
            "nodes":[{"name":"offline","status":"offline"},{"name":"pve-1","status":"online"}],
            "templates":[{"node":"pve-1","vmid":9000,"name":"debian"}]
        })
    );
}

#[tokio::test]
async fn api_inventory_errors_never_return_upstream_bodies() {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    for (status, code) in [
        ("401 Unauthorized", "provider_unauthorized"),
        ("403 Forbidden", "provider_forbidden"),
    ] {
        let fixture = Fixture::new(status, "fake-test-secret", "", Duration::ZERO).await;
        let response = crate::router_with_provider(Some(fixture.client()))
            .oneshot(
                Request::builder()
                    .uri("/api/v1/provider/inventory")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["error"]["code"], code);
        assert!(!String::from_utf8_lossy(&body).contains("fake-test-secret"));
    }
}

#[test]
fn configuration_rejects_insecure_or_ambiguous_urls_and_bad_tokens() {
    for endpoint in [
        "http://pve:8006",
        "https://user:secret@pve:8006",
        "https://pve/api2/json",
        "https://pve/?token=secret",
        "https://pve/#secret",
        "not a url",
    ] {
        assert_eq!(
            Proxmox::new(endpoint, "reader@pve!lab", "secret", None).unwrap_err(),
            Error::Configuration
        );
    }
    for (id, secret) in [
        ("root@pam", "secret"),
        ("@pve!lab", "secret"),
        ("reader@pve!", "secret"),
        ("reader@pve!lab", ""),
        ("reader@pve!lab", "injected\r\nheader"),
    ] {
        assert_eq!(
            Proxmox::new("https://pve:8006", id, secret, None).unwrap_err(),
            Error::Configuration
        );
    }
    let client = Proxmox::new("https://pve:8006", "reader@pve!lab", "secret", None).unwrap();
    assert!(!format!("{client:?}").contains("secret"));
    assert!(!format!("{client:?}").contains("reader"));
}

#[tokio::test]
async fn verified_tls_reads_api_envelope_and_sends_token_header() {
    let mut fixture = Fixture::new(
        "200 OK",
        r#"{"data":{"version":"9.0.1","release":"9.0","repoid":"ignored"}}"#,
        "",
        Duration::ZERO,
    )
    .await;
    let version = fixture.client().version().await.unwrap();
    assert_eq!(version.version, "9.0.1");
    let request = fixture.requests.recv().await.unwrap();
    assert!(request.starts_with("GET /api2/json/version HTTP/1.1"));
    assert!(request
        .to_ascii_lowercase()
        .contains("authorization: pveapitoken=reader@pve!pocketlab=fake-test-secret\r\n"));
}

#[tokio::test]
async fn untrusted_and_hostname_mismatched_certificates_are_rejected() {
    let fixture = Fixture::new("200 OK", r#"{"data":[]}"#, "", Duration::ZERO).await;
    let client = Proxmox::new(&fixture.endpoint, "reader@pve!lab", "secret", None).unwrap();
    assert_eq!(client.nodes().await.unwrap_err(), Error::Unavailable);
    let wrong_host = fixture.endpoint.replace("localhost", "127.0.0.1");
    let client = Proxmox::new(&wrong_host, "reader@pve!lab", "secret", Some(&fixture.ca)).unwrap();
    assert_eq!(client.nodes().await.unwrap_err(), Error::Unavailable);
}

#[tokio::test]
async fn provider_errors_are_classified_without_echoing_bodies_and_redirects_are_refused() {
    for (status, expected) in [
        ("401 Unauthorized", Error::Unauthorized),
        ("403 Forbidden", Error::Forbidden),
        ("500 Internal Server Error", Error::Provider),
        ("302 Found", Error::Provider),
        ("200 OK", Error::InvalidResponse),
    ] {
        let fixture = Fixture::new(
            status,
            "fake-test-secret",
            "Location: https://example.invalid/\r\n",
            Duration::ZERO,
        )
        .await;
        let error = fixture.client().nodes().await.unwrap_err();
        assert_eq!(error, expected);
        assert!(!error.to_string().contains("fake-test-secret"));
    }
}

#[tokio::test]
async fn timeout_and_oversized_responses_are_bounded() {
    let fixture = Fixture::new("200 OK", r#"{"data":[]}"#, "", Duration::from_secs(2)).await;
    let client = Proxmox::build(
        &fixture.endpoint,
        "reader@pve!lab",
        "secret",
        Some(&fixture.ca),
        Duration::from_millis(200),
    )
    .unwrap();
    assert_eq!(client.nodes().await.unwrap_err(), Error::Timeout);
    let fixture = Fixture::new(
        "200 OK",
        &"x".repeat(MAX_RESPONSE_BYTES + 1),
        "",
        Duration::ZERO,
    )
    .await;
    assert_eq!(
        fixture.client().nodes().await.unwrap_err(),
        Error::ResponseTooLarge
    );
}

#[tokio::test]
async fn inventory_parsing_and_node_path_validation() {
    let fixture = Fixture::new(
        "200 OK",
        r#"{"data":[{"node":"pve-1","status":"online"}]}"#,
        "",
        Duration::ZERO,
    )
    .await;
    assert_eq!(fixture.client().nodes().await.unwrap()[0].node, "pve-1");
    let mut fixture = Fixture::new("200 OK", r#"{"data":[{"vmid":9000,"name":"debian","status":"stopped","template":1},{"vmid":101,"status":"running"}]}"#, "", Duration::ZERO).await;
    let client = fixture.client();
    let machines = client.virtual_machines("pve-1").await.unwrap();
    assert_eq!(machines[0].template, 1);
    assert_eq!(machines[1].template, 0);
    assert!(fixture
        .requests
        .recv()
        .await
        .unwrap()
        .starts_with("GET /api2/json/nodes/pve-1/qemu "));
    for node in ["", "..", "a/b", "a?token=secret"] {
        assert_eq!(
            client.virtual_machines(node).await.unwrap_err(),
            Error::Configuration
        );
    }
}
