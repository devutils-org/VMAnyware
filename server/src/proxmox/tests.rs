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
        Self::responses(status, routes, headers, delay, false).await
    }

    async fn responses(
        status: &str,
        routes: &[(&str, &str)],
        headers: &str,
        delay: Duration,
        sequential: bool,
    ) -> Self {
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
            let mut response_index = 0;
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
                let candidates = if sequential {
                    let index = response_index.min(responses.len() - 1);
                    response_index += 1;
                    &responses[index..=index]
                } else {
                    &responses[..]
                };
                let response = candidates
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
            "reader@pve!vmanyware",
            "fake-test-secret",
            Some(&self.ca),
        )
        .unwrap()
    }
}

#[tokio::test]
async fn worker_persists_inventory_result_and_does_not_replay_completed_requests() {
    let mut fixture = Fixture::routes(
        "200 OK",
        &[
            (
                "/api2/json/version",
                r#"{"data":{"version":"9.0.1","release":"9.0"}}"#,
            ),
            (
                "/api2/json/nodes",
                r#"{"data":[{"node":"pve-1","status":"online"}]}"#,
            ),
            (
                "/api2/json/nodes/pve-1/qemu",
                r#"{"data":[{"vmid":9000,"name":"debian","status":"stopped","template":1}]}"#,
            ),
        ],
        "",
        Duration::ZERO,
    )
    .await;
    let store = crate::store::Store::memory().unwrap();
    let id = store
        .enqueue_inventory("inventory-operation-test")
        .unwrap()
        .id;
    assert!(crate::worker::run_one(&store, Some(&fixture.client()))
        .await
        .unwrap());
    let completed = store.operation(&id).unwrap().unwrap();
    assert_eq!(completed.status, crate::operation::Status::Succeeded);
    let inventory = completed.result.unwrap();
    assert_eq!(inventory["version"], "9.0.1");
    assert_eq!(inventory["templates"][0]["vmid"], 9000);
    assert_eq!(
        store
            .enqueue_inventory("inventory-operation-test")
            .unwrap()
            .id,
        id
    );
    assert!(!crate::worker::run_one(&store, Some(&fixture.client()))
        .await
        .unwrap());
    let mut count = 0;
    while fixture.requests.try_recv().is_ok() {
        count += 1
    }
    assert_eq!(count, 3);
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
        .contains("authorization: pveapitoken=reader@pve!vmanyware=fake-test-secret\r\n"));
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

#[tokio::test]
async fn preflight_observes_metadata_over_verified_tls_and_never_mutates() {
    let mut fixture = Fixture::routes("200 OK", &[
        ("/api2/json/nodes/pve-test/qemu/9000/config?current=1", r#"{"data":{"template":1,"scsi0":"local:base-9000-disk-0,size=8G","ide2":"local:vm-9000-cloudinit,media=cdrom"}}"#),
        ("/api2/json/nodes/pve-test/storage", r#"{"data":[{"storage":"local-lvm","content":"images,rootdir","active":1,"enabled":true,"avail":100000000000}]}"#),
        ("/api2/json/nodes/pve-test/network", r#"{"data":[{"iface":"vmbr0","type":"bridge","active":1}]}"#),
        ("/api2/json/cluster/nextid?vmid=201", r#"{"data":201}"#),
    ], "", Duration::ZERO).await;
    let catalog =
        crate::provisioning::Catalog::parse(include_str!("../../../examples/templates.yaml"))
            .unwrap();
    let definition =
        crate::definition::Definition::parse(include_str!("../../../examples/kernel-lab.yaml"))
            .unwrap();
    let plan = catalog.plan(&definition, 201).unwrap();
    let expected_hash = plan.plan_hash.clone();
    use tower::ServiceExt;
    let store = crate::store::Store::memory().unwrap();
    let app = crate::router_with_catalog(Some(fixture.client()), store, Some(catalog));
    let response = app.oneshot(axum::http::Request::post("/api/v1/provisioning/preflight")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(serde_json::json!({"yaml":include_str!("../../../examples/kernel-lab.yaml"),"targetVmid":201}).to_string())).unwrap()).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let report: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 16384)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(report["planHash"], expected_hash);
    assert_eq!(report["creationEnabled"], false);
    assert_eq!(report["vmidReserved"], false);
    for item in report["checks"].as_array().unwrap().iter().take(7) {
        assert_eq!(item["status"], "passed", "{item}");
    }
    for _ in 0..4 {
        let request = fixture.requests.recv().await.unwrap();
        assert!(request.starts_with("GET "));
    }
    assert!(fixture.requests.try_recv().is_err());
}

#[tokio::test]
async fn preflight_denied_reads_are_unknown_and_do_not_leak_provider_body() {
    let fixture = Fixture::new(
        "403 Forbidden",
        r#"{"data":"secret-provider-details"}"#,
        "",
        Duration::ZERO,
    )
    .await;
    let catalog =
        crate::provisioning::Catalog::parse(include_str!("../../../examples/templates.yaml"))
            .unwrap();
    let definition =
        crate::definition::Definition::parse(include_str!("../../../examples/kernel-lab.yaml"))
            .unwrap();
    let report = serde_json::to_value(
        crate::preflight::run(&fixture.client(), catalog.plan(&definition, 201).unwrap()).await,
    )
    .unwrap();
    assert!(report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .all(|item| item["status"] == "unknown"));
    assert!(!report.to_string().contains("secret-provider-details"));
}

#[tokio::test]
async fn preflight_reports_failed_metadata_and_unknown_absent_fields() {
    let fixture = Fixture::routes("200 OK", &[
        ("/api2/json/nodes/pve-test/qemu/9000/config?current=1", r#"{"data":{"template":0,"scsi0":"local:base-9000-disk-0,size=16G","description":"cloudinit"}}"#),
        ("/api2/json/nodes/pve-test/storage", r#"{"data":[{"storage":"local-lvm","content":"images","active":0,"enabled":1,"avail":1}]}"#),
        ("/api2/json/nodes/pve-test/network", r#"{"data":[{"iface":"vmbr0","type":"eth","active":1}]}"#),
        ("/api2/json/cluster/nextid?vmid=201", r#"{"data":202}"#),
    ], "", Duration::ZERO).await;
    let catalog =
        crate::provisioning::Catalog::parse(include_str!("../../../examples/templates.yaml"))
            .unwrap();
    let definition =
        crate::definition::Definition::parse(include_str!("../../../examples/kernel-lab.yaml"))
            .unwrap();
    let report = serde_json::to_value(
        crate::preflight::run(&fixture.client(), catalog.plan(&definition, 201).unwrap()).await,
    )
    .unwrap();
    for item in report["checks"].as_array().unwrap().iter().take(7) {
        assert_eq!(item["status"], "failed", "{item}");
    }
    let fixture = Fixture::new("200 OK", r#"{"data":{}}"#, "", Duration::ZERO).await;
    let report = serde_json::to_value(
        crate::preflight::run(&fixture.client(), catalog.plan(&definition, 201).unwrap()).await,
    )
    .unwrap();
    for item in report["checks"].as_array().unwrap() {
        assert_ne!(item["status"], "passed");
    }
}

const TEST_UPID: &str = "UPID:pve-1:00000001:00000002:00000003:qmclone:201:reader@pve!lab:";

fn task_body(status: &str, exit: Option<&str>) -> String {
    let mut value = serde_json::json!({"data":{"node":"pve-1","upid":TEST_UPID,"status":status}});
    if let Some(exit) = exit {
        value["data"]["exitstatus"] = exit.into();
    }
    value.to_string()
}

#[test]
fn task_reference_and_poll_configuration_reject_unsafe_values() {
    use super::{TaskPollOptions, TaskReference};
    assert!(TaskReference::new("pve-1", TEST_UPID).is_ok());
    assert!(TaskReference::new("pve-1", &TEST_UPID.replace("00000002", "100000002")).is_ok());
    for (node, upid) in [
        ("pve-2", TEST_UPID),
        ("..", TEST_UPID),
        ("pve-1", "UPID:pve-1:../secret"),
        (
            "pve-1",
            "UPID:pve-1:GGGGGGGG:00000002:00000003:qmclone:201:a@pve:",
        ),
    ] {
        assert_eq!(
            TaskReference::new(node, upid).unwrap_err(),
            Error::Configuration
        );
    }
    for (interval, timeout) in [
        (0, 1000),
        (99, 1000),
        (1000, 999),
        (60001, 100000),
        (100, 3600001),
    ] {
        assert_eq!(
            TaskPollOptions::new(
                Duration::from_millis(interval),
                Duration::from_millis(timeout)
            )
            .unwrap_err(),
            Error::Configuration
        );
    }
}

#[tokio::test]
async fn task_status_uses_verified_tls_and_redacts_failure_details() {
    use super::{TaskReference, TaskStatus};
    let task = TaskReference::new("pve-1", TEST_UPID).unwrap();
    for (status, exit, expected) in [
        ("running", None, TaskStatus::Running),
        ("stopped", Some("OK"), TaskStatus::Succeeded),
        (
            "stopped",
            Some("secret-provider-failure"),
            TaskStatus::Failed,
        ),
    ] {
        let mut fixture =
            Fixture::new("200 OK", &task_body(status, exit), "", Duration::ZERO).await;
        let outcome = fixture.client().task_status(&task).await.unwrap();
        assert_eq!(outcome, expected);
        assert!(!format!("{outcome:?}").contains("secret"));
        let request = fixture.requests.recv().await.unwrap();
        assert!(request.starts_with("GET /api2/json/nodes/pve-1/tasks/UPID:pve-1:"));
        assert!(request
            .split_whitespace()
            .nth(1)
            .unwrap()
            .ends_with("/status"));
    }
}

#[tokio::test]
async fn malformed_or_mismatched_task_status_never_reports_success() {
    use super::TaskReference;
    let task = TaskReference::new("pve-1", TEST_UPID).unwrap();
    let mut mismatch: serde_json::Value =
        serde_json::from_str(&task_body("stopped", Some("OK"))).unwrap();
    mismatch["data"]["node"] = "pve-2".into();
    let mut wrong_upid = mismatch.clone();
    wrong_upid["data"]["node"] = "pve-1".into();
    wrong_upid["data"]["upid"] = "other".into();
    for body in [
        task_body("stopped", None),
        task_body("stopped", Some("")),
        task_body("stopped", Some(" ")),
        task_body("running", Some("OK")),
        task_body("unknown", Some("OK")),
        mismatch.to_string(),
        wrong_upid.to_string(),
        r#"{"data":{"status":"stopped","exitstatus":"OK"}}"#.into(),
    ] {
        let fixture = Fixture::new("200 OK", &body, "", Duration::ZERO).await;
        assert_eq!(
            fixture.client().task_status(&task).await.unwrap_err(),
            Error::InvalidResponse
        );
    }
}

#[tokio::test]
async fn task_poll_observes_terminal_status_without_mutations_or_extra_requests() {
    use super::{TaskPollOptions, TaskReference, TaskStatus};
    let task = TaskReference::new("pve-1", TEST_UPID).unwrap();
    let running = task_body("running", None);
    for (exit, expected) in [
        ("OK", TaskStatus::Succeeded),
        ("secret-provider-failure", TaskStatus::Failed),
    ] {
        let stopped = task_body("stopped", Some(exit));
        let mut fixture = Fixture::responses(
            "200 OK",
            &[("*", &running), ("*", &stopped)],
            "",
            Duration::ZERO,
            true,
        )
        .await;
        let result = fixture
            .client()
            .wait_for_task(
                &task,
                TaskPollOptions::new(Duration::from_millis(100), Duration::from_secs(2)).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(result, expected);
        for _ in 0..2 {
            assert!(fixture.requests.recv().await.unwrap().starts_with("GET "));
        }
        assert!(fixture.requests.try_recv().is_err());
    }
}

#[tokio::test]
async fn task_poll_deadline_covers_request_time_and_stops_on_provider_errors() {
    use super::{TaskPollOptions, TaskReference};
    let task = TaskReference::new("pve-1", TEST_UPID).unwrap();
    let options =
        TaskPollOptions::new(Duration::from_millis(100), Duration::from_millis(250)).unwrap();
    for delay in [Duration::ZERO, Duration::from_secs(2)] {
        let mut fixture = Fixture::new("200 OK", &task_body("running", None), "", delay).await;
        let start = tokio::time::Instant::now();
        assert_eq!(
            fixture
                .client()
                .wait_for_task(&task, options)
                .await
                .unwrap_err(),
            Error::Timeout
        );
        assert!(start.elapsed() < Duration::from_secs(1));
        let mut count = 0;
        while fixture.requests.try_recv().is_ok() {
            count += 1;
        }
        assert!((1..=3).contains(&count));
    }
    for (status, expected) in [
        ("401 Unauthorized", Error::Unauthorized),
        ("403 Forbidden", Error::Forbidden),
        ("500 Internal Server Error", Error::Provider),
    ] {
        let mut fixture = Fixture::new(status, "secret-provider-failure", "", Duration::ZERO).await;
        assert_eq!(
            fixture
                .client()
                .wait_for_task(&task, options)
                .await
                .unwrap_err(),
            expected
        );
        assert!(fixture.requests.recv().await.unwrap().starts_with("GET "));
        assert!(fixture.requests.try_recv().is_err());
    }
}
