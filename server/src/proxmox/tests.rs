use super::{Error, Proxmox, MAX_RESPONSE_BYTES};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::mpsc,
    task::JoinHandle,
};
use tokio_rustls::{rustls, TlsAcceptor};

// Explicit connection-loss injection for mutation no-replay tests.
const DROP_REPLY: &str = "fixture-close-without-response";

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
            if *body == DROP_REPLY { String::new() } else {
                format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}", body.len())
            }
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
                if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&request[..end]);
                    let length = header
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                        .unwrap_or(0);
                    while request.len() < end + 4 + length && request.len() <= 8192 {
                        let Ok(n) = stream.read(&mut bytes).await else {
                            break;
                        };
                        if n == 0 {
                            break;
                        }
                        request.extend_from_slice(&bytes[..n]);
                    }
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
                if response.is_empty() {
                    continue;
                }
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

#[test]
fn vm_reference_rejects_ambiguous_targets() {
    use super::VmReference;
    for node in [
        "",
        ".",
        "..",
        "-node",
        "pve/other",
        "pve?token=x",
        "pve%2fother",
        "pvé",
        &"n".repeat(64),
    ] {
        assert_eq!(
            VmReference::new(node, 202).unwrap_err(),
            Error::Configuration
        );
    }
    for vmid in [0, 99, 1_000_000_000, u32::MAX] {
        assert_eq!(
            VmReference::new("pve-lab", vmid).unwrap_err(),
            Error::Configuration
        );
    }
    for vmid in [100, 202, 999_999_999] {
        assert!(VmReference::new("pve-lab", vmid).is_ok());
    }
}

#[tokio::test]
async fn vm_observation_preserves_power_and_execution_distinction_and_checks_identity() {
    use super::{PowerState, QmpState, VmReference};
    let target = VmReference::new("pve-lab", 202).unwrap();
    for (status, qmp, expected) in [
        ("running", "running", QmpState::Running),
        ("running", "paused", QmpState::Paused),
        ("running", "prelaunch", QmpState::Prelaunch),
        ("stopped", "stopped", QmpState::Stopped),
        ("running", "future-sensitive-detail", QmpState::Unknown),
    ] {
        let body = serde_json::json!({"data":{"vmid":202,"status":status,"qmpstatus":qmp,"name":"ignored-sensitive-name","agent":1}}).to_string();
        let mut fixture = Fixture::new("200 OK", &body, "", Duration::ZERO).await;
        let observation = fixture.client().vm_status(&target).await.unwrap();
        assert_eq!(observation.vmid, 202);
        assert_eq!(observation.node, "pve-lab");
        assert_eq!(
            observation.status,
            if status == "running" {
                PowerState::Running
            } else {
                PowerState::Stopped
            }
        );
        assert_eq!(observation.qmp_status, Some(expected));
        let json = serde_json::to_string(&observation).unwrap();
        assert!(!json.contains("ready"));
        assert!(!json.contains("sensitive"));
        let request = fixture.requests.recv().await.unwrap();
        assert!(request
            .starts_with("GET /api2/json/nodes/pve-lab/qemu/202/status/current HTTP/1.1\r\n"));
        assert!(fixture.requests.try_recv().is_err());
    }
    for body in [
        r#"{"data":{"vmid":203,"status":"running"}}"#,
        r#"{"data":{"status":"running"}}"#,
        r#"{"data":{"vmid":202,"status":"future-state"}}"#,
        r#"{"data":{"vmid":202,"status":null}}"#,
        r#"{"data":{"vmid":202,"status":"running","qmpstatus":42}}"#,
    ] {
        let fixture = Fixture::new("200 OK", body, "", Duration::ZERO).await;
        assert_eq!(
            fixture.client().vm_status(&target).await.unwrap_err(),
            Error::InvalidResponse
        );
    }
    let fixture = Fixture::new(
        "200 OK",
        r#"{"data":{"vmid":202,"status":"stopped"}}"#,
        "",
        Duration::ZERO,
    )
    .await;
    assert_eq!(
        fixture
            .client()
            .vm_status(&target)
            .await
            .unwrap()
            .qmp_status,
        None
    );
    let untrusted = Proxmox::new(&fixture.endpoint, "reader@pve!test", "fake", None).unwrap();
    assert_eq!(
        untrusted.vm_status(&target).await.unwrap_err(),
        Error::Unavailable
    );
}

#[tokio::test]
async fn vm_status_api_returns_only_safe_observations_and_errors() {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    for (upstream, payload, expected, code) in [
        (
            "200 OK",
            r#"{"data":{"vmid":202,"status":"running","qmpstatus":"paused","name":"sensitive-marker"}}"#,
            StatusCode::OK,
            None,
        ),
        (
            "401 Unauthorized",
            "sensitive-marker",
            StatusCode::BAD_GATEWAY,
            Some("provider_unauthorized"),
        ),
        (
            "403 Forbidden",
            "sensitive-marker",
            StatusCode::BAD_GATEWAY,
            Some("provider_forbidden"),
        ),
        (
            "404 Not Found",
            "sensitive-marker",
            StatusCode::BAD_GATEWAY,
            Some("provider_unavailable"),
        ),
        (
            "200 OK",
            r#"{"data":{"vmid":203,"status":"running"}}"#,
            StatusCode::BAD_GATEWAY,
            Some("provider_unavailable"),
        ),
    ] {
        let mut fixture = Fixture::new(upstream, payload, "", Duration::ZERO).await;
        let app = crate::router_with_provider(Some(fixture.client()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/provider/nodes/pve-lab/vms/202/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        let body = to_bytes(response.into_body(), 2048).await.unwrap();
        assert!(!String::from_utf8_lossy(&body).contains("sensitive-marker"));
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        if let Some(code) = code {
            assert_eq!(json["error"]["code"], code);
        } else {
            assert_eq!(
                json,
                serde_json::json!({"node":"pve-lab", "vmid":202,"status":"running","qmpStatus":"paused"})
            );
        }
        assert!(fixture.requests.recv().await.unwrap().starts_with("GET "));
        assert!(fixture.requests.try_recv().is_err());
    }
    let fixture = Fixture::new(
        "200 OK",
        r#"{"data":{"vmid":202,"status":"stopped"}}"#,
        "",
        Duration::from_millis(100),
    )
    .await;
    let provider = Proxmox::build(
        &fixture.endpoint,
        "reader@pve!test",
        "fake",
        Some(&fixture.ca),
        Duration::from_millis(20),
    )
    .unwrap();
    let response = crate::router_with_provider(Some(provider))
        .oneshot(
            Request::builder()
                .uri("/api/v1/provider/nodes/pve-lab/vms/202/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);
}

fn registered_fixture_instance(store: &crate::store::Store, scope: &str) -> String {
    let definition =
        crate::definition::Definition::parse(include_str!("../../../examples/kernel-lab.yaml"))
            .unwrap();
    store.create(&definition).unwrap();
    store
        .register_instance(
            &crate::instance::Registration {
                definition_name: definition.metadata.name.clone(),
                definition_hash: definition.hash().unwrap(),
                node: "pve-lab".into(),
                vmid: 202,
            },
            scope,
        )
        .unwrap()
        .summary
        .id
}

#[tokio::test]
async fn registered_instance_observation_uses_retained_target_without_changing_registration() {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let mut fixture = Fixture::new("200 OK", r#"{"data":{"vmid":202,"status":"running","qmpstatus":"prelaunch","name":"sensitive-marker"}}"#, "", Duration::ZERO).await;
    let provider = fixture.client();
    let scope = provider.scope_id();
    let store = crate::store::Store::memory().unwrap();
    let id = registered_fixture_instance(&store, &scope);
    let app = crate::router_with_services(Some(provider), store.clone());
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/environments/{id}/status?node=other&vmid=999"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 2048).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        json,
        serde_json::json!({"instanceId": id,"providerScopeId":scope,"observation":{"node":"pve-lab","vmid":202,"status":"running","qmpStatus":"prelaunch"}})
    );
    assert!(!String::from_utf8_lossy(&bytes).contains("sensitive-marker"));
    assert!(fixture
        .requests
        .recv()
        .await
        .unwrap()
        .starts_with("GET /api2/json/nodes/pve-lab/qemu/202/status/current HTTP/1.1\r\n"));
    assert!(fixture.requests.try_recv().is_err());
    let retained = serde_json::to_value(store.instance(&id).unwrap().unwrap()).unwrap();
    assert_eq!(retained["status"], "registered");
    assert!(retained.get("observation").is_none());
    assert!(store.recent_operations().unwrap().is_empty());
}

#[tokio::test]
async fn registered_status_rejects_changed_scope_and_invalid_storage_before_network() {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let mut fixture = Fixture::new(
        "200 OK",
        r#"{"data":{"vmid":202,"status":"stopped"}}"#,
        "",
        Duration::ZERO,
    )
    .await;
    let store = crate::store::Store::memory().unwrap();
    let id = registered_fixture_instance(&store, &"a".repeat(64));
    let response = crate::router_with_services(Some(fixture.client()), store.clone())
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/environments/{id}/status"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let json: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 2048).await.unwrap()).unwrap();
    assert_eq!(json["error"]["code"], "provider_scope_changed");
    assert!(fixture.requests.try_recv().is_err());
    let response = crate::router_with_services(None, store)
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/environments/{id}/status"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("invalid-instance.sqlite");
    let store = crate::store::Store::open(&path).unwrap();
    let id = registered_fixture_instance(&store, &fixture.client().scope_id());
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute("UPDATE instances SET node='../sensitive-marker'", [])
        .unwrap();
    drop(connection);
    let response = crate::router_with_services(Some(fixture.client()), store)
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/environments/{id}/status"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(fixture.requests.try_recv().is_err());
}

#[tokio::test]
async fn instance_worker_persists_scoped_observation_and_does_not_replay_terminal_work() {
    let mut fixture = Fixture::new(
        "200 OK",
        r#"{"data":{"vmid":202,"status":"running","qmpstatus":"paused"}}"#,
        "",
        Duration::ZERO,
    )
    .await;
    let provider = fixture.client();
    let store = crate::store::Store::memory().unwrap();
    let id = registered_fixture_instance(&store, &provider.scope_id());
    let operation = store
        .enqueue_instance_observation("observation", &id)
        .unwrap();
    assert!(fixture.requests.try_recv().is_err());
    assert!(crate::worker::run_one(&store, Some(&provider))
        .await
        .unwrap());
    let terminal = store.operation(&operation.id).unwrap().unwrap();
    assert_eq!(terminal.status, crate::operation::Status::Succeeded);
    let result = terminal.result.unwrap();
    assert_eq!(result["instanceId"], id);
    assert_eq!(result["observation"]["qmpStatus"], "paused");
    assert!(fixture
        .requests
        .recv()
        .await
        .unwrap()
        .starts_with("GET /api2/json/nodes/pve-lab/qemu/202/status/current "));
    assert!(!crate::worker::run_one(&store, Some(&provider))
        .await
        .unwrap());
    assert!(fixture.requests.try_recv().is_err());
}

#[tokio::test]
async fn instance_worker_rejects_changed_scope_before_transport_and_redacts_provider_failures() {
    use crate::operation::{Failure, Status};
    let mut fixture = Fixture::new("200 OK", "sensitive-marker", "", Duration::ZERO).await;
    let provider = fixture.client();
    let store = crate::store::Store::memory().unwrap();
    let id = registered_fixture_instance(&store, &"a".repeat(64));
    let operation = store
        .enqueue_instance_observation("changed-scope", &id)
        .unwrap();
    crate::worker::run_one(&store, Some(&provider))
        .await
        .unwrap();
    assert_eq!(
        store.operation(&operation.id).unwrap().unwrap().failure,
        Some(Failure::ProviderScopeChanged)
    );
    assert!(fixture.requests.try_recv().is_err());
    for (status, payload, expected) in [
        (
            "403 Forbidden",
            "sensitive-marker",
            Failure::ProviderForbidden,
        ),
        (
            "401 Unauthorized",
            "sensitive-marker",
            Failure::ProviderUnauthorized,
        ),
        (
            "200 OK",
            r#"{"data":{"vmid":203,"status":"running"}}"#,
            Failure::ProviderUnavailable,
        ),
    ] {
        let fixture = Fixture::new(status, payload, "", Duration::ZERO).await;
        let provider = fixture.client();
        let store = crate::store::Store::memory().unwrap();
        let id = registered_fixture_instance(&store, &provider.scope_id());
        let operation = store.enqueue_instance_observation("failure", &id).unwrap();
        crate::worker::run_one(&store, Some(&provider))
            .await
            .unwrap();
        let terminal = store.operation(&operation.id).unwrap().unwrap();
        assert_eq!(terminal.status, Status::Failed);
        assert_eq!(terminal.failure, Some(expected));
        assert!(terminal.result.is_none());
        assert!(!serde_json::to_string(&terminal)
            .unwrap()
            .contains("sensitive-marker"));
    }
}

fn lifecycle_summary(provider: &Proxmox) -> crate::instance::Summary {
    crate::instance::Summary {
        id: "a".repeat(32),
        definition_name: "fixture".into(),
        definition_hash: "d".repeat(64),
        definition_revision: 1,
        status: crate::instance::Status::Registered,
        created_at: "2026-10-18".into(),
        provider_reference: crate::instance::ProviderReference {
            provider: "proxmox",
            scope_id: provider.scope_id(),
            node: "pve-lab".into(),
            vmid: 202,
        },
    }
}
fn lifecycle_config() -> serde_json::Value {
    serde_json::json!({"data":{"digest":"b".repeat(40), "tags":format!("lab;vmanyware-{}", "a".repeat(32)), "template":0,"description":"sensitive-marker"}})
}
async fn lifecycle_fixture(
    before: serde_json::Value,
    pending: serde_json::Value,
    power: &str,
    qmp: Option<&str>,
    after: serde_json::Value,
) -> Fixture {
    Fixture::responses(
        "200 OK",
        &[
            (
                "/api2/json/nodes/pve-lab/qemu/202/config?current=1",
                &before.to_string(),
            ),
            (
                "/api2/json/nodes/pve-lab/qemu/202/pending",
                &pending.to_string(),
            ),
            (
                "/api2/json/nodes/pve-lab/qemu/202/status/current",
                &serde_json::json!({"data":{"vmid":202,"status":power,"qmpstatus":qmp}})
                    .to_string(),
            ),
            (
                "/api2/json/nodes/pve-lab/qemu/202/config?current=1",
                &after.to_string(),
            ),
        ],
        "",
        Duration::ZERO,
        true,
    )
    .await
}

#[tokio::test]
async fn lifecycle_preflight_uses_current_config_and_pending_reads_without_enabling_execution() {
    use crate::lifecycle::Status;
    for (power, qmp, start, shutdown) in [
        ("stopped", Some("stopped"), Status::Passed, Status::Failed),
        ("running", Some("running"), Status::Failed, Status::Passed),
        ("running", Some("paused"), Status::Failed, Status::Failed),
        ("running", Some("prelaunch"), Status::Failed, Status::Failed),
        ("running", None, Status::Failed, Status::Unknown),
        (
            "running",
            Some("future-state"),
            Status::Failed,
            Status::Unknown,
        ),
    ] {
        let mut fixture = lifecycle_fixture(
            lifecycle_config(),
            serde_json::json!({"data":[{"key":"memory","value":"2048","delete":0}]}),
            power,
            qmp,
            lifecycle_config(),
        )
        .await;
        let provider = fixture.client();
        let preflight = lifecycle_summary(&provider)
            .lifecycle_preflight(&provider)
            .await
            .unwrap();
        assert_eq!(preflight.start_preconditions, start);
        assert_eq!(preflight.shutdown_preconditions, shutdown);
        assert!(!preflight.execution_enabled);
        assert!(preflight
            .checks
            .iter()
            .all(|check| check.status == Status::Passed));
        assert!(!serde_json::to_string(&preflight)
            .unwrap()
            .contains("sensitive-marker"));
        for _ in 0..4 {
            assert!(fixture.requests.recv().await.unwrap().starts_with("GET "));
        }
        assert!(fixture.requests.try_recv().is_err());
    }
}

#[tokio::test]
async fn lifecycle_preflight_blocks_foreign_ambiguous_locked_template_pending_or_changed_config() {
    use crate::lifecycle::Status;
    for (field, value, blocked) in [
        ("tags", serde_json::json!("foreign"), "instance_marker"),
        (
            "tags",
            serde_json::json!(format!(
                "vmanyware-{};vmanyware-{}",
                "a".repeat(32),
                "c".repeat(32)
            )),
            "instance_marker",
        ),
        (
            "tags",
            serde_json::json!(format!("vmanyware-{};VMANYWARE-OTHER", "a".repeat(32))),
            "instance_marker",
        ),
        (
            "tags",
            serde_json::json!(format!(
                "vmanyware-{};vmanyware-{}",
                "a".repeat(32),
                "a".repeat(32)
            )),
            "instance_marker",
        ),
        ("template", serde_json::json!(1), "not_template"),
        ("template", serde_json::json!(true), "not_template"),
        ("lock", serde_json::json!("sensitive-marker"), "unlocked"),
    ] {
        let mut before = lifecycle_config();
        before["data"][field] = value;
        let fixture = lifecycle_fixture(
            before.clone(),
            serde_json::json!({"data":[]}),
            "stopped",
            None,
            before,
        )
        .await;
        let provider = fixture.client();
        let result = lifecycle_summary(&provider)
            .lifecycle_preflight(&provider)
            .await
            .unwrap();
        assert_eq!(result.start_preconditions, Status::Failed);
        assert_eq!(
            result
                .checks
                .iter()
                .find(|check| check.code == blocked)
                .unwrap()
                .status,
            Status::Failed
        );
        assert!(!result.execution_enabled);
    }
    for pending in [
        serde_json::json!({"key":"memory","pending":"4096"}),
        serde_json::json!({"key":"scsi0","delete":1}),
        serde_json::json!({"key":"scsi0","delete":2}),
    ] {
        let fixture = lifecycle_fixture(
            lifecycle_config(),
            serde_json::json!({"data":[pending]}),
            "stopped",
            None,
            lifecycle_config(),
        )
        .await;
        let provider = fixture.client();
        let result = lifecycle_summary(&provider)
            .lifecycle_preflight(&provider)
            .await
            .unwrap();
        assert_eq!(result.start_preconditions, Status::Failed);
        assert_eq!(
            result
                .checks
                .iter()
                .find(|check| check.code == "no_pending_changes")
                .unwrap()
                .status,
            Status::Failed
        );
    }
    let mut after = lifecycle_config();
    after["data"]["digest"] = serde_json::json!("c".repeat(40));
    let fixture = lifecycle_fixture(
        lifecycle_config(),
        serde_json::json!({"data":[]}),
        "stopped",
        None,
        after,
    )
    .await;
    let provider = fixture.client();
    let result = lifecycle_summary(&provider)
        .lifecycle_preflight(&provider)
        .await
        .unwrap();
    assert_eq!(result.start_preconditions, Status::Failed);
    assert_eq!(
        result
            .checks
            .iter()
            .find(|check| check.code == "configuration_unchanged")
            .unwrap()
            .status,
        Status::Failed
    );
}

#[tokio::test]
async fn lifecycle_preflight_rejects_incomplete_config_and_pending_metadata() {
    use crate::instance::ObservationError;
    for (field, value) in [
        ("digest", serde_json::json!("sensitive-marker")),
        ("template", serde_json::json!(2)),
        ("lock", serde_json::Value::Null),
        ("tags", serde_json::json!(42)),
        ("tags", serde_json::json!("lab;;ambiguous")),
        ("tags", serde_json::json!("lab; ambiguous")),
    ] {
        let mut before = lifecycle_config();
        before["data"][field] = value;
        let fixture = lifecycle_fixture(
            before,
            serde_json::json!({"data":[]}),
            "stopped",
            None,
            lifecycle_config(),
        )
        .await;
        let provider = fixture.client();
        assert!(matches!(
            lifecycle_summary(&provider)
                .lifecycle_preflight(&provider)
                .await,
            Err(ObservationError::Provider(Error::InvalidResponse))
        ));
    }
    for pending in [
        serde_json::json!({"data":null}),
        serde_json::json!({"data":[{"key":"memory","pending":null}]}),
        serde_json::json!({"data":[{"key":"memory","delete":3}]}),
        serde_json::json!({"data":[{"key":"memory"},{"key":"memory"}]}),
        serde_json::json!({"data":[{"value":"no-key"}]}),
    ] {
        let fixture = lifecycle_fixture(
            lifecycle_config(),
            pending,
            "stopped",
            None,
            lifecycle_config(),
        )
        .await;
        let provider = fixture.client();
        assert!(matches!(
            lifecycle_summary(&provider)
                .lifecycle_preflight(&provider)
                .await,
            Err(ObservationError::Provider(Error::InvalidResponse))
        ));
    }
}

#[tokio::test]
async fn lifecycle_preflight_scope_reference_and_transport_fail_closed() {
    use crate::instance::ObservationError;
    let mut fixture = Fixture::new("403 Forbidden", "sensitive-marker", "", Duration::ZERO).await;
    let provider = fixture.client();
    let mut instance = lifecycle_summary(&provider);
    instance.provider_reference.scope_id = "f".repeat(64);
    assert!(matches!(
        instance.lifecycle_preflight(&provider).await,
        Err(ObservationError::ScopeChanged)
    ));
    instance = lifecycle_summary(&provider);
    instance.id = "bad-id".into();
    assert!(matches!(
        instance.lifecycle_preflight(&provider).await,
        Err(ObservationError::InvalidReference)
    ));
    assert!(fixture.requests.try_recv().is_err());
    assert!(matches!(
        lifecycle_summary(&provider)
            .lifecycle_preflight(&provider)
            .await,
        Err(ObservationError::Provider(Error::Forbidden))
    ));
    let untrusted = Proxmox::new(&fixture.endpoint, "reader@pve!test", "fake", None).unwrap();
    assert!(matches!(
        lifecycle_summary(&untrusted)
            .lifecycle_preflight(&untrusted)
            .await,
        Err(ObservationError::Provider(Error::Unavailable))
    ));
}

#[tokio::test]
async fn lifecycle_preflight_api_returns_scoped_blockers_without_raw_configuration() {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let mut config = lifecycle_config();
    config["data"].as_object_mut().unwrap().remove("tags");
    let fixture = lifecycle_fixture(
        config.clone(),
        serde_json::json!({"data":[]}),
        "stopped",
        None,
        config,
    )
    .await;
    let provider = fixture.client();
    let store = crate::store::Store::memory().unwrap();
    let id = registered_fixture_instance(&store, &provider.scope_id());
    let app = crate::router_with_services(Some(provider), store.clone());
    let path = format!("/api/v1/environments/{id}/lifecycle-preflight");
    let response = app
        .clone()
        .oneshot(Request::builder().uri(&path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 8192).await.unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("sensitive-marker"));
    let report: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(report["instanceId"], id);
    assert_eq!(report["expectedMarker"], format!("vmanyware-{id}"));
    assert_eq!(report["startPreconditions"], "failed");
    assert_eq!(report["executionEnabled"], false);
    assert_eq!(store.recent_operations().unwrap().len(), 0);
    assert_eq!(
        serde_json::to_value(store.instance(&id).unwrap().unwrap()).unwrap()["status"],
        "registered"
    );
    assert_eq!(
        app.oneshot(
            Request::builder()
                .method("POST")
                .uri(&path)
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap()
        .status(),
        StatusCode::METHOD_NOT_ALLOWED
    );
}

#[tokio::test]
async fn lifecycle_dispatch_sends_only_fixed_action_and_validates_task_identity() {
    use crate::lifecycle::Action;
    let target = super::VmReference::new("pve", 202).unwrap();
    for (action, suffix, kind, body) in [
        (Action::Start, "start", "qmstart", ""),
        (
            Action::Shutdown,
            "shutdown",
            "qmshutdown",
            "timeout=60&forceStop=0",
        ),
    ] {
        let upid = format!("UPID:pve:00000001:00000002:00000003:{kind}:202:fixture@pve:");
        let response = serde_json::json!({"data":upid}).to_string();
        let mut fixture = Fixture::new("200 OK", &response, "", Duration::ZERO).await;
        let task = fixture
            .client()
            .dispatch_lifecycle(&target, action)
            .await
            .unwrap();
        assert!(task.matches_action(&target, action));
        let request = fixture.requests.recv().await.unwrap();
        assert!(request.starts_with(&format!(
            "POST /api2/json/nodes/pve/qemu/202/status/{suffix} HTTP/1.1\r\n"
        )));
        assert_eq!(request.split_once("\r\n\r\n").unwrap().1, body);
        assert!(request
            .to_lowercase()
            .contains("content-type: application/x-www-form-urlencoded"));
        assert!(request.contains("PVEAPIToken=reader@pve!vmanyware=fake-test-secret"));
        assert!(!request.contains("skiplock"));
        assert!(!format!("{task:?}").contains("fixture@pve"));
        assert!(fixture.requests.try_recv().is_err());
    }
}

#[tokio::test]
async fn lifecycle_dispatch_rejects_unbound_and_malformed_task_replies_without_replay() {
    use crate::lifecycle::Action;
    let target = super::VmReference::new("pve", 202).unwrap();
    for data in [
        serde_json::json!(null),
        serde_json::json!({"upid":"hidden"}),
        serde_json::json!("UPID:other:00000001:00000002:00000003:qmstart:202:fixture@pve:"),
        serde_json::json!("UPID:pve:00000001:00000002:00000003:qmshutdown:202:fixture@pve:"),
        serde_json::json!("UPID:pve:00000001:00000002:00000003:qmstart:203:fixture@pve:"),
        serde_json::json!("UPID:pve:00000001:00000002:00000003:qmstart:0202:fixture@pve:"),
    ] {
        let mut fixture = Fixture::new(
            "200 OK",
            &serde_json::json!({"data":data}).to_string(),
            "",
            Duration::ZERO,
        )
        .await;
        assert_eq!(
            fixture
                .client()
                .dispatch_lifecycle(&target, Action::Start)
                .await,
            Err(Error::InvalidResponse)
        );
        fixture.requests.recv().await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(30), fixture.requests.recv())
                .await
                .is_err()
        );
    }
}

#[tokio::test]
async fn lifecycle_dispatch_preserves_tls_errors_bounds_and_refuses_redirects() {
    use crate::lifecycle::Action;
    let target = super::VmReference::new("pve", 202).unwrap();
    for (status, headers, expected) in [
        ("401 Unauthorized", "", Error::Unauthorized),
        ("403 Forbidden", "", Error::Forbidden),
        ("500 Internal Server Error", "", Error::Provider),
        (
            "307 Temporary Redirect",
            "Location: https://localhost:1/leak\r\n",
            Error::Provider,
        ),
    ] {
        let mut fixture =
            Fixture::new(status, "private provider message", headers, Duration::ZERO).await;
        assert_eq!(
            fixture
                .client()
                .dispatch_lifecycle(&target, Action::Start)
                .await,
            Err(expected)
        );
        fixture.requests.recv().await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(30), fixture.requests.recv())
                .await
                .is_err()
        );
    }
    let fixture = Fixture::new("200 OK", "{}", "", Duration::ZERO).await;
    let untrusted = Proxmox::new(&fixture.endpoint, "reader@pve!test", "fake", None).unwrap();
    assert_eq!(
        untrusted.dispatch_lifecycle(&target, Action::Start).await,
        Err(Error::Unavailable)
    );
    let mut fixture = Fixture::new("200 OK", "{}", "", Duration::from_secs(1)).await;
    let client = Proxmox::build(
        &fixture.endpoint,
        "reader@pve!test",
        "fake",
        Some(&fixture.ca),
        Duration::from_millis(30),
    )
    .unwrap();
    assert_eq!(
        client.dispatch_lifecycle(&target, Action::Start).await,
        Err(Error::Timeout)
    );
    fixture.requests.recv().await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(30), fixture.requests.recv())
            .await
            .is_err()
    );
    let fixture = Fixture::new(
        "200 OK",
        &"x".repeat(MAX_RESPONSE_BYTES + 1),
        "",
        Duration::ZERO,
    )
    .await;
    assert_eq!(
        fixture
            .client()
            .dispatch_lifecycle(&target, Action::Start)
            .await,
        Err(Error::ResponseTooLarge)
    );
}

const EXECUTOR_UPID: &str = "UPID:pve-lab:00000001:00000002:00000003:qmstart:202:fixture@pve:";

async fn executor_fixture(
    reply: serde_json::Value,
    task_exit: &str,
    final_power: &str,
    delay: Duration,
) -> Fixture {
    let config = lifecycle_config().to_string();
    Fixture::responses("200 OK", &[
        ("/api2/json/nodes/pve-lab/qemu/202/config?current=1", &config),
        ("/api2/json/nodes/pve-lab/qemu/202/pending", r#"{"data":[]}"#),
        ("/api2/json/nodes/pve-lab/qemu/202/status/current", r#"{"data":{"vmid":202,"status":"stopped"}}"#),
        ("/api2/json/nodes/pve-lab/qemu/202/config?current=1", &config),
        ("/api2/json/nodes/pve-lab/qemu/202/status/start", &serde_json::json!({"data":reply}).to_string()),
        ("*", &serde_json::json!({"data":{"node":"pve-lab","upid":EXECUTOR_UPID,"status":"stopped","exitstatus":task_exit}}).to_string()),
        ("/api2/json/nodes/pve-lab/qemu/202/status/current", &serde_json::json!({"data":{"vmid":202,"status":final_power}}).to_string()),
    ], "", delay, true).await
}

fn executor_store(
    provider: &Proxmox,
) -> (
    tempfile::TempDir,
    crate::store::Store,
    crate::lifecycle::Approval,
) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("executor.sqlite");
    let store = crate::store::Store::open(&path).unwrap();
    let id = registered_fixture_instance(&store, &provider.scope_id());
    // Deterministic fixture identity, before any operation or foreign-key reference.
    rusqlite::Connection::open(path)
        .unwrap()
        .execute(
            "UPDATE instances SET id=?1 WHERE id=?2",
            [&"a".repeat(32), &id],
        )
        .unwrap();
    let instance = store.instance(&"a".repeat(32)).unwrap().unwrap();
    let approval = crate::lifecycle::Approval {
        action: crate::lifecycle::Action::Start,
        instance_id: instance.summary.id,
        provider_scope_id: provider.scope_id(),
        node: "pve-lab".into(),
        vmid: 202,
        definition_hash: instance.summary.definition_hash,
        config_digest: "b".repeat(40),
        confirmed: true,
    };
    (directory, store, approval)
}

#[tokio::test]
async fn lifecycle_executor_default_disabled_and_changed_scope_or_revision_never_post() {
    use crate::{
        lifecycle_executor,
        operation::{Failure, Status},
    };
    for variant in ["disabled", "scope", "revision", "expiry"] {
        let mut fixture = executor_fixture(
            serde_json::json!(EXECUTOR_UPID),
            "OK",
            "running",
            Duration::ZERO,
        )
        .await;
        let provider = fixture.client();
        let (directory, store, mut approval) = executor_store(&provider);
        if variant == "revision" {
            approval.config_digest = "c".repeat(40);
        }
        let operation = store.enqueue_lifecycle("execute", &approval).unwrap();
        if variant == "scope" {
            // A different endpoint can never be used for this retained approval.
            let other = Fixture::new("200 OK", "{}", "", Duration::ZERO).await;
            assert!(
                lifecycle_executor::run_one(&store, Some(&other.client()), true)
                    .await
                    .unwrap()
            );
        } else {
            if variant == "expiry" {
                rusqlite::Connection::open(directory.path().join("executor.sqlite"))
                    .unwrap()
                    .execute("UPDATE operations SET confirmed_at=unixepoch()-121", [])
                    .unwrap();
            }
            assert!(
                lifecycle_executor::run_one(&store, Some(&provider), variant != "disabled")
                    .await
                    .unwrap()
            );
        }
        let result = store.operation(&operation.id).unwrap().unwrap();
        assert_eq!(result.status, Status::Failed);
        assert_eq!(
            result.failure,
            Some(match variant {
                "disabled" => Failure::LifecycleDisabled,
                "scope" => Failure::ProviderScopeChanged,
                "expiry" => Failure::ApprovalExpired,
                _ => Failure::LifecycleTargetChanged,
            })
        );
        while let Ok(request) = fixture.requests.try_recv() {
            assert!(request.starts_with("GET "));
        }
    }
}

#[tokio::test]
async fn lifecycle_executor_persists_task_and_separate_power_evidence_or_uncertainty() {
    use crate::{
        lifecycle_executor,
        operation::{Failure, Status},
    };
    for (reply, exit, power, expected, failure) in [
        (
            serde_json::json!(EXECUTOR_UPID),
            "OK",
            "running",
            Status::Succeeded,
            None,
        ),
        (
            serde_json::json!(EXECUTOR_UPID),
            "private failure details",
            "running",
            Status::Failed,
            Some(Failure::ProviderTaskFailed),
        ),
        (
            serde_json::json!(EXECUTOR_UPID),
            "OK",
            "stopped",
            Status::Uncertain,
            Some(Failure::PowerStateUnconfirmed),
        ),
        (
            serde_json::json!(null),
            "OK",
            "running",
            Status::Uncertain,
            Some(Failure::ProviderSubmissionUnconfirmed),
        ),
    ] {
        let mut fixture = executor_fixture(reply, exit, power, Duration::ZERO).await;
        let provider = fixture.client();
        let (_directory, store, approval) = executor_store(&provider);
        let operation = store.enqueue_lifecycle("execute", &approval).unwrap();
        assert!(lifecycle_executor::run_one(&store, Some(&provider), true)
            .await
            .unwrap());
        let result = store.operation(&operation.id).unwrap().unwrap();
        assert_eq!(result.status, expected);
        assert_eq!(result.failure, failure);
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert_eq!(
            requests.iter().filter(|r| r.starts_with("POST ")).count(),
            1
        );
        assert_eq!(
            requests.len(),
            if expected == Status::Succeeded || failure == Some(Failure::PowerStateUnconfirmed) {
                7
            } else if failure == Some(Failure::ProviderTaskFailed) {
                6
            } else {
                5
            }
        );
        assert!(!serde_json::to_string(&result)
            .unwrap()
            .contains("fixture@pve"));
        assert_eq!(
            store.enqueue_lifecycle("execute", &approval).unwrap().id,
            operation.id
        );
        if expected == Status::Uncertain {
            assert!(store.enqueue_lifecycle("new-key", &approval).is_err());
        }
    }
}

#[tokio::test]
async fn lifecycle_executor_interruption_recovers_without_post_replay_with_or_without_task() {
    use crate::{lifecycle_executor, operation::Status};
    for cut_after in [5, 6] {
        let mut fixture = executor_fixture(
            serde_json::json!(EXECUTOR_UPID),
            "OK",
            "running",
            Duration::from_millis(75),
        )
        .await;
        let provider = fixture.client();
        let (directory, store, approval) = executor_store(&provider);
        let operation = store.enqueue_lifecycle("execute", &approval).unwrap();
        let database = store.clone();
        let peer = provider.clone();
        let worker =
            tokio::spawn(
                async move { lifecycle_executor::run_one(&database, Some(&peer), true).await },
            );
        let mut requests = Vec::new();
        for _ in 0..cut_after {
            requests.push(
                tokio::time::timeout(Duration::from_secs(3), fixture.requests.recv())
                    .await
                    .unwrap()
                    .unwrap(),
            );
        }
        worker.abort();
        assert!(worker.await.unwrap_err().is_cancelled());
        let interrupted = store.lifecycle_job(&operation.id).unwrap().unwrap();
        assert!(interrupted.dispatch_started);
        assert_eq!(interrupted.provider_task.is_some(), cut_after == 6);
        drop(store);
        let reopened = crate::store::Store::open(directory.path().join("executor.sqlite")).unwrap();
        assert_eq!(reopened.recover_interrupted_operations().unwrap(), 1);
        assert_eq!(
            reopened.operation(&operation.id).unwrap().unwrap().status,
            Status::Uncertain
        );
        assert_eq!(
            reopened.enqueue_lifecycle("execute", &approval).unwrap().id,
            operation.id
        );
        assert!(reopened
            .enqueue_lifecycle("another-key", &approval)
            .is_err());
        // Disable writes after restart. Known tasks still get read-only diagnosis.
        assert_eq!(
            lifecycle_executor::run_one(&reopened, Some(&provider), false)
                .await
                .unwrap(),
            cut_after == 6
        );
        while let Ok(request) = fixture.requests.try_recv() {
            requests.push(request);
        }
        assert_eq!(
            requests.iter().filter(|r| r.starts_with("POST ")).count(),
            1
        );
        assert_eq!(requests.len(), cut_after + usize::from(cut_after == 6));
        if cut_after == 6 {
            assert!(requests.last().unwrap().contains("/tasks/"));
        }
        assert_eq!(
            reopened.operation(&operation.id).unwrap().unwrap().status,
            Status::Uncertain
        );
    }
}

#[tokio::test]
async fn lifecycle_executor_blocks_foreign_marker_before_dispatch() {
    use crate::{
        lifecycle_executor,
        operation::{Failure, Status},
    };
    let config =
        serde_json::json!({"data":{"digest":"b".repeat(40),"tags":"foreign","template":0}})
            .to_string();
    let mut fixture = Fixture::routes(
        "200 OK",
        &[
            (
                "/api2/json/nodes/pve-lab/qemu/202/config?current=1",
                &config,
            ),
            (
                "/api2/json/nodes/pve-lab/qemu/202/pending",
                r#"{"data":[]}"#,
            ),
            (
                "/api2/json/nodes/pve-lab/qemu/202/status/current",
                r#"{"data":{"vmid":202,"status":"stopped"}}"#,
            ),
        ],
        "",
        Duration::ZERO,
    )
    .await;
    let provider = fixture.client();
    let (_directory, store, approval) = executor_store(&provider);
    let operation = store.enqueue_lifecycle("execute", &approval).unwrap();
    lifecycle_executor::run_one(&store, Some(&provider), true)
        .await
        .unwrap();
    let result = store.operation(&operation.id).unwrap().unwrap();
    assert_eq!(result.status, Status::Failed);
    assert_eq!(result.failure, Some(Failure::LifecyclePreconditionsFailed));
    let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
    assert_eq!(requests.len(), 4);
    assert!(requests.iter().all(|request| request.starts_with("GET ")));
    assert!(
        !store
            .lifecycle_job(&operation.id)
            .unwrap()
            .unwrap()
            .dispatch_started
    );
}

#[tokio::test]
async fn lifecycle_executor_shutdown_tracks_task_then_independent_stopped_power() {
    use crate::{
        lifecycle::{Action, Outcome},
        lifecycle_executor,
        operation::Status,
    };
    let config = lifecycle_config().to_string();
    let upid = "UPID:pve-lab:00000001:00000002:00000003:qmshutdown:202:fixture@pve:";
    let mut fixture = Fixture::responses("200 OK", &[
        ("/api2/json/nodes/pve-lab/qemu/202/config?current=1", &config),
        ("/api2/json/nodes/pve-lab/qemu/202/pending", r#"{"data":[]}"#),
        ("/api2/json/nodes/pve-lab/qemu/202/status/current", r#"{"data":{"vmid":202,"status":"running","qmpstatus":"running"}}"#),
        ("/api2/json/nodes/pve-lab/qemu/202/config?current=1", &config),
        ("/api2/json/nodes/pve-lab/qemu/202/status/shutdown", &serde_json::json!({"data":upid}).to_string()),
        ("*", &serde_json::json!({"data":{"node":"pve-lab","upid":upid,"status":"stopped","exitstatus":"OK"}}).to_string()),
        ("/api2/json/nodes/pve-lab/qemu/202/status/current", r#"{"data":{"vmid":202,"status":"stopped"}}"#),
    ], "", Duration::ZERO, true).await;
    let provider = fixture.client();
    let (_directory, store, mut approval) = executor_store(&provider);
    approval.action = Action::Shutdown;
    let operation = store.enqueue_lifecycle("execute", &approval).unwrap();
    lifecycle_executor::run_one(&store, Some(&provider), true)
        .await
        .unwrap();
    let result = store.operation(&operation.id).unwrap().unwrap();
    assert_eq!(result.status, Status::Succeeded);
    let evidence = result.result.unwrap();
    assert_eq!(evidence["action"], "shutdown");
    assert_eq!(evidence["taskStatus"], "succeeded");
    assert_eq!(evidence["observation"]["status"], "stopped");
    let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
    assert_eq!(requests.len(), 7);
    assert!(requests[4].starts_with("POST /api2/json/nodes/pve-lab/qemu/202/status/shutdown "));
    assert!(requests[4].ends_with("timeout=60&forceStop=0"));
    assert!(requests[5].starts_with("GET ") && requests[5].contains("/tasks/"));
    assert!(requests[6].starts_with("GET ") && requests[6].contains("/status/current"));
    // Terminal results cannot be rewritten through the lifecycle path.
    assert!(store
        .finish_lifecycle(
            &operation.id,
            Outcome::Uncertain(crate::operation::Failure::ProcessInterrupted)
        )
        .is_err());
}

#[tokio::test]
async fn lifecycle_executor_reconciles_retained_task_to_success_even_with_writes_disabled() {
    use crate::{lifecycle_executor, operation::Status};
    let mut fixture = Fixture::routes("200 OK", &[
        ("/api2/json/nodes/pve-lab/qemu/202/status/current", r#"{"data":{"vmid":202,"status":"running","qmpstatus":"prelaunch"}}"#),
        ("*", &serde_json::json!({"data":{"node":"pve-lab","upid":EXECUTOR_UPID,"status":"stopped","exitstatus":"OK"}}).to_string()),
    ], "", Duration::ZERO).await;
    let provider = fixture.client();
    let (directory, store, approval) = executor_store(&provider);
    let operation = store.enqueue_lifecycle("execute", &approval).unwrap();
    store.claim_lifecycle().unwrap().unwrap();
    store
        .begin_lifecycle_dispatch(&operation.id, &approval)
        .unwrap();
    store
        .link_lifecycle_task(
            &operation.id,
            &super::TaskReference::new("pve-lab", EXECUTOR_UPID).unwrap(),
        )
        .unwrap();
    // Seed the precise persisted crash boundary after task linkage; no POST in this test.
    drop(store);
    let store = crate::store::Store::open(directory.path().join("executor.sqlite")).unwrap();
    assert_eq!(store.recover_interrupted_operations().unwrap(), 1);
    assert!(lifecycle_executor::run_one(&store, Some(&provider), false)
        .await
        .unwrap());
    let result = store.operation(&operation.id).unwrap().unwrap();
    assert_eq!(result.status, Status::Succeeded);
    assert_eq!(
        result.result.unwrap()["observation"]["qmpStatus"],
        "prelaunch"
    );
    let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
    assert_eq!(requests.len(), 2);
    assert!(requests.iter().all(|request| request.starts_with("GET ")));
    assert!(requests[0].contains("/tasks/"));
    assert!(requests[1].contains("/status/current"));
    assert!(!lifecycle_executor::run_one(&store, Some(&provider), true)
        .await
        .unwrap());
}

async fn lifecycle_http(
    app: axum::Router,
    id: &str,
    suffix: &str,
    key: Option<&str>,
    origin: bool,
    body: serde_json::Value,
) -> (axum::http::StatusCode, serde_json::Value, Option<String>) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use tower::ServiceExt;
    let mut request = Request::post(format!("/api/v1/environments/{id}/{suffix}"))
        .header("content-type", "application/json");
    if let Some(key) = key {
        request = request.header("idempotency-key", key);
    }
    if origin {
        request = request.header("origin", "https://untrusted.invalid");
    }
    let response = app
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let location = response
        .headers()
        .get("location")
        .map(|v| v.to_str().unwrap().to_owned());
    let status = response.status();
    let body =
        serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
    (status, body, location)
}

#[tokio::test]
async fn lifecycle_http_requires_opt_in_scope_and_complete_action_bound_confirmation() {
    use axum::http::StatusCode;
    let mut fixture = Fixture::new("200 OK", "{}", "", Duration::ZERO).await;
    let provider = fixture.client();
    let (_directory, store, approval) = executor_store(&provider);
    let body = serde_json::to_value(&approval).unwrap();
    let app = crate::router_with_lifecycle(Some(provider.clone()), store.clone(), None, true);
    for (field, value, expected) in [
        (
            "confirmed",
            serde_json::json!(false),
            StatusCode::BAD_REQUEST,
        ),
        (
            "action",
            serde_json::json!("shutdown"),
            StatusCode::BAD_REQUEST,
        ),
        (
            "instanceId",
            serde_json::json!("f".repeat(32)),
            StatusCode::BAD_REQUEST,
        ),
        (
            "providerScopeId",
            serde_json::json!("f".repeat(64)),
            StatusCode::CONFLICT,
        ),
        (
            "definitionHash",
            serde_json::json!("f".repeat(64)),
            StatusCode::CONFLICT,
        ),
        ("vmid", serde_json::json!(203), StatusCode::CONFLICT),
        (
            "configDigest",
            serde_json::json!("invalid"),
            StatusCode::BAD_REQUEST,
        ),
        (
            "forceStop",
            serde_json::json!(true),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        let mut input = body.clone();
        input[field] = value;
        let (status, response, _) = lifecycle_http(
            app.clone(),
            &approval.instance_id,
            "start",
            Some("test"),
            false,
            input,
        )
        .await;
        assert_eq!(status, expected, "{field}: {response}");
    }
    for (router, key, origin, expected) in [
        (
            crate::router_with_services(Some(provider.clone()), store.clone()),
            Some("test"),
            false,
            StatusCode::FORBIDDEN,
        ),
        (app.clone(), None, false, StatusCode::BAD_REQUEST),
        (app.clone(), Some("test"), true, StatusCode::FORBIDDEN),
        (
            crate::router_with_lifecycle(None, store.clone(), None, true),
            Some("test"),
            false,
            StatusCode::SERVICE_UNAVAILABLE,
        ),
    ] {
        assert_eq!(
            lifecycle_http(
                router,
                &approval.instance_id,
                "start",
                key,
                origin,
                body.clone()
            )
            .await
            .0,
            expected
        );
    }
    let mut oversized = body.clone();
    oversized["padding"] = serde_json::json!("x".repeat(5000));
    assert_eq!(
        lifecycle_http(
            app.clone(),
            &approval.instance_id,
            "start",
            Some("oversized"),
            false,
            oversized
        )
        .await
        .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert!(store.recent_operations().unwrap().is_empty());
    let (status, first, location) = lifecycle_http(
        app.clone(),
        &approval.instance_id,
        "start",
        Some("test"),
        false,
        body.clone(),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(first["status"], "queued");
    assert_eq!(first["kind"], "vm_start");
    assert_eq!(
        location.unwrap(),
        format!("/api/v1/operations/{}", first["id"].as_str().unwrap())
    );
    let duplicate = lifecycle_http(
        app.clone(),
        &approval.instance_id,
        "start",
        Some("test"),
        false,
        body.clone(),
    )
    .await;
    assert_eq!(duplicate.0, StatusCode::ACCEPTED);
    assert_eq!(duplicate.1, first);
    let mut changed = body.clone();
    changed["configDigest"] = serde_json::json!("c".repeat(40));
    assert_eq!(
        lifecycle_http(
            app.clone(),
            &approval.instance_id,
            "start",
            Some("test"),
            false,
            changed
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        lifecycle_http(
            app.clone(),
            &approval.instance_id,
            "start",
            Some("new-key"),
            false,
            body.clone()
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let mut shutdown = body;
    shutdown["action"] = serde_json::json!("shutdown");
    assert_eq!(
        lifecycle_http(
            app,
            &approval.instance_id,
            "shutdown",
            Some("test"),
            false,
            shutdown
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(store.recent_operations().unwrap().len(), 1);
    assert!(
        fixture.requests.try_recv().is_err(),
        "enqueue must not contact the provider"
    );
}

#[tokio::test]
async fn lifecycle_http_enqueue_drives_actual_executor_and_preflight_exposes_capability() {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use tower::ServiceExt;
    let mut fixture = executor_fixture(
        serde_json::json!(EXECUTOR_UPID),
        "OK",
        "running",
        Duration::ZERO,
    )
    .await;
    let provider = fixture.client();
    let (_directory, store, approval) = executor_store(&provider);
    let app = crate::router_with_lifecycle(Some(provider.clone()), store.clone(), None, true);
    let (status, operation, _) = lifecycle_http(
        app.clone(),
        &approval.instance_id,
        "start",
        Some("confirmed"),
        false,
        serde_json::to_value(&approval).unwrap(),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::ACCEPTED);
    assert!(fixture.requests.try_recv().is_err());
    assert!(crate::worker::run_round(&store, Some(&provider), true)
        .await
        .unwrap());
    assert_eq!(
        store
            .operation(operation["id"].as_str().unwrap())
            .unwrap()
            .unwrap()
            .status,
        crate::operation::Status::Succeeded
    );
    let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
    assert_eq!(
        requests.iter().filter(|r| r.starts_with("POST ")).count(),
        1
    );
    let fixture = lifecycle_fixture(
        lifecycle_config(),
        serde_json::json!({"data":[]}),
        "stopped",
        None,
        lifecycle_config(),
    )
    .await;
    let provider = fixture.client();
    let (_directory, store, approval) = executor_store(&provider);
    let response = crate::router_with_lifecycle(Some(provider), store, None, true)
        .oneshot(
            Request::get(format!(
                "/api/v1/environments/{}/lifecycle-preflight",
                approval.instance_id
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let report: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
    assert_eq!(report["executionEnabled"], true);
    assert_eq!(report["startPreconditions"], "passed");
}

fn full_clone_plan() -> crate::provisioning::Plan {
    crate::provisioning::Catalog::parse(include_str!("../../../examples/templates.yaml"))
        .unwrap()
        .plan(
            &crate::definition::Definition::parse(include_str!(
                "../../../examples/kernel-lab.yaml"
            ))
            .unwrap(),
            202,
        )
        .unwrap()
}
fn clone_description(plan: &crate::provisioning::Plan) -> String {
    format!(
        "VMAnyware instance {}\ndefinition-sha256: {}\nplan-sha256: {}",
        "a".repeat(32),
        plan.definition_hash,
        plan.plan_hash
    )
}

#[tokio::test]
async fn full_clone_posts_fixed_form_and_validates_task_source_not_target() {
    let plan = full_clone_plan();
    let request = plan.clone_request(&"a".repeat(32)).unwrap();
    let upid = "UPID:pve-test:00000001:00000002:00000003:qmclone:9000:fixture@pve:";
    let mut fixture = Fixture::new(
        "200 OK",
        &serde_json::json!({"data":upid}).to_string(),
        "",
        Duration::ZERO,
    )
    .await;
    let task = fixture
        .client()
        .dispatch_full_clone(&request)
        .await
        .unwrap();
    assert!(task.matches_clone(&super::VmReference::new("pve-test", 9000).unwrap()));
    assert!(!task.matches_clone(&super::VmReference::new("pve-test", 202).unwrap()));
    assert!(!format!("{task:?}").contains("fixture@pve"));
    let posted = fixture.requests.recv().await.unwrap();
    assert!(posted.starts_with("POST /api2/json/nodes/pve-test/qemu/9000/clone HTTP/1.1\r\n"));
    assert!(posted
        .to_ascii_lowercase()
        .contains("content-type: application/x-www-form-urlencoded"));
    let form: Vec<_> = form_urlencoded::parse(posted.split_once("\r\n\r\n").unwrap().1.as_bytes())
        .into_owned()
        .collect();
    assert_eq!(
        form,
        vec![
            ("newid".into(), "202".into()),
            ("name".into(), "kernel-lab".into()),
            ("storage".into(), "local-lvm".into()),
            ("full".into(), "1".into()),
            ("description".into(), clone_description(&plan))
        ]
    );
    assert!(posted.contains("%0A"));
    assert!(
        !posted.contains("skiplock") && !posted.contains("snapname") && !posted.contains("target=")
    );
    assert!(fixture.requests.try_recv().is_err());
    for upid in [
        "UPID:pve-test:00000001:00000002:00000003:qmclone:202:fixture@pve:",
        "UPID:pve-test:00000001:00000002:00000003:qmclone:09000:fixture@pve:",
        "UPID:pve-test:00000001:00000002:00000003:qmstart:9000:fixture@pve:",
        "UPID:foreign:00000001:00000002:00000003:qmclone:9000:fixture@pve:",
    ] {
        let mut fixture = Fixture::new(
            "200 OK",
            &serde_json::json!({"data":upid}).to_string(),
            "",
            Duration::ZERO,
        )
        .await;
        assert_eq!(
            fixture
                .client()
                .dispatch_full_clone(&request)
                .await
                .unwrap_err(),
            Error::InvalidResponse
        );
        fixture.requests.recv().await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(30), fixture.requests.recv())
                .await
                .is_err()
        );
    }
}

#[tokio::test]
async fn full_clone_observation_binds_exact_marker_name_digest_and_stopped_target() {
    let plan = full_clone_plan();
    let request = plan.clone_request(&"a".repeat(32)).unwrap();
    let baseline = serde_json::json!({"digest":"d".repeat(40),"description":clone_description(&plan),"name":"kernel-lab","template":0});
    for variant in [
        "valid", "marker", "name", "digest", "lock", "template", "running", "qmp",
    ] {
        let mut config = baseline.clone();
        match variant {
            "marker" => config["description"] = serde_json::json!("another-target"),
            "name" => config["name"] = serde_json::json!("other-name"),
            "digest" => config["digest"] = serde_json::json!("invalid"),
            "lock" => config["lock"] = serde_json::json!("clone"),
            "template" => config["template"] = serde_json::json!(1),
            _ => {}
        }
        let mut fixture = Fixture::responses("200 OK", &[
            ("/api2/json/nodes/pve-test/qemu/202/config?current=1", &serde_json::json!({"data":config}).to_string()),
            ("/api2/json/nodes/pve-test/qemu/202/status/current", &serde_json::json!({"data":{"vmid":202,"status":if variant == "running" {"running"} else {"stopped"},"qmpstatus":if variant == "qmp" {Some("running")} else {None}}}).to_string()),
        ], "", Duration::ZERO, true).await;
        let observed = fixture.client().observe_full_clone(&request).await;
        if variant == "valid" {
            let observed = observed.unwrap();
            assert_eq!(observed.config_digest, "d".repeat(40));
            assert_eq!(observed.observation.vmid, 202);
            assert_eq!(observed.observation.status, super::PowerState::Stopped);
        } else {
            assert_eq!(observed.unwrap_err(), Error::InvalidResponse);
        }
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert!(requests.iter().all(|r| r.starts_with("GET ")));
        assert_eq!(
            requests.len(),
            if matches!(variant, "valid" | "running" | "qmp") {
                2
            } else {
                1
            }
        );
    }
    assert!(plan.clone_request("../escape").is_err());
    assert!(super::CloneRequest::new(
        super::VmReference::new("pve-test", 9000).unwrap(),
        super::VmReference::new("other", 202).unwrap(),
        "local-lvm",
        "kernel-lab",
        &"a".repeat(32),
        &plan.definition_hash,
        &plan.plan_hash
    )
    .is_err());
}

#[tokio::test]
async fn full_clone_submission_errors_are_bounded_and_never_replayed() {
    let request = full_clone_plan().clone_request(&"a".repeat(32)).unwrap();
    for (status, body, headers, expected) in [
        (
            "401 Unauthorized",
            "private response",
            "",
            Error::Unauthorized,
        ),
        ("403 Forbidden", "private response", "", Error::Forbidden),
        (
            "500 Internal Server Error",
            "private response",
            "",
            Error::Provider,
        ),
        (
            "307 Temporary Redirect",
            "private response",
            "Location: https://localhost:1/leak\r\n",
            Error::Provider,
        ),
        ("200 OK", "{\"data\":null}", "", Error::InvalidResponse),
    ] {
        let mut fixture = Fixture::new(status, body, headers, Duration::ZERO).await;
        assert_eq!(
            fixture
                .client()
                .dispatch_full_clone(&request)
                .await
                .unwrap_err(),
            expected
        );
        fixture.requests.recv().await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(30), fixture.requests.recv())
                .await
                .is_err()
        );
    }
    let mut fixture = Fixture::new("200 OK", "{}", "", Duration::from_secs(1)).await;
    let client = Proxmox::build(
        &fixture.endpoint,
        "reader@pve!fixture",
        "fake",
        Some(&fixture.ca),
        Duration::from_millis(30),
    )
    .unwrap();
    assert_eq!(
        client.dispatch_full_clone(&request).await.unwrap_err(),
        Error::Timeout
    );
    fixture.requests.recv().await.unwrap();
    assert!(fixture.requests.try_recv().is_err());
    let fixture = Fixture::new(
        "200 OK",
        &"x".repeat(MAX_RESPONSE_BYTES + 1),
        "",
        Duration::ZERO,
    )
    .await;
    assert_eq!(
        fixture
            .client()
            .dispatch_full_clone(&request)
            .await
            .unwrap_err(),
        Error::ResponseTooLarge
    );
}

async fn provisioning_executor_fixture(
    reply: serde_json::Value,
    exit: &str,
    marker_valid: bool,
    gate: &str,
) -> Fixture {
    let plan = full_clone_plan();
    let marker = if marker_valid {
        clone_description(&plan)
    } else {
        "foreign-marker".into()
    };
    let mut source = serde_json::json!({"data":{"digest":"c".repeat(40),"template":1,"scsi0":"local-lvm:base-9000-disk-0,size=8G","ide2":"local-lvm:vm-9000-cloudinit,media=cdrom"}});
    if gate == "revision_missing" {
        source["data"].as_object_mut().unwrap().remove("digest");
    }
    if gate == "locked" {
        source["data"]["lock"] = serde_json::json!("clone");
    }
    let source = source.to_string();
    let pending = if gate == "pending" {
        r#"{"data":[{"key":"memory","pending":"8192"}]}"#
    } else {
        r#"{"data":[]}"#
    };
    let available = if gate == "occupied" {
        r#"{"data":203}"#
    } else {
        r#"{"data":202}"#
    };
    let power=serde_json::json!({"data":{"vmid":9000,"status":if gate=="running" {"running"} else {"stopped"}}}).to_string();
    let upid = "UPID:pve-test:00000001:00000002:00000003:qmclone:9000:fixture@pve:";
    Fixture::routes("200 OK",&[
        ("/api2/json/nodes/pve-test/qemu/9000/config?current=1",&source),
        ("/api2/json/nodes/pve-test/storage",r#"{"data":[{"storage":"local-lvm","active":1,"enabled":1,"content":"images","avail":1099511627776}]}"#),
        ("/api2/json/nodes/pve-test/network",r#"{"data":[{"iface":"vmbr0","active":1,"type":"bridge"}]}"#),
        ("/api2/json/cluster/nextid?vmid=202",available),
        ("/api2/json/nodes/pve-test/qemu/9000/pending",pending),
        ("/api2/json/nodes/pve-test/qemu/9000/status/current",&power),
        ("/api2/json/nodes/pve-test/qemu/9000/clone",&serde_json::json!({"data":reply}).to_string()),
        ("/api2/json/nodes/pve-test/qemu/202/config?current=1",&serde_json::json!({"data":{"digest":"d".repeat(40),"description":marker,"name":"kernel-lab","template":0}}).to_string()),
        ("/api2/json/nodes/pve-test/qemu/202/status/current",r#"{"data":{"vmid":202,"status":"stopped"}}"#),
        ("*",&serde_json::json!({"data":{"node":"pve-test","upid":upid,"status":"stopped","exitstatus":exit}}).to_string()),
    ],"",Duration::ZERO).await
}
fn queued_clone(
    provider: &Proxmox,
) -> (
    tempfile::TempDir,
    crate::store::Store,
    crate::operation::Operation,
) {
    queued_clone_with_yaml(provider, include_str!("../../../examples/kernel-lab.yaml"))
}
fn queued_clone_with_yaml(
    provider: &Proxmox,
    yaml: &str,
) -> (
    tempfile::TempDir,
    crate::store::Store,
    crate::operation::Operation,
) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("clone.sqlite");
    let store = crate::store::Store::open(&path).unwrap();
    let definition = crate::definition::Definition::parse(yaml).unwrap();
    let final_snapshot = definition.spec.lifecycle.snapshot_after_provisioning;
    store.create(&definition).unwrap();
    let plan =
        crate::provisioning::Catalog::parse(include_str!("../../../examples/templates.yaml"))
            .unwrap()
            .plan(&definition, 202)
            .unwrap();
    let approval = crate::provisioning_intent::Approval {
        definition_name: definition.metadata.name,
        definition_hash: plan.definition_hash,
        provider_scope_id: provider.scope_id(),
        plan_hash: plan.plan_hash,
        target_node: "pve-test".into(),
        target_vmid: 202,
        source_vmid: 9000,
        storage: "local-lvm".into(),
        bridge: "vmbr0".into(),
        confirmed: true,
    };
    let catalog =
        crate::provisioning::Catalog::parse(include_str!("../../../examples/templates.yaml"))
            .unwrap();
    let operation = store
        .enqueue_provisioning("provision", &approval, &catalog)
        .unwrap();
    // Seed a deterministic fixture identity before claims/dispatch, preserving all FKs atomically.
    let mut connection = rusqlite::Connection::open(&path).unwrap();
    let transaction = connection.transaction().unwrap();
    transaction
        .pragma_update(None, "defer_foreign_keys", true)
        .unwrap();
    transaction
        .execute(
            "UPDATE instances SET id=?1 WHERE id=?2",
            rusqlite::params!["a".repeat(32), operation.instance_id],
        )
        .unwrap();
    transaction
        .execute(
            "UPDATE operations SET instance_id=?1 WHERE id=?2",
            rusqlite::params!["a".repeat(32), operation.id],
        )
        .unwrap();
    let operation_id = if final_snapshot {
        // Snapshot fixtures need deterministic metadata, before any dispatch.
        let id = "b".repeat(32);
        transaction
            .execute(
                "UPDATE operations SET id=?1 WHERE id=?2",
                rusqlite::params![id, operation.id],
            )
            .unwrap();
        transaction
            .execute(
                "UPDATE provisioning_intents SET operation_id=?1 WHERE operation_id=?2",
                rusqlite::params![id, operation.id],
            )
            .unwrap();
        transaction
            .execute(
                "UPDATE provisioning_stages SET operation_id=?1 WHERE operation_id=?2",
                rusqlite::params![id, operation.id],
            )
            .unwrap();
        id
    } else {
        operation.id
    };
    transaction.commit().unwrap();
    let operation = store.operation(&operation_id).unwrap().unwrap();
    (directory, store, operation)
}

#[tokio::test]
async fn provisioning_clone_executor_blocks_disabled_or_changed_catalog_without_post() {
    use crate::{
        operation::{Failure, Status},
        provisioning_executor,
    };
    let upid = "UPID:pve-test:00000001:00000002:00000003:qmclone:9000:fixture@pve:";
    for enabled in [false, true] {
        let mut fixture =
            provisioning_executor_fixture(serde_json::json!(upid), "OK", true, "valid").await;
        let provider = fixture.client();
        let (_directory, store, operation) = queued_clone(&provider);
        let catalog = crate::provisioning::Catalog::parse(
            &include_str!("../../../examples/templates.yaml").replace("local-lvm", "other-store"),
        )
        .unwrap();
        assert!(
            provisioning_executor::run_clone(&store, Some(&provider), Some(&catalog), enabled)
                .await
                .unwrap()
        );
        let result = store.operation(&operation.id).unwrap().unwrap();
        assert_eq!(result.status, Status::Failed);
        assert_eq!(
            result.failure,
            Some(if enabled {
                Failure::ProvisioningPlanChanged
            } else {
                Failure::ProvisioningDisabled
            })
        );
        assert!(fixture.requests.try_recv().is_err());
    }
}

#[tokio::test]
async fn provisioning_clone_executor_tracks_one_task_and_only_completes_clone_stage() {
    use crate::{
        operation::{Failure, Status},
        provisioning_executor,
    };
    let upid = "UPID:pve-test:00000001:00000002:00000003:qmclone:9000:fixture@pve:";
    for (reply, exit, marker, failure) in [
        (serde_json::json!(upid), "OK", true, None),
        (
            serde_json::json!(upid),
            "private error",
            true,
            Some(Failure::ProviderTaskFailed),
        ),
        (
            serde_json::json!(upid),
            "OK",
            false,
            Some(Failure::PowerStateUnconfirmed),
        ),
        (
            serde_json::json!(null),
            "OK",
            true,
            Some(Failure::ProviderSubmissionUnconfirmed),
        ),
    ] {
        let mut fixture = provisioning_executor_fixture(reply, exit, marker, "valid").await;
        let provider = fixture.client();
        let (_directory, store, operation) = queued_clone(&provider);
        let catalog =
            crate::provisioning::Catalog::parse(include_str!("../../../examples/templates.yaml"))
                .unwrap();
        assert!(
            provisioning_executor::run_clone(&store, Some(&provider), Some(&catalog), true)
                .await
                .unwrap()
        );
        let job = store.provisioning_job(&operation.id).unwrap().unwrap();
        assert_eq!(job.operation.failure, failure);
        assert_eq!(
            job.operation.status,
            match failure {
                None => Status::Running,
                Some(Failure::ProviderTaskFailed) => Status::Failed,
                _ => Status::Uncertain,
            }
        );
        assert_eq!(
            job.stages[0].status,
            match failure {
                None => Status::Succeeded,
                Some(Failure::ProviderTaskFailed) => Status::Failed,
                _ => Status::Uncertain,
            }
        );
        assert!(job.stages[1..].iter().all(|s| s.status == Status::Queued));
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert_eq!(
            requests.iter().filter(|r| r.starts_with("POST ")).count(),
            1
        );
        assert!(requests
            .iter()
            .any(|r| r.starts_with("GET /api2/json/nodes/pve-test/qemu/9000/pending ")));
        assert!(!requests.iter().any(|r| r.contains("/status/start")));
        if failure == Some(Failure::ProviderSubmissionUnconfirmed) {
            assert!(store.claim_provisioning_clone().unwrap().is_none());
        }
    }
}

#[tokio::test]
async fn provisioning_clone_executor_reconciles_retained_task_by_get_with_creation_disabled() {
    let upid = "UPID:pve-test:00000001:00000002:00000003:qmclone:9000:fixture@pve:";
    let mut fixture =
        provisioning_executor_fixture(serde_json::json!(upid), "OK", true, "valid").await;
    let provider = fixture.client();
    let (directory, store, operation) = queued_clone(&provider);
    let job = store.claim_provisioning_clone().unwrap().unwrap();
    store
        .begin_provisioning_clone(&operation.id, &job.approval)
        .unwrap();
    store
        .link_provisioning_clone(
            &operation.id,
            &super::TaskReference::new("pve-test", upid).unwrap(),
        )
        .unwrap();
    drop(store);
    let store = crate::store::Store::open(directory.path().join("clone.sqlite")).unwrap();
    assert_eq!(store.recover_interrupted_operations().unwrap(), 1);
    assert!(
        crate::provisioning_executor::run_clone(&store, Some(&provider), None, false)
            .await
            .unwrap()
    );
    let job = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert_eq!(job.stages[0].status, crate::operation::Status::Succeeded);
    assert_eq!(job.operation.status, crate::operation::Status::Running);
    let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
    assert_eq!(requests.len(), 3);
    assert!(requests.iter().all(|r| r.starts_with("GET ")));
    assert!(requests[0].contains("/tasks/"));
}

#[tokio::test]
async fn provisioning_clone_executor_rejects_unproven_source_or_occupied_target_without_dispatch() {
    for gate in [
        "revision_missing",
        "locked",
        "pending",
        "occupied",
        "running",
    ] {
        let mut fixture =
            provisioning_executor_fixture(serde_json::json!(null), "OK", true, gate).await;
        let provider = fixture.client();
        let (_directory, store, operation) = queued_clone(&provider);
        let catalog =
            crate::provisioning::Catalog::parse(include_str!("../../../examples/templates.yaml"))
                .unwrap();
        assert!(crate::provisioning_executor::run_clone(
            &store,
            Some(&provider),
            Some(&catalog),
            true
        )
        .await
        .unwrap());
        let job = store.provisioning_job(&operation.id).unwrap().unwrap();
        assert_eq!(
            job.operation.failure,
            Some(crate::operation::Failure::ProvisioningPreconditionsFailed)
        );
        assert!(!job.stages[0].dispatch_started);
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert!(requests.iter().all(|r| r.starts_with("GET ")), "{gate}");
    }
}

fn resource_config(plan: &crate::provisioning::Plan) -> serde_json::Value {
    serde_json::json!({"digest":"d".repeat(40),"description":clone_description(plan),
        "name":"kernel-lab","tags":format!("vmanyware-{}","a".repeat(32)),"template":0,"sockets":1,"cores":4,"vcpus":4,"cpulimit":0,"memory":8192,"balloon":0})
}

#[tokio::test]
async fn resource_transport_checks_stopped_clone_and_puts_only_fixed_digest_bound_fields() {
    let plan = full_clone_plan();
    let request = plan
        .resource_request(&"a".repeat(32), &"d".repeat(40))
        .unwrap();
    assert!(plan
        .resource_request(&"a".repeat(32), "bad-revision")
        .is_err());
    let config = serde_json::json!({"data":resource_config(&plan)}).to_string();
    let mut fixture = Fixture::responses(
        "200 OK",
        &[
            ("*", &config),
            ("*", r#"{"data":{"vmid":202,"status":"stopped"}}"#),
            ("*", r#"{"data":[]}"#),
            ("*", r#"{"data":null}"#),
        ],
        "",
        Duration::ZERO,
        true,
    )
    .await;
    fixture.client().dispatch_resources(&request).await.unwrap();
    let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
    assert_eq!(requests.len(), 4);
    assert!(requests[..3].iter().all(|r| r.starts_with("GET ")));
    assert!(requests[3].starts_with("PUT /api2/json/nodes/pve-test/qemu/202/config "));
    assert_eq!(
        requests[3].split_once("\r\n\r\n").unwrap().1,
        format!(
            "digest={}&sockets=1&cores=4&vcpus=4&cpulimit=0&memory=8192&balloon=0&tags=vmanyware-{}",
            "d".repeat(40), "a".repeat(32)
        )
    );
}

#[tokio::test]
async fn resource_transport_blocks_changed_foreign_locked_running_pending_targets() {
    let plan = full_clone_plan();
    let request = plan
        .resource_request(&"a".repeat(32), &"d".repeat(40))
        .unwrap();
    for variant in ["digest", "marker", "lock", "template", "running", "pending"] {
        let mut config = resource_config(&plan);
        match variant {
            "digest" => config["digest"] = serde_json::json!("e".repeat(40)),
            "marker" => config["description"] = serde_json::json!("foreign"),
            "lock" => config["lock"] = serde_json::json!("backup"),
            "template" => config["template"] = serde_json::json!(1),
            _ => {}
        }
        let mut fixture = Fixture::responses("200 OK", &[
            ("*", &serde_json::json!({"data":config}).to_string()),
            ("*", &serde_json::json!({"data":{"vmid":202,"status":if variant == "running" {"running"} else {"stopped"}}}).to_string()),
            ("*", &serde_json::json!({"data":if variant == "pending" {serde_json::json!([{"key":"memory","pending":"1024"}])} else {serde_json::json!([])}}).to_string()),
        ], "", Duration::ZERO, true).await;
        assert_eq!(
            fixture
                .client()
                .dispatch_resources(&request)
                .await
                .unwrap_err(),
            Error::InvalidResponse,
            "{variant}"
        );
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert!(!requests.is_empty());
        assert!(requests.iter().all(|r| r.starts_with("GET ")), "{variant}");
    }
}

#[tokio::test]
async fn resource_observation_requires_exact_applied_settings_and_consistent_revision_by_get() {
    let plan = full_clone_plan();
    let request = plan
        .resource_request(&"a".repeat(32), &"c".repeat(40))
        .unwrap();
    for variant in [
        "valid",
        "sockets",
        "cores",
        "vcpus",
        "memory",
        "balloon",
        "cpulimit",
        "revision",
        "pending",
        "missing_tag",
        "foreign_tag",
    ] {
        let mut config = resource_config(&plan);
        if variant == "missing_tag" {
            config.as_object_mut().unwrap().remove("tags");
        }
        if variant == "foreign_tag" {
            config["tags"] = serde_json::json!(format!("vmanyware-{}", "b".repeat(32)));
        }
        if ["sockets", "cores", "vcpus", "memory", "balloon", "cpulimit"].contains(&variant) {
            config[variant] = serde_json::json!(999);
        }
        let mut current = config.clone();
        if variant == "revision" {
            current["digest"] = serde_json::json!("e".repeat(40));
        }
        let mut fixture = Fixture::responses("200 OK", &[
            ("*", &serde_json::json!({"data":config}).to_string()),
            ("*", &serde_json::json!({"data":current}).to_string()),
            ("*", r#"{"data":{"vmid":202,"status":"stopped"}}"#),
            ("*", &serde_json::json!({"data":if variant == "pending" {serde_json::json!([{"key":"memory","pending":"1024"}])} else {serde_json::json!([])}}).to_string()),
        ], "", Duration::ZERO, true).await;
        let observed = fixture.client().observe_resources(&request).await;
        if variant == "valid" {
            let observed = observed.unwrap();
            assert_eq!(observed.cpu, 4);
            assert_eq!(observed.memory_mib, 8192);
            assert_eq!(observed.target.config_digest, "d".repeat(40));
        } else {
            assert_eq!(observed.unwrap_err(), Error::InvalidResponse, "{variant}");
        }
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert!(requests.iter().all(|r| r.starts_with("GET ")));
    }
}

#[tokio::test]
async fn resource_transport_never_replays_put_after_invalid_submission_reply() {
    let plan = full_clone_plan();
    let request = plan
        .resource_request(&"a".repeat(32), &"d".repeat(40))
        .unwrap();
    let config = serde_json::json!({"data":resource_config(&plan)}).to_string();
    for reply in [
        r#"{"data":"unexpected-task"}"#,
        r#"{"data":{}}"#,
        "truncated",
    ] {
        let mut fixture = Fixture::responses(
            "200 OK",
            &[
                ("*", &config),
                ("*", r#"{"data":{"vmid":202,"status":"stopped"}}"#),
                ("*", r#"{"data":[]}"#),
                ("*", reply),
            ],
            "",
            Duration::ZERO,
            true,
        )
        .await;
        assert_eq!(
            fixture
                .client()
                .dispatch_resources(&request)
                .await
                .unwrap_err(),
            Error::InvalidResponse
        );
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert_eq!(requests.len(), 4);
        assert_eq!(requests.iter().filter(|r| r.starts_with("PUT ")).count(), 1);
    }
}

fn completed_clone_for_resources(
    provider: &Proxmox,
) -> (
    tempfile::TempDir,
    crate::store::Store,
    crate::operation::Operation,
) {
    completed_clone_for_resources_with_yaml(
        provider,
        include_str!("../../../examples/kernel-lab.yaml"),
    )
}
fn completed_clone_for_resources_with_yaml(
    provider: &Proxmox,
    yaml: &str,
) -> (
    tempfile::TempDir,
    crate::store::Store,
    crate::operation::Operation,
) {
    use crate::provisioning_intent::{CloneOutcome, CloneResult};
    let (directory, store, operation) = queued_clone_with_yaml(provider, yaml);
    let job = store.claim_provisioning_clone().unwrap().unwrap();
    store
        .begin_provisioning_clone(&operation.id, &job.approval)
        .unwrap();
    let task = super::TaskReference::new(
        "pve-test",
        "UPID:pve-test:00000001:00000002:00000003:qmclone:9000:fixture@pve:",
    )
    .unwrap();
    store.link_provisioning_clone(&operation.id, &task).unwrap();
    store
        .finish_provisioning_clone(
            &operation.id,
            CloneOutcome::Completed(CloneResult {
                instance_id: operation.instance_id.clone().unwrap(),
                provider_scope_id: provider.scope_id(),
                plan_hash: job.approval.plan_hash,
                task_status: super::TaskStatus::Succeeded,
                target: super::CloneObservation {
                    config_digest: "d".repeat(40),
                    observation: super::VmObservation {
                        node: "pve-test".into(),
                        vmid: 202,
                        status: super::PowerState::Stopped,
                        qmp_status: None,
                    },
                },
            }),
        )
        .unwrap();
    (directory, store, operation)
}

async fn resource_executor_fixture(reply: &str, applied: bool, reconcile_only: bool) -> Fixture {
    let plan = full_clone_plan();
    let old = serde_json::json!({"data":resource_config(&plan)}).to_string();
    let mut config = resource_config(&plan);
    config["digest"] = serde_json::json!("e".repeat(40));
    if !applied {
        config["memory"] = serde_json::json!(1024);
    }
    let current = serde_json::json!({"data":config}).to_string();
    let power = r#"{"data":{"vmid":202,"status":"stopped"}}"#;
    let pending = r#"{"data":[]}"#;
    let mut responses = Vec::new();
    if !reconcile_only {
        responses.extend_from_slice(&[
            ("*", old.as_str()),
            ("*", power),
            ("*", pending),
            ("*", old.as_str()),
            ("*", power),
            ("*", pending),
            ("*", reply),
        ]);
    }
    responses.extend_from_slice(&[
        ("*", current.as_str()),
        ("*", current.as_str()),
        ("*", power),
        ("*", pending),
    ]);
    Fixture::responses("200 OK", &responses, "", Duration::ZERO, true).await
}

#[tokio::test]
async fn resource_executor_commits_one_put_then_observes_even_if_reply_is_lost_or_invalid() {
    use crate::{
        operation::{Failure, Status},
        provisioning_executor::run_resources,
    };
    let catalog =
        crate::provisioning::Catalog::parse(include_str!("../../../examples/templates.yaml"))
            .unwrap();
    for (reply, applied) in [
        (r#"{"data":null}"#, true),
        (DROP_REPLY, true),
        ("malformed-json", true),
        (r#"{"data":null}"#, false),
        (DROP_REPLY, false),
    ] {
        let mut fixture = resource_executor_fixture(reply, applied, false).await;
        let provider = fixture.client();
        let (_directory, store, operation) = completed_clone_for_resources(&provider);
        assert!(run_resources(&store, Some(&provider), Some(&catalog), true)
            .await
            .unwrap());
        let job = store.provisioning_job(&operation.id).unwrap().unwrap();
        assert_eq!(job.stages[0].status, Status::Succeeded);
        assert_eq!(
            job.stages[1].status,
            if applied {
                Status::Succeeded
            } else {
                Status::Uncertain
            }
        );
        assert_eq!(
            job.operation.status,
            if applied {
                Status::Running
            } else {
                Status::Uncertain
            }
        );
        if !applied {
            assert_eq!(
                job.operation.failure.unwrap(),
                if reply == DROP_REPLY {
                    Failure::ProviderSubmissionUnconfirmed
                } else {
                    Failure::ResourceStateUnconfirmed
                }
            );
        }
        assert!(job.operation.result.is_none());
        assert!(job.stages[1].dispatch_started);
        assert!(job.stages[1].task.is_none());
        assert!(job.stages[2..].iter().all(|s| s.status == Status::Queued));
        assert!(
            !run_resources(&store, Some(&provider), Some(&catalog), true)
                .await
                .unwrap()
        );
        assert!(store.claim_provisioning_clone().unwrap().is_none());
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert_eq!(requests.iter().filter(|r| r.starts_with("PUT ")).count(), 1);
        assert!(requests.iter().all(|r| r.starts_with("GET ")
            || r.starts_with("PUT /api2/json/nodes/pve-test/qemu/202/config ")));
    }
}

#[tokio::test]
async fn resource_executor_restart_reconciles_without_task_or_new_creation_capability() {
    use crate::{operation::Status, provisioning_executor::run_resources};
    let mut fixture = resource_executor_fixture("unused", true, true).await;
    let provider = fixture.client();
    let (directory, store, operation) = completed_clone_for_resources(&provider);
    let job = store.claim_provisioning_resources().unwrap().unwrap();
    store
        .begin_provisioning_resources(&operation.id, &job.approval, &"d".repeat(40))
        .unwrap();
    drop(store);
    let reopened = crate::store::Store::open(&directory.path().join("clone.sqlite")).unwrap();
    assert_eq!(reopened.recover_interrupted_operations().unwrap(), 1);
    assert!(run_resources(&reopened, Some(&provider), None, false)
        .await
        .unwrap());
    let job = reopened.provisioning_job(&operation.id).unwrap().unwrap();
    assert_eq!(job.stages[1].status, Status::Succeeded);
    assert_eq!(job.operation.status, Status::Running);
    let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
    assert_eq!(requests.len(), 4);
    assert!(requests.iter().all(|r| r.starts_with("GET ")));
    assert!(reopened.claim_provisioning_clone().unwrap().is_none());
}

#[tokio::test]
async fn resource_executor_rejects_disabled_changed_or_foreign_target_before_dispatch_marker() {
    use crate::{
        operation::{Failure, Status},
        provisioning_executor::run_resources,
    };
    let catalog =
        crate::provisioning::Catalog::parse(include_str!("../../../examples/templates.yaml"))
            .unwrap();
    for variant in ["disabled", "catalog", "scope", "marker", "revision"] {
        let mut config = resource_config(&full_clone_plan());
        if variant == "marker" {
            config["description"] = serde_json::json!("foreign");
        }
        if variant == "revision" {
            config["digest"] = serde_json::json!("e".repeat(40));
        }
        let mut fixture = Fixture::responses(
            "200 OK",
            &[
                ("*", &serde_json::json!({"data":config}).to_string()),
                ("*", r#"{"data":{"vmid":202,"status":"stopped"}}"#),
                ("*", r#"{"data":[]}"#),
            ],
            "",
            Duration::ZERO,
            true,
        )
        .await;
        let provider = fixture.client();
        let (_directory, store, operation) = completed_clone_for_resources(&provider);
        let foreign = Fixture::new("200 OK", r#"{"data":{}}"#, "", Duration::ZERO).await;
        let foreign_provider = foreign.client();
        assert!(run_resources(
            &store,
            Some(if variant == "scope" {
                &foreign_provider
            } else {
                &provider
            }),
            if variant == "catalog" {
                None
            } else {
                Some(&catalog)
            },
            variant != "disabled"
        )
        .await
        .unwrap());
        let job = store.provisioning_job(&operation.id).unwrap().unwrap();
        assert_eq!(job.stages[1].status, Status::Failed);
        assert!(!job.stages[1].dispatch_started);
        assert_eq!(
            job.operation.failure.unwrap(),
            match variant {
                "disabled" => Failure::ProvisioningDisabled,
                "catalog" => Failure::ProvisioningPlanChanged,
                "scope" => Failure::ProviderScopeChanged,
                _ => Failure::ProvisioningPreconditionsFailed,
            }
        );
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert!(requests.iter().all(|r| r.starts_with("GET ")));
        if ["disabled", "catalog", "scope"].contains(&variant) {
            assert!(requests.is_empty());
        }
    }
}

async fn disk_fixture(config: serde_json::Value, reply: &str, revision_changed: bool) -> Fixture {
    let mut current = config.clone();
    if revision_changed {
        current["digest"] = serde_json::json!("e".repeat(40));
    }
    Fixture::responses(
        "200 OK",
        &[
            ("*", &serde_json::json!({"data":config}).to_string()),
            (
                "*",
                &serde_json::json!({"data":current.clone()}).to_string(),
            ),
            ("*", &serde_json::json!({"data":current}).to_string()),
            ("*", r#"{"data":{"vmid":202,"status":"stopped"}}"#),
            ("*", r#"{"data":[]}"#),
            ("*", reply),
        ],
        "",
        Duration::ZERO,
        true,
    )
    .await
}
fn disk_config(drive: &str) -> serde_json::Value {
    let mut config = resource_config(&full_clone_plan());
    config["scsi0"] = serde_json::json!(drive);
    config
}

#[tokio::test]
async fn disk_expansion_uses_absolute_size_fixed_volume_revision_and_target_task_identity() {
    let plan = full_clone_plan();
    let request = plan
        .disk_request(&"a".repeat(32), &"d".repeat(40), "local-lvm:vm-202-disk-0")
        .unwrap();
    for (node, kind, vmid, valid) in [
        ("pve-test", "resize", "202", true),
        ("pve-test", "resize", "9000", false),
        ("pve-test", "resize", "0202", false),
        ("pve-test", "qmclone", "202", false),
        ("foreign", "resize", "202", false),
    ] {
        let reply=serde_json::json!({"data":format!("UPID:{node}:00000001:00000002:00000003:{kind}:{vmid}:fixture@pve:")}).to_string();
        let mut fixture = disk_fixture(
            disk_config("local-lvm:vm-202-disk-0,size=8G"),
            &reply,
            false,
        )
        .await;
        let result = fixture.client().dispatch_disk_expansion(&request).await;
        if valid {
            assert!(result.is_ok());
        } else {
            assert_eq!(result.unwrap_err(), Error::InvalidResponse);
        }
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert_eq!(requests.len(), 6);
        assert!(requests[..5].iter().all(|r| r.starts_with("GET ")));
        assert!(requests[5].starts_with("PUT /api2/json/nodes/pve-test/qemu/202/resize "));
        assert_eq!(
            requests[5].split_once("\r\n\r\n").unwrap().1,
            format!("disk=scsi0&size=40G&digest={}", "d".repeat(40))
        );
    }
    for volume in [
        "other:vm-202-disk-0",
        "local-lvm:",
        "local-lvm:../foreign",
        "local-lvm:vm-202,discard=on",
        "local-lvm:/absolute",
        "local-lvm:vm-202\nsecret",
    ] {
        assert!(plan
            .disk_request(&"a".repeat(32), &"d".repeat(40), volume)
            .is_err());
    }
    assert!(plan
        .disk_request(&"a".repeat(32), "bad", "local-lvm:vm-202-disk-0")
        .is_err());
    let definition = crate::definition::Definition::parse(
        &include_str!("../../../examples/kernel-lab.yaml").replace("diskGiB: 40", "diskGiB: 8"),
    )
    .unwrap();
    let catalog =
        crate::provisioning::Catalog::parse(include_str!("../../../examples/templates.yaml"))
            .unwrap();
    assert!(catalog
        .plan(&definition, 202)
        .unwrap()
        .disk_request(&"a".repeat(32), &"d".repeat(40), "local-lvm:vm-202-disk-0")
        .is_err());
}

#[tokio::test]
async fn disk_expansion_blocks_ambiguous_or_changed_source_disk_without_put() {
    let request = full_clone_plan()
        .disk_request(&"a".repeat(32), &"d".repeat(40), "local-lvm:vm-202-disk-0")
        .unwrap();
    for drive in [
        "local-lvm:vm-203-disk-0,size=8G",
        "other:vm-202-disk-0,size=8G",
        "local-lvm:vm-202-disk-0,size=40G",
        "local-lvm:vm-202-disk-0,size=4G",
        "local-lvm:vm-202-disk-0,size=8G,size=8G",
        "local-lvm:vm-202-disk-0,size=8.0G",
        "local-lvm:vm-202-disk-0,size=8G,media=cdrom",
        "local-lvm:vm-202-disk-0",
        "local-lvm:vm-202-disk-0,size=18446744073709551615T",
    ] {
        let mut fixture = disk_fixture(disk_config(drive), r#"{"data":null}"#, false).await;
        assert_eq!(
            fixture
                .client()
                .dispatch_disk_expansion(&request)
                .await
                .unwrap_err(),
            Error::InvalidResponse
        );
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert!(requests.iter().all(|r| r.starts_with("GET ")));
    }
    for variant in ["revision", "resources", "marker"] {
        let mut config = disk_config("local-lvm:vm-202-disk-0,size=8G");
        if variant == "resources" {
            config["memory"] = serde_json::json!(1024);
        }
        if variant == "marker" {
            config["description"] = serde_json::json!("foreign");
        }
        let mut fixture = disk_fixture(config, r#"{"data":null}"#, variant == "revision").await;
        assert_eq!(
            fixture
                .client()
                .dispatch_disk_expansion(&request)
                .await
                .unwrap_err(),
            Error::InvalidResponse
        );
        assert!(
            std::iter::from_fn(|| fixture.requests.try_recv().ok()).all(|r| r.starts_with("GET "))
        );
    }
}

#[tokio::test]
async fn disk_expansion_observes_exact_configured_size_and_never_replays_invalid_or_lost_reply() {
    let request = full_clone_plan()
        .disk_request(&"a".repeat(32), &"c".repeat(40), "local-lvm:vm-202-disk-0")
        .unwrap();
    for (drive, valid) in [
        ("local-lvm:vm-202-disk-0,size=40G,discard=on", true),
        ("local-lvm:vm-202-disk-0,size=40960M", true),
        ("local-lvm:vm-202-disk-0,size=42949672960", true),
        ("local-lvm:vm-202-disk-0,size=8G", false),
        ("local-lvm:vm-202-disk-0,size=41G", false),
        ("local-lvm:vm-203-disk-0,size=40G", false),
    ] {
        let mut fixture = disk_fixture(disk_config(drive), "unused", false).await;
        let result = fixture.client().observe_disk_expansion(&request).await;
        if valid {
            let result = result.unwrap();
            assert_eq!(result.device, "scsi0");
            assert_eq!(result.volume, "local-lvm:vm-202-disk-0");
            assert_eq!(result.size_bytes, 40 * 1024_u64.pow(3));
        } else {
            assert_eq!(result.unwrap_err(), Error::InvalidResponse);
        }
        assert!(
            std::iter::from_fn(|| fixture.requests.try_recv().ok()).all(|r| r.starts_with("GET "))
        );
    }
    let request = full_clone_plan()
        .disk_request(&"a".repeat(32), &"d".repeat(40), "local-lvm:vm-202-disk-0")
        .unwrap();
    for reply in [r#"{"data":null}"#, "truncated", DROP_REPLY] {
        let mut fixture =
            disk_fixture(disk_config("local-lvm:vm-202-disk-0,size=8G"), reply, false).await;
        assert!(fixture
            .client()
            .dispatch_disk_expansion(&request)
            .await
            .is_err());
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert_eq!(requests.len(), 6);
        assert_eq!(requests.iter().filter(|r| r.starts_with("PUT ")).count(), 1);
    }
}

#[tokio::test]
async fn disk_expansion_uses_only_the_catalog_mapped_device_including_file_backed_volumes() {
    let definition =
        crate::definition::Definition::parse(include_str!("../../../examples/kernel-lab.yaml"))
            .unwrap();
    for device in ["virtio0", "sata0"] {
        let catalog = crate::provisioning::Catalog::parse(
            &include_str!("../../../examples/templates.yaml")
                .replace("diskDevice: scsi0", &format!("diskDevice: {device}")),
        )
        .unwrap();
        let plan = catalog.plan(&definition, 202).unwrap();
        let volume = "local-lvm:202/vm-202-disk-0.qcow2";
        let request = plan
            .disk_request(&"a".repeat(32), &"d".repeat(40), volume)
            .unwrap();
        let mut config = resource_config(&plan);
        config[device] = serde_json::json!(format!("{volume},size=8192M"));
        // An unrelated disk must never be selected by capacity or enumeration order.
        config["scsi0"] = serde_json::json!("local-lvm:vm-203-disk-0,size=8G");
        let mut fixture = disk_fixture(
            config,
            r#"{"data":"UPID:pve-test:00000001:00000002:00000003:resize:202:fixture@pve:"}"#,
            false,
        )
        .await;
        fixture
            .client()
            .dispatch_disk_expansion(&request)
            .await
            .unwrap();
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert_eq!(requests.len(), 6);
        assert_eq!(
            requests[5].split_once("\r\n\r\n").unwrap().1,
            format!("disk={device}&size=40G&digest={}", "d".repeat(40))
        );
    }
}

fn completed_resources_for_disk(
    provider: &Proxmox,
) -> (
    tempfile::TempDir,
    crate::store::Store,
    crate::operation::Operation,
) {
    completed_resources_for_disk_with_yaml(
        provider,
        include_str!("../../../examples/kernel-lab.yaml"),
    )
}
fn completed_resources_for_disk_with_yaml(
    provider: &Proxmox,
    yaml: &str,
) -> (
    tempfile::TempDir,
    crate::store::Store,
    crate::operation::Operation,
) {
    let (directory, store, operation) = completed_clone_for_resources_with_yaml(provider, yaml);
    let job = store.claim_provisioning_resources().unwrap().unwrap();
    store
        .begin_provisioning_resources(&operation.id, &job.approval, &"d".repeat(40))
        .unwrap();
    store
        .finish_provisioning_resources(
            &operation.id,
            crate::provisioning_intent::ResourceOutcome::Completed(
                crate::provisioning_intent::ResourceResult {
                    instance_id: operation.instance_id.clone().unwrap(),
                    provider_scope_id: provider.scope_id(),
                    plan_hash: job.approval.plan_hash,
                    resources: super::ResourceObservation {
                        cpu: 4,
                        memory_mib: 8192,
                        target: super::CloneObservation {
                            config_digest: "e".repeat(40),
                            observation: super::VmObservation {
                                node: "pve-test".into(),
                                vmid: 202,
                                status: super::PowerState::Stopped,
                                qmp_status: None,
                            },
                        },
                    },
                },
            ),
        )
        .unwrap();
    (directory, store, operation)
}
const RESIZE_TASK: &str = "UPID:pve-test:00000001:00000002:00000003:resize:202:fixture@pve:";
async fn disk_executor_fixture(variant: &str, reconcile_only: bool) -> Fixture {
    let mut old = disk_config("local-lvm:vm-202-disk-0,size=8G");
    old["digest"] = serde_json::json!("e".repeat(40));
    let old = serde_json::json!({"data":old}).to_string();
    let mut current = disk_config(if variant == "size" {
        "local-lvm:vm-202-disk-0,size=8G"
    } else {
        "local-lvm:vm-202-disk-0,size=40G"
    });
    current["digest"] = serde_json::json!("f".repeat(40));
    if variant == "marker" {
        current["description"] = serde_json::json!("foreign");
    }
    let current = serde_json::json!({"data":current}).to_string();
    let reply = if variant == "unknown" {
        DROP_REPLY.to_owned()
    } else if variant == "foreign_task" {
        serde_json::json!({"data":RESIZE_TASK.replace(":resize:202:",":resize:9000:")}).to_string()
    } else {
        serde_json::json!({"data":RESIZE_TASK}).to_string()
    };
    let task = if variant == "task_unknown" {
        r#"{"data":{}}"#.to_owned()
    } else {
        serde_json::json!({"data":{"node":"pve-test","upid":RESIZE_TASK,"status":"stopped","exitstatus":if variant=="failed" {"fixture-failure"}else{"OK"}}}).to_string()
    };
    let power = r#"{"data":{"vmid":202,"status":"stopped"}}"#;
    let pending = r#"{"data":[]}"#;
    let mut responses = Vec::new();
    if !reconcile_only {
        responses.extend_from_slice(&[
            ("*", old.as_str()), // Volume selection, then full pre-dispatch guard.
            ("*", old.as_str()),
            ("*", old.as_str()),
            ("*", old.as_str()),
            ("*", power),
            ("*", pending),
            // Repeat the guard after committing the durable dispatch marker.
            ("*", old.as_str()),
            ("*", old.as_str()),
            ("*", old.as_str()),
            ("*", power),
            ("*", pending),
            ("*", reply.as_str()),
        ]);
    }
    responses.extend_from_slice(&[
        ("*", task.as_str()),
        ("*", current.as_str()),
        ("*", current.as_str()),
        ("*", current.as_str()),
        ("*", power),
        ("*", pending),
    ]);
    Fixture::responses("200 OK", &responses, "", Duration::ZERO, true).await
}

#[tokio::test]
async fn disk_executor_retains_one_resize_task_and_only_completes_its_stage() {
    use crate::{
        operation::{Failure, Status},
        provisioning_executor::run_disk,
    };
    let catalog =
        crate::provisioning::Catalog::parse(include_str!("../../../examples/templates.yaml"))
            .unwrap();
    for variant in [
        "valid",
        "failed",
        "size",
        "marker",
        "unknown",
        "foreign_task",
        "task_unknown",
    ] {
        let mut fixture = disk_executor_fixture(variant, false).await;
        let provider = fixture.client();
        let (_directory, store, operation) = completed_resources_for_disk(&provider);
        assert!(run_disk(&store, Some(&provider), Some(&catalog), true)
            .await
            .unwrap());
        let job = store.provisioning_job(&operation.id).unwrap().unwrap();
        assert!(job.stages[..2]
            .iter()
            .all(|s| s.status == Status::Succeeded));
        let expected = match variant {
            "valid" => Status::Succeeded,
            "failed" => Status::Failed,
            _ => Status::Uncertain,
        };
        assert_eq!(job.stages[2].status, expected, "{variant}");
        assert_eq!(
            job.operation.status,
            if variant == "valid" {
                Status::Running
            } else {
                expected
            }
        );
        assert!(job.operation.result.is_none());
        assert!(job.stages[3..].iter().all(|s| s.status == Status::Queued));
        assert!(job.disk_dispatch.is_some());
        assert_eq!(
            job.stages[2].task.is_some(),
            !["unknown", "foreign_task"].contains(&variant)
        );
        assert_eq!(
            job.operation.failure,
            match variant {
                "valid" => None,
                "failed" => Some(Failure::ProviderTaskFailed),
                "size" | "marker" => Some(Failure::DiskStateUnconfirmed),
                "task_unknown" => Some(Failure::ProviderTaskUnconfirmed),
                _ => Some(Failure::ProviderSubmissionUnconfirmed),
            }
        );
        assert!(!run_disk(&store, Some(&provider), Some(&catalog), true)
            .await
            .unwrap());
        assert!(store.claim_provisioning_clone().unwrap().is_none());
        assert!(store.claim_provisioning_resources().unwrap().is_none());
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert_eq!(
            requests.iter().filter(|r| r.starts_with("PUT ")).count(),
            1,
            "{variant}"
        );
        assert!(requests.iter().all(|r| r.starts_with("GET ")
            || r.starts_with("PUT /api2/json/nodes/pve-test/qemu/202/resize ")));
        if variant == "valid" {
            assert_eq!(requests.len(), 18);
        }
    }
}

#[tokio::test]
async fn disk_executor_restart_reconciles_only_known_task_by_get_with_creation_disabled() {
    use crate::{operation::Status, provisioning_executor::run_disk};
    for known in [true, false] {
        let mut fixture = disk_executor_fixture("valid", true).await;
        let provider = fixture.client();
        let (directory, store, operation) = completed_resources_for_disk(&provider);
        let job = store.claim_provisioning_disk().unwrap().unwrap();
        store
            .begin_provisioning_disk(
                &operation.id,
                &job.approval,
                &"e".repeat(40),
                "local-lvm:vm-202-disk-0",
            )
            .unwrap();
        if known {
            store
                .link_provisioning_disk(
                    &operation.id,
                    &super::TaskReference::new("pve-test", RESIZE_TASK).unwrap(),
                )
                .unwrap();
        }
        drop(store);
        let reopened = crate::store::Store::open(&directory.path().join("clone.sqlite")).unwrap();
        assert_eq!(reopened.recover_interrupted_operations().unwrap(), 1);
        assert_eq!(
            run_disk(&reopened, Some(&provider), None, false)
                .await
                .unwrap(),
            known
        );
        let retained = reopened.provisioning_job(&operation.id).unwrap().unwrap();
        assert_eq!(
            retained.stages[2].status,
            if known {
                Status::Succeeded
            } else {
                Status::Uncertain
            }
        );
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert_eq!(requests.len(), if known { 6 } else { 0 });
        assert!(requests.iter().all(|r| r.starts_with("GET ")));
        assert!(reopened.claim_provisioning_clone().unwrap().is_none());
    }
}

#[tokio::test]
async fn disk_executor_rejects_disabled_changed_or_unproven_inputs_before_marking_dispatch() {
    use crate::{
        operation::{Failure, Status},
        provisioning_executor::run_disk,
    };
    let catalog =
        crate::provisioning::Catalog::parse(include_str!("../../../examples/templates.yaml"))
            .unwrap();
    for variant in [
        "disabled", "catalog", "scope", "volume", "revision", "size", "memory", "pending",
        "running",
    ] {
        let mut config = disk_config(if variant == "volume" {
            "other:vm-202-disk-0,size=8G"
        } else if variant == "size" {
            "local-lvm:vm-202-disk-0,size=40G"
        } else {
            "local-lvm:vm-202-disk-0,size=8G"
        });
        config["digest"] = serde_json::json!(if variant == "revision" {
            "f".repeat(40)
        } else {
            "e".repeat(40)
        });
        if variant == "memory" {
            config["memory"] = serde_json::json!(1024);
        }
        let mut fixture=Fixture::routes("200 OK",&[
            ("/api2/json/nodes/pve-test/qemu/202/config?current=1",&serde_json::json!({"data":config}).to_string()),
            ("/api2/json/nodes/pve-test/qemu/202/status/current",&serde_json::json!({"data":{"vmid":202,"status":if variant=="running" {"running"}else{"stopped"}}}).to_string()),
            ("/api2/json/nodes/pve-test/qemu/202/pending",&serde_json::json!({"data":if variant=="pending" {serde_json::json!([{"key":"memory","pending":"1024"}])}else{serde_json::json!([])}}).to_string()),
        ],"",Duration::ZERO).await;
        let provider = fixture.client();
        let (_directory, store, operation) = completed_resources_for_disk(&provider);
        let foreign = Fixture::new("200 OK", r#"{"data":{}}"#, "", Duration::ZERO).await;
        let foreign_provider = foreign.client();
        assert!(run_disk(
            &store,
            Some(if variant == "scope" {
                &foreign_provider
            } else {
                &provider
            }),
            if variant == "catalog" {
                None
            } else {
                Some(&catalog)
            },
            variant != "disabled"
        )
        .await
        .unwrap());
        let job = store.provisioning_job(&operation.id).unwrap().unwrap();
        assert_eq!(job.stages[2].status, Status::Failed);
        assert!(!job.stages[2].dispatch_started);
        assert!(job.disk_dispatch.is_none());
        assert_eq!(
            job.operation.failure,
            Some(match variant {
                "disabled" => Failure::ProvisioningDisabled,
                "catalog" => Failure::ProvisioningPlanChanged,
                "scope" => Failure::ProviderScopeChanged,
                _ => Failure::ProvisioningPreconditionsFailed,
            })
        );
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert!(requests.iter().all(|r| r.starts_with("GET ")));
        if ["disabled", "catalog", "scope"].contains(&variant) {
            assert!(requests.is_empty());
        }
    }
}

#[tokio::test]
async fn disk_executor_configuration_change_after_marker_never_sends_or_retries_resize() {
    use crate::{
        operation::{Failure, Status},
        provisioning_executor::run_disk,
    };
    let mut config = disk_config("local-lvm:vm-202-disk-0,size=8G");
    config["digest"] = serde_json::json!("e".repeat(40));
    let old = serde_json::json!({"data":config}).to_string();
    config["digest"] = serde_json::json!("f".repeat(40));
    let changed = serde_json::json!({"data":config}).to_string();
    let power = r#"{"data":{"vmid":202,"status":"stopped"}}"#;
    let pending = r#"{"data":[]}"#;
    let mut fixture = Fixture::responses(
        "200 OK",
        &[
            ("*", &old),
            ("*", &old),
            ("*", &old),
            ("*", &old),
            ("*", power),
            ("*", pending),
            ("*", &changed),
            ("*", &changed),
            ("*", &changed),
            ("*", power),
            ("*", pending),
        ],
        "",
        Duration::ZERO,
        true,
    )
    .await;
    let provider = fixture.client();
    let (_directory, store, operation) = completed_resources_for_disk(&provider);
    let catalog =
        crate::provisioning::Catalog::parse(include_str!("../../../examples/templates.yaml"))
            .unwrap();
    assert!(run_disk(&store, Some(&provider), Some(&catalog), true)
        .await
        .unwrap());
    let job = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert!(job.stages[2].dispatch_started);
    assert!(job.stages[2].task.is_none());
    assert_eq!(job.stages[2].status, Status::Uncertain);
    assert_eq!(
        job.operation.failure,
        Some(Failure::ProviderSubmissionUnconfirmed)
    );
    assert_eq!(job.disk_dispatch.unwrap().config_digest, "e".repeat(40));
    assert!(!run_disk(&store, Some(&provider), Some(&catalog), true)
        .await
        .unwrap());
    let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
    assert_eq!(requests.len(), 11);
    assert!(requests.iter().all(|r| r.starts_with("GET ")));
}

fn cloud_init_request() -> super::CloudInitRequest {
    full_clone_plan()
        .cloud_init_request(
            &crate::definition::Definition::parse(include_str!(
                "../../../examples/kernel-lab.yaml"
            ))
            .unwrap(),
            &"a".repeat(32),
            &"e".repeat(40),
            "local-lvm:vm-202-disk-0",
            "virtio=BC:24:11:12:34:56,bridge=old-br,firewall=1",
        )
        .unwrap()
}
fn cloud_init_config(applied: bool) -> serde_json::Value {
    let mut config = disk_config("local-lvm:vm-202-disk-0,size=40G");
    config["digest"] = serde_json::json!(if applied {
        "f".repeat(40)
    } else {
        "e".repeat(40)
    });
    config["ide2"] = serde_json::json!("local-lvm:vm-202-cloudinit,media=cdrom,size=4M");
    config["net0"] = serde_json::json!(if applied {
        "virtio=BC:24:11:12:34:56,firewall=1,bridge=vmbr0"
    } else {
        "virtio=BC:24:11:12:34:56,bridge=old-br,firewall=1"
    });
    if applied {
        config["ipconfig0"] = serde_json::json!("ip=dhcp");
        config["citype"] = serde_json::json!("nocloud");
        config["ciuser"] = serde_json::json!("debian");
        config["ciupgrade"] = serde_json::json!(0);
        config["sshkeys"] = serde_json::json!(
            "ssh-ed25519%20AAAAC3NzaC1lZDI1NTE5AAAAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        );
    } else {
        config["cicustom"] = serde_json::json!("user=local:snippets/inherited.yaml");
        config["cipassword"] = serde_json::json!("fixture-password-placeholder");
    }
    config
}
async fn cloud_init_fixture(config: serde_json::Value, reply: &str) -> Fixture {
    let config = serde_json::json!({"data":config}).to_string();
    Fixture::responses(
        "200 OK",
        &[
            ("*", &config),
            ("*", &config),
            ("*", &config),
            ("*", r#"{"data":{"vmid":202,"status":"stopped"}}"#),
            ("*", r#"{"data":[]}"#),
            ("*", reply),
        ],
        "",
        Duration::ZERO,
        true,
    )
    .await
}
#[tokio::test]
async fn cloud_init_transport_binds_definition_network_and_encodes_comment_free_public_keys() {
    let request = cloud_init_request();
    let mut fixture = cloud_init_fixture(cloud_init_config(false), r#"{"data":null}"#).await;
    fixture
        .client()
        .dispatch_cloud_init(&request)
        .await
        .unwrap();
    let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
    assert_eq!(requests.len(), 6);
    assert!(requests[..5].iter().all(|r| r.starts_with("GET ")));
    assert!(requests[5].starts_with("PUT /api2/json/nodes/pve-test/qemu/202/config "));
    let form: std::collections::BTreeMap<_, _> =
        form_urlencoded::parse(requests[5].split_once("\r\n\r\n").unwrap().1.as_bytes())
            .into_owned()
            .collect();
    assert_eq!(form.len(), 8);
    assert_eq!(form["digest"], "e".repeat(40));
    assert_eq!(
        form["net0"],
        "virtio=BC:24:11:12:34:56,bridge=vmbr0,firewall=1"
    );
    assert_eq!(form["ipconfig0"], "ip=dhcp");
    assert_eq!(form["citype"], "nocloud");
    assert_eq!(form["ciuser"], "debian");
    assert_eq!(form["ciupgrade"], "0");
    assert_eq!(form["delete"], "cicustom,cipassword");
    assert_eq!(
        form["sshkeys"],
        "ssh-ed25519%20AAAAC3NzaC1lZDI1NTE5AAAAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    );
    assert!(!format!("{request:?}").contains("AAAAC3"));
    let definition =
        crate::definition::Definition::parse(include_str!("../../../examples/kernel-lab.yaml"))
            .unwrap();
    for network in [
        "virtio=00:00:00:00:00:00,bridge=vmbr0",
        "virtio=01:24:11:12:34:56,bridge=vmbr0",
        "virtio=BC:24:11:12:34:56,bridge=vmbr0,tag=10",
        "virtio=BC:24:11:12:34:56,bridge=vmbr0,bridge=other",
        "e1000=BC:24:11:12:34:56,bridge=vmbr0",
    ] {
        assert!(full_clone_plan()
            .cloud_init_request(
                &definition,
                &"a".repeat(32),
                &"e".repeat(40),
                "local-lvm:vm-202-disk-0",
                network
            )
            .is_err());
    }
    let changed = crate::definition::Definition::parse(
        &include_str!("../../../examples/kernel-lab.yaml")
            .replace("memoryMiB: 8192", "memoryMiB: 4096"),
    )
    .unwrap();
    assert!(full_clone_plan()
        .cloud_init_request(
            &changed,
            &"a".repeat(32),
            &"e".repeat(40),
            "local-lvm:vm-202-disk-0",
            "virtio=BC:24:11:12:34:56,bridge=vmbr0"
        )
        .is_err());
}
#[tokio::test]
async fn cloud_init_transport_rejects_unproven_disk_drive_network_or_revision_before_put() {
    let request = cloud_init_request();
    for variant in [
        "disk",
        "drive",
        "network",
        "revision",
        "extra_net",
        "extra_ip",
        "memory",
        "marker",
    ] {
        let mut config = cloud_init_config(false);
        match variant {
            "disk" => config["scsi0"] = serde_json::json!("local-lvm:vm-202-disk-0,size=8G"),
            "drive" => config["ide2"] = serde_json::json!("local-lvm:vm-203-cloudinit,media=cdrom"),
            "network" => {
                config["net0"] =
                    serde_json::json!("virtio=BC:24:11:12:34:57,bridge=old-br,firewall=1")
            }
            "revision" => config["digest"] = serde_json::json!("f".repeat(40)),
            "extra_net" => {
                config["net1"] = serde_json::json!("virtio=BC:24:11:12:34:57,bridge=other")
            }
            "extra_ip" => config["ipconfig1"] = serde_json::json!("ip=dhcp"),
            "memory" => config["memory"] = serde_json::json!(1024),
            _ => config["description"] = serde_json::json!("foreign"),
        }
        let mut fixture = cloud_init_fixture(config, r#"{"data":null}"#).await;
        assert_eq!(
            fixture
                .client()
                .dispatch_cloud_init(&request)
                .await
                .unwrap_err(),
            Error::InvalidResponse,
            "{variant}"
        );
        assert!(
            std::iter::from_fn(|| fixture.requests.try_recv().ok()).all(|r| r.starts_with("GET "))
        );
    }
    for reply in [r#"{"data":"unexpected-task"}"#, "truncated", DROP_REPLY] {
        let mut fixture = cloud_init_fixture(cloud_init_config(false), reply).await;
        assert!(fixture
            .client()
            .dispatch_cloud_init(&request)
            .await
            .is_err());
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert_eq!(requests.iter().filter(|r| r.starts_with("PUT ")).count(), 1);
    }
}
#[tokio::test]
async fn cloud_init_observation_requires_exact_settings_without_claiming_seed_or_guest_readiness() {
    let request = cloud_init_request();
    for variant in [
        "valid", "keys", "password", "custom", "user", "upgrade", "format", "ip", "network",
    ] {
        let mut config = cloud_init_config(true);
        match variant {
            "keys" => config["sshkeys"] = serde_json::json!("another-key"),
            "password" => config["cipassword"] = serde_json::json!("masked"),
            "custom" => config["cicustom"] = serde_json::json!("user=local:snippets/old.yaml"),
            "user" => config["ciuser"] = serde_json::json!("root"),
            "upgrade" => config["ciupgrade"] = serde_json::json!(1),
            "format" => config["citype"] = serde_json::json!("configdrive2"),
            "ip" => config["ipconfig0"] = serde_json::json!("ip=dhcp,ip6=auto"),
            "network" => {
                config["net0"] =
                    serde_json::json!("virtio=BC:24:11:12:34:56,bridge=other,firewall=1")
            }
            _ => {}
        }
        let mut fixture = cloud_init_fixture(config, "unused").await;
        let observed = fixture.client().observe_cloud_init(&request).await;
        if variant == "valid" {
            let observed = observed.unwrap();
            assert_eq!(observed.guest_user, "debian");
            assert_eq!(observed.ssh_key_count, 1);
            assert!(!observed.guest_ready);
            assert!(!serde_json::to_string(&observed).unwrap().contains("AAAAC3"));
        } else {
            assert_eq!(observed.unwrap_err(), Error::InvalidResponse, "{variant}");
        }
        assert!(
            std::iter::from_fn(|| fixture.requests.try_recv().ok()).all(|r| r.starts_with("GET "))
        );
    }
}

#[tokio::test]
async fn cloud_init_transport_preserves_multiple_key_material_through_both_uri_encoding_layers() {
    let mut definition =
        crate::definition::Definition::parse(include_str!("../../../examples/kernel-lab.yaml"))
            .unwrap();
    let mut key = ssh_key::PublicKey::from(ssh_key::public::Ed25519PublicKey([255; 32]));
    key.set_comment("ignored%0Acomment: value");
    definition
        .spec
        .provisioning
        .ssh_authorized_keys
        .push(key.to_openssh().unwrap());
    let catalog =
        crate::provisioning::Catalog::parse(include_str!("../../../examples/templates.yaml"))
            .unwrap();
    let plan = catalog.plan(&definition, 202).unwrap();
    let request = plan
        .cloud_init_request(
            &definition,
            &"a".repeat(32),
            &"e".repeat(40),
            "local-lvm:vm-202-disk-0",
            "virtio=BC:24:11:12:34:56,bridge=old-br,firewall=1",
        )
        .unwrap();
    let mut config = cloud_init_config(false);
    config["description"] = serde_json::json!(clone_description(&plan));
    let mut fixture = cloud_init_fixture(config, r#"{"data":null}"#).await;
    fixture
        .client()
        .dispatch_cloud_init(&request)
        .await
        .unwrap();
    let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
    let body = requests[5].split_once("\r\n\r\n").unwrap().1;
    let form: std::collections::BTreeMap<_, _> = form_urlencoded::parse(body.as_bytes())
        .into_owned()
        .collect();
    assert!(form["sshkeys"].contains("%0A"));
    assert!(form["sshkeys"].contains("%2F"));
    assert!(!form["sshkeys"].contains("comment"));
    let decoded: Vec<_> = form_urlencoded::parse(format!("keys={}", form["sshkeys"]).as_bytes())
        .into_owned()
        .collect();
    let actual: Vec<_> = decoded[0]
        .1
        .lines()
        .map(|v| {
            ssh_key::PublicKey::from_openssh(v)
                .unwrap()
                .key_data()
                .clone()
        })
        .collect();
    let expected: Vec<_> = definition
        .spec
        .provisioning
        .ssh_authorized_keys
        .iter()
        .map(|v| {
            ssh_key::PublicKey::from_openssh(v)
                .unwrap()
                .key_data()
                .clone()
        })
        .collect();
    assert_eq!(actual, expected);
}

fn completed_disk_for_cloud_init(
    provider: &Proxmox,
) -> (
    tempfile::TempDir,
    crate::store::Store,
    crate::operation::Operation,
) {
    completed_disk_for_cloud_init_with_yaml(
        provider,
        include_str!("../../../examples/kernel-lab.yaml"),
    )
}
fn completed_disk_for_cloud_init_with_yaml(
    provider: &Proxmox,
    yaml: &str,
) -> (
    tempfile::TempDir,
    crate::store::Store,
    crate::operation::Operation,
) {
    let (directory, store, operation) = completed_resources_for_disk_with_yaml(provider, yaml);
    let job = store.claim_provisioning_disk().unwrap().unwrap();
    store
        .begin_provisioning_disk(
            &operation.id,
            &job.approval,
            &"e".repeat(40),
            "local-lvm:vm-202-disk-0",
        )
        .unwrap();
    store
        .link_provisioning_disk(
            &operation.id,
            &super::TaskReference::new("pve-test", RESIZE_TASK).unwrap(),
        )
        .unwrap();
    store
        .finish_provisioning_disk(
            &operation.id,
            crate::provisioning_intent::DiskOutcome::Completed(
                crate::provisioning_intent::DiskResult {
                    instance_id: operation.instance_id.clone().unwrap(),
                    provider_scope_id: provider.scope_id(),
                    plan_hash: job.approval.plan_hash,
                    task_status: super::TaskStatus::Succeeded,
                    disk: super::DiskObservation {
                        device: "scsi0".into(),
                        volume: "local-lvm:vm-202-disk-0".into(),
                        size_bytes: 40 * 1024_u64.pow(3),
                        resources: super::ResourceObservation {
                            cpu: 4,
                            memory_mib: 8192,
                            target: super::CloneObservation {
                                config_digest: "f".repeat(40),
                                observation: super::VmObservation {
                                    node: "pve-test".into(),
                                    vmid: 202,
                                    status: super::PowerState::Stopped,
                                    qmp_status: None,
                                },
                            },
                        },
                    },
                },
            ),
        )
        .unwrap();
    (directory, store, operation)
}
async fn cloud_init_executor_fixture(reply: &str, applied: bool, reconcile_only: bool) -> Fixture {
    let mut old = cloud_init_config(false);
    old["digest"] = serde_json::json!("f".repeat(40));
    let old = serde_json::json!({"data":old}).to_string();
    let mut current = cloud_init_config(applied);
    current["digest"] = serde_json::json!("a".repeat(40));
    let current = serde_json::json!({"data":current}).to_string();
    let power = r#"{"data":{"vmid":202,"status":"stopped"}}"#;
    let pending = r#"{"data":[]}"#;
    let mut responses = Vec::new();
    if !reconcile_only {
        responses.extend_from_slice(&[
            ("*", old.as_str()),
            ("*", old.as_str()),
            ("*", old.as_str()),
            ("*", old.as_str()),
            ("*", power),
            ("*", pending),
            ("*", old.as_str()),
            ("*", old.as_str()),
            ("*", old.as_str()),
            ("*", power),
            ("*", pending),
            ("*", reply),
        ]);
    }
    responses.extend_from_slice(&[
        ("*", current.as_str()),
        ("*", current.as_str()),
        ("*", current.as_str()),
        ("*", power),
        ("*", pending),
    ]);
    Fixture::responses("200 OK", &responses, "", Duration::ZERO, true).await
}
#[tokio::test]
async fn cloud_init_executor_writes_once_and_records_only_api_settings_after_independent_observation(
) {
    use crate::{
        operation::{Failure, Status},
        provisioning_executor::run_cloud_init,
    };
    let catalog =
        crate::provisioning::Catalog::parse(include_str!("../../../examples/templates.yaml"))
            .unwrap();
    for (reply, applied) in [
        (r#"{"data":null}"#, true),
        (DROP_REPLY, true),
        ("malformed", true),
        (DROP_REPLY, false),
        (r#"{"data":null}"#, false),
    ] {
        let mut fixture = cloud_init_executor_fixture(reply, applied, false).await;
        let provider = fixture.client();
        let (_directory, store, operation) = completed_disk_for_cloud_init(&provider);
        assert!(
            run_cloud_init(&store, Some(&provider), Some(&catalog), true)
                .await
                .unwrap()
        );
        let job = store.provisioning_job(&operation.id).unwrap().unwrap();
        assert_eq!(
            job.stages[3].status,
            if applied {
                Status::Succeeded
            } else {
                Status::Uncertain
            }
        );
        assert!(job.stages[..3]
            .iter()
            .all(|s| s.status == Status::Succeeded));
        assert!(job.stages[4..].iter().all(|s| s.status == Status::Queued));
        assert!(job.operation.result.is_none());
        if applied {
            assert!(
                !job.stages[3].result.as_ref().unwrap()["settings"]["guestReady"]
                    .as_bool()
                    .unwrap()
            );
        } else {
            assert_eq!(
                job.operation.failure,
                Some(if reply == DROP_REPLY {
                    Failure::ProviderSubmissionUnconfirmed
                } else {
                    Failure::CloudInitStateUnconfirmed
                })
            );
        }
        assert!(
            !run_cloud_init(&store, Some(&provider), Some(&catalog), true)
                .await
                .unwrap()
        );
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert_eq!(requests.iter().filter(|r| r.starts_with("PUT ")).count(), 1);
        assert!(requests.iter().all(|r| r.starts_with("GET ")
            || r.starts_with("PUT /api2/json/nodes/pve-test/qemu/202/config ")));
        assert_eq!(requests.len(), 17);
    }
}
#[tokio::test]
async fn cloud_init_executor_restart_uses_only_get_without_creation_capability_or_catalog() {
    use crate::{operation::Status, provisioning_executor::run_cloud_init};
    let mut fixture = cloud_init_executor_fixture("unused", true, true).await;
    let provider = fixture.client();
    let (directory, store, operation) = completed_disk_for_cloud_init(&provider);
    let job = store.claim_provisioning_cloud_init().unwrap().unwrap();
    store
        .begin_provisioning_cloud_init(
            &operation.id,
            &job.approval,
            &"f".repeat(40),
            "local-lvm:vm-202-disk-0",
            "virtio=BC:24:11:12:34:56,bridge=old-br,firewall=1",
        )
        .unwrap();
    drop(store);
    let reopened = crate::store::Store::open(&directory.path().join("clone.sqlite")).unwrap();
    assert_eq!(reopened.recover_interrupted_operations().unwrap(), 1);
    assert!(run_cloud_init(&reopened, Some(&provider), None, false)
        .await
        .unwrap());
    let job = reopened.provisioning_job(&operation.id).unwrap().unwrap();
    assert_eq!(job.stages[3].status, Status::Succeeded);
    assert_eq!(job.operation.status, Status::Running);
    let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
    assert_eq!(requests.len(), 5);
    assert!(requests.iter().all(|r| r.starts_with("GET ")));
    assert!(reopened.claim_provisioning_clone().unwrap().is_none());
    assert!(reopened.claim_provisioning_disk().unwrap().is_none());
}
#[tokio::test]
async fn cloud_init_executor_rejects_disabled_changed_or_unproven_inputs_before_marker() {
    use crate::{operation::Status, provisioning_executor::run_cloud_init};
    let catalog =
        crate::provisioning::Catalog::parse(include_str!("../../../examples/templates.yaml"))
            .unwrap();
    for variant in [
        "disabled", "catalog", "scope", "volume", "revision", "network", "drive",
    ] {
        let mut config = cloud_init_config(false);
        config["digest"] = serde_json::json!("f".repeat(40));
        match variant {
            "volume" => config["scsi0"] = serde_json::json!("local-lvm:vm-203-disk-0,size=40G"),
            "revision" => config["digest"] = serde_json::json!("e".repeat(40)),
            "network" => {
                config["net0"] = serde_json::json!("virtio=BC:24:11:12:34:56,bridge=old-br,tag=10")
            }
            "drive" => config["ide2"] = serde_json::json!("local-lvm:vm-203-cloudinit,media=cdrom"),
            _ => {}
        }
        let mut fixture = Fixture::routes(
            "200 OK",
            &[
                (
                    "/api2/json/nodes/pve-test/qemu/202/config?current=1",
                    &serde_json::json!({"data":config}).to_string(),
                ),
                (
                    "/api2/json/nodes/pve-test/qemu/202/status/current",
                    r#"{"data":{"vmid":202,"status":"stopped"}}"#,
                ),
                (
                    "/api2/json/nodes/pve-test/qemu/202/pending",
                    r#"{"data":[]}"#,
                ),
            ],
            "",
            Duration::ZERO,
        )
        .await;
        let provider = fixture.client();
        let (_directory, store, operation) = completed_disk_for_cloud_init(&provider);
        let foreign = Fixture::new("200 OK", r#"{"data":{}}"#, "", Duration::ZERO).await;
        let foreign_provider = foreign.client();
        assert!(run_cloud_init(
            &store,
            Some(if variant == "scope" {
                &foreign_provider
            } else {
                &provider
            }),
            if variant == "catalog" {
                None
            } else {
                Some(&catalog)
            },
            variant != "disabled"
        )
        .await
        .unwrap());
        let job = store.provisioning_job(&operation.id).unwrap().unwrap();
        assert_eq!(job.stages[3].status, Status::Failed);
        assert!(!job.stages[3].dispatch_started);
        assert!(job.cloud_init_dispatch.is_none());
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert!(requests.iter().all(|r| r.starts_with("GET ")));
        if ["disabled", "catalog", "scope"].contains(&variant) {
            assert!(requests.is_empty());
        }
    }
}

fn seed_user(named: bool) -> String {
    format!("#cloud-config\nhostname: kernel-lab\nmanage_etc_hosts: true\nfqdn: kernel-lab\n{}\nusers: [default]\nssh_authorized_keys:\n  - ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\nchpasswd:\n  expire: False\n",if named {"user: {name: debian}"}else{"user: debian"})
}
const SEED_NETWORK:&str="version: 1\nconfig:\n  - type: physical\n    name: eth0\n    mac_address: 'bc:24:11:12:34:56'\n    subnets:\n      - type: dhcp4\n  - type: nameserver\n    address: [192.0.2.53]\n    search: [example.test]\n";
async fn seed_fixture(user: &str, network: &str, changed: bool) -> Fixture {
    seed_fixture_config(user, network, changed, cloud_init_config(true)).await
}
async fn seed_fixture_config(
    user: &str,
    network: &str,
    changed: bool,
    before: serde_json::Value,
) -> Fixture {
    let mut after = before.clone();
    if changed {
        after["digest"] = serde_json::json!("a".repeat(40));
    }
    let before = serde_json::json!({"data":before}).to_string();
    let after = serde_json::json!({"data":after}).to_string();
    let user = serde_json::json!({"data":user}).to_string();
    let network = serde_json::json!({"data":network}).to_string();
    let power = r#"{"data":{"vmid":202,"status":"stopped"}}"#;
    let pending = r#"{"data":[]}"#;
    Fixture::responses(
        "200 OK",
        &[
            ("*", &before),
            ("*", &before),
            ("*", &before),
            ("*", power),
            ("*", pending),
            ("*", &user),
            ("*", &network),
            ("*", &after),
            ("*", &after),
            ("*", &after),
            ("*", power),
            ("*", pending),
        ],
        "",
        Duration::ZERO,
        true,
    )
    .await
}
#[tokio::test]
async fn generated_seed_preflight_distinguishes_scalar_user_without_exposing_seed_or_guest_readiness(
) {
    let request = cloud_init_request();
    for named in [true, false] {
        let mut fixture = seed_fixture(&seed_user(named), SEED_NETWORK, false).await;
        let observed = fixture
            .client()
            .preflight_generated_seed(&request)
            .await
            .unwrap();
        assert_eq!(observed.compatible, named);
        assert!(!observed.guest_ready);
        assert_eq!(
            observed.user_format,
            if named {
                super::SeedUserFormat::Named
            } else {
                super::SeedUserFormat::ScalarUnsupported
            }
        );
        let serialized = serde_json::to_string(&observed).unwrap();
        assert!(!serialized.contains("AAAAC3"));
        assert!(!serialized.contains("192.0.2.53"));
        assert_eq!(observed.user_data_hash.len(), 64);
        assert_eq!(observed.network_data_hash.len(), 64);
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert_eq!(requests.len(), 12);
        assert!(requests.iter().all(|r| r.starts_with("GET ")));
        assert!(requests[5]
            .starts_with("GET /api2/json/nodes/pve-test/qemu/202/cloudinit/dump?type=user "));
        assert!(requests[6]
            .starts_with("GET /api2/json/nodes/pve-test/qemu/202/cloudinit/dump?type=network "));
    }
}
#[tokio::test]
async fn generated_seed_preflight_rejects_unexpected_user_keys_commands_network_and_revision() {
    let request = cloud_init_request();
    let baseline = seed_user(true);
    let variants = vec![
        baseline.replace("name: debian", "name: root"),
        baseline.replace("users: [default]", "users: [root]"),
        baseline.replace("hostname: kernel-lab", "hostname: foreign"),
        baseline.replace("expire: False", "expire: True"),
        format!("{baseline}runcmd: [unexpected]\n"),
        format!("{baseline}password: unexpected\n"),
        format!("{baseline}package_upgrade: true\n"),
        format!("{baseline}hostname: duplicated\n"),
        baseline.replace(
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "invalid-key",
        ),
    ];
    for user in variants {
        let mut fixture = seed_fixture(&user, SEED_NETWORK, false).await;
        assert_eq!(
            fixture
                .client()
                .preflight_generated_seed(&request)
                .await
                .unwrap_err(),
            Error::InvalidResponse
        );
        assert!(
            std::iter::from_fn(|| fixture.requests.try_recv().ok()).all(|r| r.starts_with("GET "))
        );
    }
    for network in [
        SEED_NETWORK.replace("eth0", "eth1"),
        SEED_NETWORK.replace("34:56", "34:57"),
        SEED_NETWORK.replace("dhcp4", "dhcp6"),
        SEED_NETWORK.replace("version: 1", "version: 2"),
        SEED_NETWORK.replace("192.0.2.53", "not-an-address"),
    ] {
        let fixture = seed_fixture(&baseline, &network, false).await;
        assert_eq!(
            fixture
                .client()
                .preflight_generated_seed(&request)
                .await
                .unwrap_err(),
            Error::InvalidResponse
        );
    }
    let fixture = seed_fixture(&baseline, SEED_NETWORK, true).await;
    assert_eq!(
        fixture
            .client()
            .preflight_generated_seed(&request)
            .await
            .unwrap_err(),
        Error::InvalidResponse
    );
}

#[tokio::test]
async fn image_default_cloud_init_requires_attestation_deletes_ciuser_and_checks_seed() {
    let catalog = crate::provisioning::Catalog::parse(&include_str!("../../../examples/templates.yaml").replace(
        "    guestUser: debian", "    guestUser: debian\n    imageDefaultUser: {name: debian, templateVersion: debian-13-cloud-20261016, passwordLocked: true, noAuthorizedKeys: true}" )).unwrap();
    let definition =
        crate::definition::Definition::parse(include_str!("../../../examples/kernel-lab.yaml"))
            .unwrap();
    let plan = catalog.plan(&definition, 202).unwrap();
    let request = plan
        .cloud_init_request(
            &definition,
            &"a".repeat(32),
            &"e".repeat(40),
            "local-lvm:vm-202-disk-0",
            "virtio=BC:24:11:12:34:56,bridge=old-br,firewall=1",
        )
        .unwrap();
    let mut config = cloud_init_config(false);
    config["description"] = serde_json::json!(clone_description(&plan));
    let mut fixture = cloud_init_fixture(config, r#"{"data":null}"#).await;
    fixture
        .client()
        .dispatch_cloud_init(&request)
        .await
        .unwrap();
    let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
    let form: std::collections::BTreeMap<_, _> =
        form_urlencoded::parse(requests[5].split_once("\r\n\r\n").unwrap().1.as_bytes())
            .into_owned()
            .collect();
    assert!(!form.contains_key("ciuser"));
    assert_eq!(form["delete"], "cicustom,cipassword,ciuser");
    let mut applied = cloud_init_config(true);
    applied["description"] = serde_json::json!(clone_description(&plan));
    applied.as_object_mut().unwrap().remove("ciuser");
    let omitted = seed_user(true).replace("user: {name: debian}\n", "");
    for (user, allowed) in [
        (omitted.clone(), true),
        (seed_user(true), false),
        (seed_user(false), false),
        (
            omitted.replace("users: [default]", "user: null\nusers: [default]"),
            false,
        ),
    ] {
        let fixture = seed_fixture_config(&user, SEED_NETWORK, false, applied.clone()).await;
        let outcome = fixture.client().preflight_generated_seed(&request).await;
        if allowed {
            let observed = outcome.unwrap();
            assert!(observed.compatible);
            assert_eq!(observed.user_format, super::SeedUserFormat::ImageDefault);
            assert!(!observed.guest_ready);
        } else {
            assert!(outcome.is_err());
        }
    }
    let fixture = seed_fixture(&omitted, SEED_NETWORK, false).await;
    assert!(fixture
        .client()
        .preflight_generated_seed(&cloud_init_request())
        .await
        .is_err());
    applied["ciuser"] = serde_json::json!("debian");
    let fixture = seed_fixture_config(&omitted, SEED_NETWORK, false, applied).await;
    assert!(fixture
        .client()
        .preflight_generated_seed(&request)
        .await
        .is_err());
}

#[tokio::test]
async fn seed_regeneration_checks_compatible_previews_and_sends_one_fixed_synchronous_put() {
    // Generated fixtures only; no provider configuration or real VM access.
    for variant in [
        "success",
        "incompatible",
        "revision",
        "hash",
        "lost",
        "task",
        "changed",
    ] {
        let user = seed_user(true);
        let config = serde_json::json!({"data":cloud_init_config(true)}).to_string();
        let user_body = serde_json::json!({"data":&user}).to_string();
        let changed_user = serde_json::json!({"data":user.replace("fqdn: kernel-lab\n", "fqdn: kernel-lab.example.test\n")}).to_string();
        let network = serde_json::json!({"data":SEED_NETWORK}).to_string();
        let power = r#"{"data":{"vmid":202,"status":"stopped"}}"#;
        let pending = r#"{"data":[]}"#;
        let mut responses = Vec::new();
        // Independent expected preflight, immediate pre-dispatch and post-dispatch.
        for round in 0..3 {
            responses.extend([
                ("*", config.as_str()),
                ("*", config.as_str()),
                ("*", config.as_str()),
                ("*", power),
                ("*", pending),
                (
                    "*",
                    if round == 2 && variant == "changed" {
                        changed_user.as_str()
                    } else {
                        user_body.as_str()
                    },
                ),
                ("*", network.as_str()),
                ("*", config.as_str()),
                ("*", config.as_str()),
                ("*", config.as_str()),
                ("*", power),
                ("*", pending),
            ]);
            if round == 1 {
                responses.push((
                    "*",
                    match variant {
                        "lost" => DROP_REPLY,
                        "task" => r#"{"data":"unexpected-task"}"#,
                        _ => r#"{"data":null}"#,
                    },
                ));
            }
        }
        let mut fixture = Fixture::responses("200 OK", &responses, "", Duration::ZERO, true).await;
        let provider = fixture.client();
        let request = cloud_init_request();
        let mut expected = provider.preflight_generated_seed(&request).await.unwrap();
        if variant == "incompatible" {
            expected.compatible = false;
        }
        if variant == "revision" {
            expected.config_digest = "a".repeat(40);
        }
        if variant == "hash" {
            expected.user_data_hash = "0".repeat(64);
        }
        let result = provider
            .dispatch_seed_regeneration(&request, &expected)
            .await;
        if variant == "success" {
            let proof = result.unwrap();
            assert!(proof.compatible);
            assert!(!proof.guest_ready);
            assert_eq!(proof.user_data_hash, expected.user_data_hash);
        } else {
            assert!(result.is_err(), "{variant}");
        }
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        let writes: Vec<_> = requests.iter().filter(|r| !r.starts_with("GET ")).collect();
        if ["incompatible", "revision", "hash"].contains(&variant) {
            assert!(writes.is_empty());
        } else {
            assert_eq!(writes.len(), 1, "{variant}");
            assert!(writes[0].starts_with("PUT /api2/json/nodes/pve-test/qemu/202/cloudinit "));
            assert!(writes[0]
                .to_ascii_lowercase()
                .contains("authorization: pveapitoken="));
            assert_eq!(writes[0].split_once("\r\n\r\n").unwrap().1, "");
        }
    }
}

#[tokio::test]
async fn seed_executor_persists_regeneration_and_restart_never_replays_uncertain_writes() {
    use crate::{
        operation::{Failure, Status},
        provisioning_executor::run_seed,
        provisioning_intent::{CloudInitOutcome, CloudInitResult},
    };
    let catalog =
        crate::provisioning::Catalog::parse(include_str!("../../../examples/templates.yaml"))
            .unwrap();
    for variant in [
        "success", "lost", "disabled", "catalog", "provider", "scalar",
    ] {
        let mut config = cloud_init_config(true);
        config["digest"] = serde_json::json!("a".repeat(40));
        let config = serde_json::json!({"data":config}).to_string();
        let user = serde_json::json!({"data":seed_user(variant!="scalar")}).to_string();
        let network = serde_json::json!({"data":SEED_NETWORK}).to_string();
        let power = r#"{"data":{"vmid":202,"status":"stopped"}}"#;
        let pending = r#"{"data":[]}"#;
        let mut responses = vec![
            ("*", config.as_str()),
            ("*", config.as_str()),
            ("*", config.as_str()),
            ("*", power),
            ("*", pending),
        ];
        for round in 0..3 {
            responses.extend([
                ("*", config.as_str()),
                ("*", config.as_str()),
                ("*", config.as_str()),
                ("*", power),
                ("*", pending),
                ("*", user.as_str()),
                ("*", network.as_str()),
                ("*", config.as_str()),
                ("*", config.as_str()),
                ("*", config.as_str()),
                ("*", power),
                ("*", pending),
            ]);
            if round == 1 {
                responses.push((
                    "*",
                    if variant == "lost" {
                        DROP_REPLY
                    } else {
                        r#"{"data":null}"#
                    },
                ));
            }
        }
        let mut fixture = Fixture::responses("200 OK", &responses, "", Duration::ZERO, true).await;
        let provider = fixture.client();
        let (directory, store, operation) = completed_disk_for_cloud_init(&provider);
        let job = store.claim_provisioning_cloud_init().unwrap().unwrap();
        store
            .begin_provisioning_cloud_init(
                &operation.id,
                &job.approval,
                &"f".repeat(40),
                "local-lvm:vm-202-disk-0",
                "virtio=BC:24:11:12:34:56,bridge=old-br,firewall=1",
            )
            .unwrap();
        let request = job
            .plan
            .cloud_init_request(
                &job.definition,
                job.operation.instance_id.as_deref().unwrap(),
                &"f".repeat(40),
                "local-lvm:vm-202-disk-0",
                "virtio=BC:24:11:12:34:56,bridge=old-br,firewall=1",
            )
            .unwrap();
        let settings = provider.observe_cloud_init(&request).await.unwrap();
        store
            .finish_provisioning_cloud_init(
                &operation.id,
                CloudInitOutcome::Completed(Box::new(CloudInitResult {
                    instance_id: operation.instance_id.clone().unwrap(),
                    provider_scope_id: provider.scope_id(),
                    plan_hash: job.approval.plan_hash,
                    settings,
                })),
            )
            .unwrap();
        assert!(run_seed(
            &store,
            if variant == "provider" {
                None
            } else {
                Some(&provider)
            },
            if variant == "catalog" {
                None
            } else {
                Some(&catalog)
            },
            variant != "disabled"
        )
        .await
        .unwrap());
        let retained = store.provisioning_job(&operation.id).unwrap().unwrap();
        assert_eq!(retained.regenerated_seed().is_some(), variant == "success");
        let start = retained
            .stages
            .iter()
            .find(|s| s.kind == "start_vm")
            .unwrap();
        if variant == "lost" {
            assert_eq!(start.status, Status::Uncertain);
            assert_eq!(
                retained.operation.failure,
                Some(Failure::SeedRegenerationUnconfirmed)
            );
        } else if variant != "success" {
            assert_eq!(start.status, Status::Failed);
            assert!(!start.dispatch_started);
        }
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert_eq!(
            requests.iter().filter(|r| r.starts_with("PUT ")).count(),
            usize::from(["success", "lost"].contains(&variant)),
            "{variant}"
        );
        drop(store);
        let reopened = crate::store::Store::open(directory.path().join("clone.sqlite")).unwrap();
        reopened.recover_interrupted_operations().unwrap();
        assert!(!run_seed(&reopened, Some(&provider), Some(&catalog), true)
            .await
            .unwrap());
        assert_eq!(
            reopened
                .provisioning_job(&operation.id)
                .unwrap()
                .unwrap()
                .regenerated_seed()
                .is_some(),
            variant == "success"
        );
        assert!(
            fixture.requests.try_recv().is_err(),
            "Restart issued an extra request"
        );
    }
}

#[tokio::test]
async fn provisioning_start_executor_checks_seed_submits_once_and_recovers_known_tasks_get_only() {
    use crate::{
        operation::Status,
        provisioning_executor::{run_seed, run_start},
        provisioning_intent::{CloudInitOutcome, CloudInitResult},
    };
    let catalog =
        crate::provisioning::Catalog::parse(include_str!("../../../examples/templates.yaml"))
            .unwrap();
    let upid = "UPID:pve-test:00000001:00000002:00000003:qmstart:202:fixture@pve:";
    for variant in [
        "success",
        "lost",
        "task_recovery",
        "seed_changed",
        "disabled",
        "failed_task",
        "paused",
    ] {
        let mut config = cloud_init_config(true);
        config["digest"] = serde_json::json!("a".repeat(40));
        let config = serde_json::json!({"data":config}).to_string();
        let user = serde_json::json!({"data":seed_user(true)}).to_string();
        let changed=serde_json::json!({"data":seed_user(true).replace("fqdn: kernel-lab\n","fqdn: kernel-lab.changed.test\n")}).to_string();
        let network = serde_json::json!({"data":SEED_NETWORK}).to_string();
        let power = r#"{"data":{"vmid":202,"status":"stopped"}}"#;
        let pending = r#"{"data":[]}"#;
        let reply = serde_json::json!({"data":upid}).to_string();
        let terminal=serde_json::json!({"data":{"node":"pve-test","upid":upid,"status":"stopped","exitstatus":if variant=="failed_task"{"fixture-error"}else{"OK"}}}).to_string();
        let running=serde_json::json!({"data":{"vmid":202,"status":"running","qmpstatus":if variant=="paused"{"paused"}else{"running"}}}).to_string();
        let mut responses = vec![
            ("*", config.as_str()),
            ("*", config.as_str()),
            ("*", config.as_str()),
            ("*", power),
            ("*", pending),
        ];
        // Real seed executor: expected preview, pre-dispatch and post-dispatch.
        // Fourth round is start's fresh inspection of the retained seed proof.
        for round in 0..4 {
            responses.extend([
                ("*", config.as_str()),
                ("*", config.as_str()),
                ("*", config.as_str()),
                ("*", power),
                ("*", pending),
                (
                    "*",
                    if round == 3 && variant == "seed_changed" {
                        changed.as_str()
                    } else {
                        user.as_str()
                    },
                ),
                ("*", network.as_str()),
                ("*", config.as_str()),
                ("*", config.as_str()),
                ("*", config.as_str()),
                ("*", power),
                ("*", pending),
            ]);
            if round == 1 {
                responses.push(("*", r#"{"data":null}"#));
            }
        }
        responses.push((
            "*",
            if variant == "lost" {
                DROP_REPLY
            } else {
                reply.as_str()
            },
        ));
        if variant == "task_recovery" {
            responses.push(("*", r#"{"data":{"status":"invalid"}}"#));
        }
        responses.extend([("*", terminal.as_str()), ("*", running.as_str())]);
        let mut fixture = Fixture::responses("200 OK", &responses, "", Duration::ZERO, true).await;
        let provider = fixture.client();
        let (directory, store, operation) = completed_disk_for_cloud_init(&provider);
        let job = store.claim_provisioning_cloud_init().unwrap().unwrap();
        store
            .begin_provisioning_cloud_init(
                &operation.id,
                &job.approval,
                &"f".repeat(40),
                "local-lvm:vm-202-disk-0",
                "virtio=BC:24:11:12:34:56,bridge=old-br,firewall=1",
            )
            .unwrap();
        let request = job
            .plan
            .cloud_init_request(
                &job.definition,
                job.operation.instance_id.as_deref().unwrap(),
                &"f".repeat(40),
                "local-lvm:vm-202-disk-0",
                "virtio=BC:24:11:12:34:56,bridge=old-br,firewall=1",
            )
            .unwrap();
        let settings = provider.observe_cloud_init(&request).await.unwrap();
        store
            .finish_provisioning_cloud_init(
                &operation.id,
                CloudInitOutcome::Completed(Box::new(CloudInitResult {
                    instance_id: operation.instance_id.clone().unwrap(),
                    provider_scope_id: provider.scope_id(),
                    plan_hash: job.approval.plan_hash,
                    settings,
                })),
            )
            .unwrap();
        assert!(run_seed(&store, Some(&provider), Some(&catalog), true)
            .await
            .unwrap());
        assert!(run_start(
            &store,
            Some(&provider),
            Some(&catalog),
            variant != "disabled"
        )
        .await
        .unwrap());
        let retained = store.provisioning_job(&operation.id).unwrap().unwrap();
        let stage = retained
            .stages
            .iter()
            .find(|s| s.kind == "start_vm")
            .unwrap();
        assert_eq!(
            stage.status,
            match variant {
                "success" => Status::Succeeded,
                "lost" | "task_recovery" | "paused" => Status::Uncertain,
                _ => Status::Failed,
            },
            "{variant}"
        );
        assert_eq!(
            retained
                .stages
                .iter()
                .find(|s| s.kind == "wait_for_guest_readiness")
                .unwrap()
                .status,
            Status::Queued
        );
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        let posts: Vec<_> = requests.iter().filter(|r| r.starts_with("POST ")).collect();
        assert_eq!(
            posts.len(),
            usize::from(!["disabled", "seed_changed"].contains(&variant)),
            "{variant}"
        );
        if let Some(post) = posts.first() {
            assert!(post.starts_with("POST /api2/json/nodes/pve-test/qemu/202/status/start "));
            assert_eq!(post.split_once("\r\n\r\n").unwrap().1, "");
        }
        drop(store);
        let reopened = crate::store::Store::open(directory.path().join("clone.sqlite")).unwrap();
        reopened.recover_interrupted_operations().unwrap();
        if variant == "task_recovery" {
            // Advance only this owned fixture's reconciliation deadline, without sleeping.
            rusqlite::Connection::open(directory.path().join("clone.sqlite"))
                .unwrap()
                .execute(
                    "UPDATE operations SET next_reconcile_at=0 WHERE id=?1",
                    [&operation.id],
                )
                .unwrap();
            assert!(run_start(&reopened, Some(&provider), None, false)
                .await
                .unwrap());
            let job = reopened.provisioning_job(&operation.id).unwrap().unwrap();
            assert_eq!(
                job.stages
                    .iter()
                    .find(|s| s.kind == "start_vm")
                    .unwrap()
                    .status,
                Status::Succeeded
            );
            let recovered: Vec<_> =
                std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
            assert_eq!(recovered.len(), 2);
            assert!(recovered.iter().all(|r| r.starts_with("GET ")));
        } else if variant != "paused" {
            assert!(!run_start(&reopened, Some(&provider), Some(&catalog), true)
                .await
                .unwrap());
            assert!(fixture.requests.try_recv().is_err());
        }
    }
}

fn clean_cloud_status() -> serde_json::Value {
    let stage =
        serde_json::json!({"errors":[],"recoverable_errors":{},"start":10.0,"finished":11.0});
    serde_json::json!({"status":"done","extended_status":"done","errors":[],"recoverable_errors":{},"init-local":stage,"init":stage,"modules-config":stage,"modules-final":stage})
}
fn guest_network() -> serde_json::Value {
    serde_json::json!({"data":{"result":[{"name":"ens18","hardware-address":"bc:24:11:12:34:56","ip-addresses":[{"ip-address-type":"ipv4","ip-address":"192.0.2.42","prefix":24}]}]}})
}
fn guest_guard_config() -> String {
    serde_json::json!({"data":{"digest":"f".repeat(40),"description":clone_description(&full_clone_plan()),"template":0}}).to_string()
}
fn guest_readiness_request() -> super::ReadinessRequest {
    full_clone_plan()
        .readiness_request(
            &"a".repeat(32),
            "virtio=BC:24:11:12:34:56,bridge=vmbr0,firewall=1",
            &"f".repeat(40),
        )
        .unwrap()
}
#[tokio::test]
async fn guest_readiness_uses_only_fixed_command_scoped_pid_and_clean_expected_interface() {
    for flag in [serde_json::json!(1), serde_json::json!(true)] {
        let config = guest_guard_config();
        let power = r#"{"data":{"vmid":202,"status":"running","qmpstatus":"running"}}"#;
        let pending = r#"{"data":[]}"#;
        let status=serde_json::json!({"data":{"exited":flag,"exitcode":0,"out-data":clean_cloud_status().to_string(),"out-truncated":0,"err-truncated":false}}).to_string();
        let network = guest_network().to_string();
        let mut fixture = Fixture::responses(
            "200 OK",
            &[
                ("*", &config),
                ("*", power),
                ("*", pending),
                ("*", r#"{"data":{"pid":42}}"#),
                ("*", &config),
                ("*", power),
                ("*", pending),
                ("*", &status),
                ("*", &network),
                ("*", &config),
                ("*", power),
                ("*", pending),
            ],
            "",
            Duration::ZERO,
            true,
        )
        .await;
        let provider = fixture.client();
        let request = guest_readiness_request();
        let pid = provider.dispatch_guest_probe(&request).await.unwrap();
        assert_eq!(pid, 42);
        let super::GuestProbe::Ready(proof) =
            provider.observe_guest_probe(&request, pid).await.unwrap()
        else {
            panic!("Expected clean guest evidence");
        };
        assert_eq!(proof.ipv4.to_string(), "192.0.2.42");
        assert!(proof.cloud_init_clean);
        assert_eq!(proof.cloud_init_exit_code, 0);
        assert_eq!(proof.interface, "ens18");
        let safe = serde_json::to_string(&proof).unwrap();
        assert!(!safe.contains("modules-final"));
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert_eq!(requests.len(), 12);
        assert!(requests[3].starts_with("POST /api2/json/nodes/pve-test/qemu/202/agent/exec "));
        let body: serde_json::Value =
            serde_json::from_str(requests[3].split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(
            body,
            serde_json::json!({"command":["/usr/bin/cloud-init","status","--format","json"]})
        );
        assert!(requests[7]
            .starts_with("GET /api2/json/nodes/pve-test/qemu/202/agent/exec-status?pid=42 "));
        assert!(requests[8]
            .starts_with("GET /api2/json/nodes/pve-test/qemu/202/agent/network-get-interfaces "));
        assert_eq!(
            requests.iter().filter(|r| !r.starts_with("GET ")).count(),
            1
        );
    }
}
#[tokio::test]
async fn guest_readiness_pending_errors_ambiguity_and_revision_changes_never_report_ready() {
    for variant in [
        "process_pending",
        "cloud_pending",
        "not_started",
        "broadcast",
        "recoverable",
        "stage_error",
        "exit2",
        "truncated",
        "stderr",
        "missing_stage",
        "wrong_mac",
        "loopback",
        "ambiguous_ip",
        "revision",
        "paused",
    ] {
        let config = guest_guard_config();
        let mut after: serde_json::Value = serde_json::from_str(&config).unwrap();
        if variant == "revision" {
            after["data"]["digest"] = serde_json::json!("e".repeat(40));
        }
        let after = after.to_string();
        let power=serde_json::json!({"data":{"vmid":202,"status":"running","qmpstatus":if variant=="paused"{"paused"}else{"running"}}}).to_string();
        let pending = r#"{"data":[]}"#;
        let mut cloud = clean_cloud_status();
        match variant {
            "recoverable" => {
                cloud["recoverable_errors"] =
                    serde_json::json!({"DEPRECATED":["PRIVATE_GUEST_OUTPUT"]})
            }
            "stage_error" => {
                cloud["modules-final"]["errors"] = serde_json::json!(["PRIVATE_GUEST_OUTPUT"])
            }
            "cloud_pending" | "not_started" => {
                cloud["status"] = serde_json::json!(if variant == "not_started" {
                    "not started"
                } else {
                    "running"
                });
                cloud["extended_status"] = cloud["status"].clone();
                cloud["modules-final"] = serde_json::Value::Null;
            }
            "missing_stage" => {
                cloud.as_object_mut().unwrap().remove("modules-final");
            }
            _ => {}
        }
        let mut status = serde_json::json!({"data":{"exited":1,"exitcode":if variant=="exit2"{2}else{0},"out-data":cloud.to_string()}});
        if variant == "process_pending" {
            status = serde_json::json!({"data":{"exited":0}});
        }
        if variant == "truncated" {
            status["data"]["out-truncated"] = serde_json::json!(1);
        }
        if variant == "stderr" {
            status["data"]["err-data"] = serde_json::json!("PRIVATE_GUEST_OUTPUT");
        }
        let status = status.to_string();
        let mut network = guest_network();
        if variant == "wrong_mac" {
            network["data"]["result"][0]["hardware-address"] =
                serde_json::json!("bc:24:11:00:00:01");
        }
        if variant == "loopback" {
            network["data"]["result"][0]["ip-addresses"][0]["ip-address"] =
                serde_json::json!("127.0.0.1");
        }
        if variant == "ambiguous_ip" {
            network["data"]["result"][0]["ip-addresses"].as_array_mut().unwrap().push(serde_json::json!({"ip-address-type":"ipv4","ip-address":"192.0.2.43","prefix":24}));
        }
        if variant == "broadcast" {
            network["data"]["result"][0]["ip-addresses"][0]["ip-address"] =
                serde_json::json!("192.0.2.255");
        }
        let network = network.to_string();
        let mut fixture = Fixture::responses(
            "200 OK",
            &[
                ("*", &config),
                ("*", &power),
                ("*", pending),
                ("*", &status),
                ("*", &network),
                ("*", &after),
                ("*", &power),
                ("*", pending),
            ],
            "",
            Duration::ZERO,
            true,
        )
        .await;
        let result = fixture
            .client()
            .observe_guest_probe(&guest_readiness_request(), 42)
            .await;
        if [
            "process_pending",
            "cloud_pending",
            "not_started",
            "broadcast",
            "wrong_mac",
            "loopback",
        ]
        .contains(&variant)
        {
            if let Ok(super::GuestProbe::Pending { process_exited }) = &result {
                assert_eq!(*process_exited, variant != "process_pending");
            }
            assert!(
                matches!(result, Ok(super::GuestProbe::Pending { .. })),
                "{variant}: {result:?}"
            );
        } else {
            let error = result.unwrap_err();
            assert!(
                !error.to_string().contains("PRIVATE_GUEST_OUTPUT"),
                "{variant}"
            );
        }
        assert!(
            std::iter::from_fn(|| fixture.requests.try_recv().ok()).all(|r| r.starts_with("GET "))
        );
    }
}
#[tokio::test]
async fn guest_probe_rejects_foreign_target_or_invalid_pid_without_command_replay() {
    for variant in ["foreign", "pid", "lost"] {
        let mut config: serde_json::Value = serde_json::from_str(&guest_guard_config()).unwrap();
        if variant == "foreign" {
            config["data"]["description"] = serde_json::json!("foreign");
        }
        let config = config.to_string();
        let mut fixture = Fixture::responses(
            "200 OK",
            &[
                ("*", &config),
                ("*", r#"{"data":{"vmid":202,"status":"running"}}"#),
                ("*", r#"{"data":[]}"#),
                (
                    "*",
                    if variant == "lost" {
                        DROP_REPLY
                    } else {
                        r#"{"data":{"pid":0}}"#
                    },
                ),
            ],
            "",
            Duration::ZERO,
            true,
        )
        .await;
        let provider = fixture.client();
        let request = guest_readiness_request();
        assert!(provider.dispatch_guest_probe(&request).await.is_err());
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert_eq!(
            requests.iter().filter(|r| r.starts_with("POST ")).count(),
            usize::from(variant != "foreign")
        );
        assert!(provider.observe_guest_probe(&request, 0).await.is_err());
        assert!(fixture.requests.try_recv().is_err());
    }
}

fn completed_start_for_guest_probe(
    provider: &Proxmox,
) -> (
    tempfile::TempDir,
    crate::store::Store,
    crate::operation::Operation,
) {
    completed_start_for_guest_probe_with_yaml(
        provider,
        include_str!("../../../examples/kernel-lab.yaml"),
    )
}
fn completed_start_for_guest_probe_with_yaml(
    provider: &Proxmox,
    yaml: &str,
) -> (
    tempfile::TempDir,
    crate::store::Store,
    crate::operation::Operation,
) {
    use crate::provisioning_intent::{
        CloudInitOutcome, CloudInitResult, SeedOutcome, StartOutcome, StartResult,
    };
    let (directory, store, operation) = completed_disk_for_cloud_init_with_yaml(provider, yaml);
    let job = store.claim_provisioning_cloud_init().unwrap().unwrap();
    store
        .begin_provisioning_cloud_init(
            &operation.id,
            &job.approval,
            &"f".repeat(40),
            "local-lvm:vm-202-disk-0",
            "virtio=BC:24:11:12:34:56,bridge=old-br,firewall=1",
        )
        .unwrap();
    let observation = |status| super::VmObservation {
        node: "pve-test".into(),
        vmid: 202,
        status,
        qmp_status: None,
    };
    let settings = super::CloudInitObservation {
        resources: super::ResourceObservation {
            cpu: 4,
            memory_mib: 8192,
            target: super::CloneObservation {
                config_digest: "f".repeat(40),
                observation: observation(super::PowerState::Stopped),
            },
        },
        primary_volume: "local-lvm:vm-202-disk-0".into(),
        disk_size_bytes: 40 * 1024_u64.pow(3),
        network: "virtio=BC:24:11:12:34:56,bridge=vmbr0,firewall=1".into(),
        guest_user: "debian".into(),
        ssh_key_count: 1,
        guest_ready: false,
    };
    store
        .finish_provisioning_cloud_init(
            &operation.id,
            CloudInitOutcome::Completed(Box::new(CloudInitResult {
                instance_id: operation.instance_id.clone().unwrap(),
                provider_scope_id: provider.scope_id(),
                plan_hash: job.approval.plan_hash.clone(),
                settings,
            })),
        )
        .unwrap();
    store.claim_provisioning_seed().unwrap().unwrap();
    let proof = || super::SeedPreflight {
        config_digest: "f".repeat(40),
        user_data_hash: "b".repeat(64),
        network_data_hash: "c".repeat(64),
        user_format: super::SeedUserFormat::Named,
        compatible: true,
        guest_ready: false,
    };
    store
        .begin_provisioning_seed(&operation.id, &job.approval, proof())
        .unwrap();
    store
        .finish_provisioning_seed(&operation.id, SeedOutcome::Completed(proof()))
        .unwrap();
    store.claim_provisioning_start().unwrap().unwrap();
    store
        .begin_provisioning_start(&operation.id, &job.approval, &"f".repeat(40))
        .unwrap();
    store
        .link_provisioning_start(
            &operation.id,
            &super::TaskReference::new(
                "pve-test",
                "UPID:pve-test:00000001:00000002:00000003:qmstart:202:fixture@pve:",
            )
            .unwrap(),
        )
        .unwrap();
    store
        .finish_provisioning_start(
            &operation.id,
            StartOutcome::Completed(StartResult {
                instance_id: operation.instance_id.clone().unwrap(),
                provider_scope_id: provider.scope_id(),
                plan_hash: job.approval.plan_hash,
                task_status: super::TaskStatus::Succeeded,
                observation: observation(super::PowerState::Running),
                guest_ready: false,
            }),
        )
        .unwrap();
    (directory, store, operation)
}
#[tokio::test]
async fn durable_readiness_executor_persists_pid_and_recovers_with_get_only() {
    use crate::{operation::Status, provisioning_executor::run_readiness};
    let config = guest_guard_config();
    let power = r#"{"data":{"vmid":202,"status":"running","qmpstatus":"running"}}"#;
    let pending = r#"{"data":[]}"#;
    let network = guest_network().to_string();
    let clean=serde_json::json!({"data":{"exited":1,"exitcode":0,"out-data":clean_cloud_status().to_string()}}).to_string();
    let mut fixture = Fixture::responses(
        "200 OK",
        &[
            ("*", &config),
            ("*", power),
            ("*", pending),
            ("*", &network),
            ("*", &config),
            ("*", power),
            ("*", pending),
            ("*", r#"{"data":{"pid":42}}"#),
            ("*", &config),
            ("*", power),
            ("*", pending),
            ("*", r#"{"data":{"exited":0}}"#),
            ("*", &config),
            ("*", power),
            ("*", pending),
            ("*", &clean),
            ("*", &network),
            ("*", &config),
            ("*", power),
            ("*", pending),
        ],
        "",
        Duration::ZERO,
        true,
    )
    .await;
    let provider = fixture.client();
    let (directory, store, operation) = completed_start_for_guest_probe(&provider);
    assert!(run_readiness(&store, Some(&provider)).await.unwrap());
    let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
    assert_eq!(requests.len(), 12);
    assert_eq!(
        requests.iter().filter(|r| r.starts_with("POST ")).count(),
        1
    );
    let connection = rusqlite::Connection::open(directory.path().join("clone.sqlite")).unwrap();
    let retained: String = connection
        .query_row(
            "SELECT journal_json FROM provisioning_seed_regeneration WHERE operation_id=?1",
            [&operation.id],
            |r| r.get(0),
        )
        .unwrap();
    let retained: serde_json::Value = serde_json::from_str(&retained).unwrap();
    assert_eq!(retained["readiness"]["pid"], 42);
    drop(store);
    let reopened = crate::store::Store::open(directory.path().join("clone.sqlite")).unwrap();
    reopened.recover_interrupted_operations().unwrap();
    connection
        .execute(
            "UPDATE operations SET next_reconcile_at=0 WHERE id=?1",
            [&operation.id],
        )
        .unwrap();
    assert!(run_readiness(&reopened, Some(&provider)).await.unwrap());
    let job = reopened.provisioning_job(&operation.id).unwrap().unwrap();
    let stage = job
        .stages
        .iter()
        .find(|s| s.kind == "wait_for_guest_readiness")
        .unwrap();
    assert_eq!(stage.status, Status::Succeeded);
    assert_eq!(job.operation.status, Status::Running); // Declared packages are not silently skipped.
    let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
    assert_eq!(requests.len(), 8);
    assert!(requests.iter().all(|r| r.starts_with("GET ")));
    assert!(requests.iter().any(|r| r.contains("exec-status?pid=42")));
    let after: String = connection
        .query_row(
            "SELECT journal_json FROM provisioning_seed_regeneration WHERE operation_id=?1",
            [&operation.id],
            |r| r.get(0),
        )
        .unwrap();
    let after: serde_json::Value = serde_json::from_str(&after).unwrap();
    assert_eq!(
        after["readiness"]["deadline"],
        retained["readiness"]["deadline"]
    );
}
#[tokio::test]
async fn durable_readiness_executor_lost_submission_or_expired_deadline_never_reexecutes() {
    use crate::{
        operation::{Failure, Status},
        provisioning_executor::run_readiness,
    };
    for variant in ["lost", "expired", "rejected"] {
        let config = guest_guard_config();
        let power = r#"{"data":{"vmid":202,"status":"running"}}"#;
        let pending = r#"{"data":[]}"#;
        let network = guest_network().to_string();
        let bad=serde_json::json!({"data":{"exited":1,"exitcode":2,"out-data":clean_cloud_status().to_string()}}).to_string();
        let mut fixture = Fixture::responses(
            "200 OK",
            &[
                ("*", &config),
                ("*", power),
                ("*", pending),
                ("*", &network),
                ("*", &config),
                ("*", power),
                ("*", pending),
                (
                    "*",
                    if variant == "rejected" {
                        r#"{"data":{"pid":42}}"#
                    } else {
                        DROP_REPLY
                    },
                ),
                ("*", &config),
                ("*", power),
                ("*", pending),
                ("*", &bad),
            ],
            "",
            Duration::ZERO,
            true,
        )
        .await;
        let provider = fixture.client();
        let (directory, store, operation) = completed_start_for_guest_probe(&provider);
        if variant != "expired" {
            assert!(run_readiness(&store, Some(&provider)).await.unwrap());
        } else {
            store.claim_provisioning_readiness().unwrap().unwrap();
        }
        let connection = rusqlite::Connection::open(directory.path().join("clone.sqlite")).unwrap();
        if variant == "expired" {
            connection.execute("UPDATE provisioning_seed_regeneration SET journal_json=json_set(journal_json,'$.readiness.startedAt',unixepoch()-301,'$.readiness.deadline',unixepoch()-1) WHERE operation_id=?1",[&operation.id]).unwrap();
        }
        drop(store);
        let reopened = crate::store::Store::open(directory.path().join("clone.sqlite")).unwrap();
        reopened.recover_interrupted_operations().unwrap();
        connection
            .execute(
                "UPDATE operations SET next_reconcile_at=0 WHERE id=?1",
                [&operation.id],
            )
            .unwrap();
        assert_eq!(
            run_readiness(&reopened, Some(&provider)).await.unwrap(),
            variant != "rejected"
        );
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert_eq!(
            requests.iter().filter(|r| r.starts_with("POST ")).count(),
            usize::from(variant != "expired")
        );
        if variant == "expired" {
            assert!(requests.is_empty());
        }
        let job = reopened.provisioning_job(&operation.id).unwrap().unwrap();
        assert_eq!(
            job.operation.status,
            if variant == "lost" {
                Status::Uncertain
            } else {
                Status::Failed
            }
        );
        if variant == "expired" {
            assert_eq!(job.operation.failure, Some(Failure::GuestReadinessTimedOut));
        }
    }
}

#[path = "tests/create_flow.rs"]
mod create_flow;

#[path = "tests/snapshots.rs"]
mod snapshots;

#[path = "tests/snapshot_execution.rs"]
mod snapshot_execution;

#[path = "tests/snapshot_restore.rs"]
mod snapshot_restore;

#[path = "tests/restore_execution.rs"]
mod restore_execution;

#[path = "tests/deletion.rs"]
mod deletion;

#[path = "tests/deletion_management.rs"]
mod deletion_management;

#[path = "tests/deletion_absence.rs"]
mod deletion_absence;

#[path = "tests/deletion_execution.rs"]
mod deletion_execution;

#[path = "tests/recreation_preflight.rs"]
mod recreation_preflight;

mod packages;
mod packages_worker;

mod provisioning_snapshot;
