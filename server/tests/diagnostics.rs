#![cfg(unix)]

use serde_json::Value;
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn start(database: &std::path::Path, port: u16) -> (Process, mpsc::Receiver<String>) {
    start_configured(database, port, None)
}

fn start_configured(
    database: &std::path::Path,
    port: u16,
    provider: Option<(&str, &std::path::Path)>,
) -> (Process, mpsc::Receiver<String>) {
    start_with_lifecycle(database, port, provider, None)
}

fn start_with_lifecycle(
    database: &std::path::Path,
    port: u16,
    provider: Option<(&str, &std::path::Path)>,
    lifecycle: Option<&str>,
) -> (Process, mpsc::Receiver<String>) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vmanyware-server"));
    command
        .env_clear()
        .env("VMANYWARE_PORT", port.to_string())
        .env("VMANYWARE_DATABASE", database)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if let Some(value) = lifecycle {
        command.env("VMANYWARE_LIFECYCLE_ENABLED", value);
    }
    if let Some((endpoint, ca)) = provider {
        command
            .env("VMANYWARE_PROXMOX_URL", endpoint)
            .env("VMANYWARE_PROXMOX_CA", ca)
            .env("VMANYWARE_PROXMOX_TOKEN_ID", "reader@pve!fixture")
            .env("VMANYWARE_PROXMOX_TOKEN_SECRET", "fixture-token-secret");
    }
    let mut child = command.spawn().unwrap();
    let stderr = child.stderr.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            if tx.send(line.unwrap()).is_err() {
                break;
            }
        }
    });
    let process = Process(child);
    let ready: Value =
        serde_json::from_str(&rx.recv_timeout(Duration::from_secs(10)).unwrap()).unwrap();
    assert_eq!(
        ready,
        serde_json::json!({"event":"server_listening", "port":port})
    );
    (process, rx)
}

fn stop(process: &mut Process, signal: &str) {
    assert!(Command::new("/bin/sh")
        // POSIX shells provide kill even when minimal images omit its executable.
        .args([
            "-c",
            "kill -s \"$1\" \"$2\"",
            "vmanyware-test-signal",
            signal.trim_start_matches('-'),
            &process.0.id().to_string(),
        ])
        .status()
        .unwrap()
        .success());
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = process.0.try_wait().unwrap() {
            assert!(
                status.success(),
                "service must gracefully exit successfully"
            );
            break;
        }
        assert!(Instant::now() < deadline, "service did not exit");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[tokio::test]
async fn binary_logs_only_safe_fields_and_releases_lock_after_sigterm() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("database-sensitive-marker.sqlite");
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let (mut process, logs) = start(&database, port);
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let mut ids = Vec::new();
    for (method, path, status, route, logged_method) in [
        (
            "GET",
            "/api/v1/health?query-secret-marker",
            200,
            "/api/v1/health",
            "GET",
        ),
        (
            "GET",
            "/path-secret-marker?query-secret-marker",
            404,
            "unmatched",
            "GET",
        ),
        (
            "GET",
            "/api/v1/definitions/name-secret-marker",
            404,
            "/api/v1/definitions/{name}",
            "GET",
        ),
        (
            "GET",
            "/api/v1/provider/nodes/node-secret-marker/vms/202/status?query-secret-marker",
            503,
            "/api/v1/provider/nodes/{node}/vms/{vmid}/status",
            "GET",
        ),
        (
            "GET",
            "/api/v1/operations",
            200,
            "/api/v1/operations",
            "GET",
        ),
        (
            "GET",
            "/api/v1/environments?query-secret-marker",
            200,
            "/api/v1/environments",
            "GET",
        ),
        (
            "GET",
            "/api/v1/environments/id-secret-marker",
            404,
            "/api/v1/environments/{id}",
            "GET",
        ),
        (
            "GET",
            "/api/v1/environments/id-secret-marker/status",
            404,
            "/api/v1/environments/{id}/status",
            "GET",
        ),
        (
            "GET",
            "/api/v1/environments/id-secret-marker/lifecycle-preflight",
            404,
            "/api/v1/environments/{id}/lifecycle-preflight",
            "GET",
        ),
        (
            "POST",
            "/api/v1/environments/id-secret-marker/refresh",
            400,
            "/api/v1/environments/{id}/refresh",
            "POST",
        ),
        (
            "POST",
            "/api/v1/environments/id-secret-marker/start",
            400,
            "/api/v1/environments/{id}/start",
            "POST",
        ),
        (
            "POST",
            "/api/v1/environments/id-secret-marker/shutdown",
            400,
            "/api/v1/environments/{id}/shutdown",
            "POST",
        ),
        (
            "POST",
            "/api/v1/environments",
            415,
            "/api/v1/environments",
            "POST",
        ),
        (
            "POST",
            "/api/v1/definitions/validate",
            422,
            "/api/v1/definitions/validate",
            "POST",
        ),
        (
            "CUSTOM-secret-marker",
            "/api/v1/health",
            405,
            "/api/v1/health",
            "OTHER",
        ),
    ] {
        let response = client
            .request(
                method.parse().unwrap(),
                format!("http://127.0.0.1:{port}{path}"),
            )
            .header("authorization", "Bearer header-secret-marker")
            .header("x-request-id", "incoming-id-secret-marker")
            .header("content-type", "application/yaml")
            .body("body-secret-marker")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), status);
        let id = response.headers()["x-request-id"]
            .to_str()
            .unwrap()
            .to_string();
        assert!(id.parse::<u64>().is_ok());
        assert!(!ids.contains(&id));
        ids.push(id.clone());
        let line = logs.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(!line.contains("secret-marker"));
        let event: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(event["event"], "request_complete");
        assert_eq!(event["requestId"], id);
        assert_eq!(event["route"], route);
        assert_eq!(event["method"], logged_method);
        assert_eq!(event["status"], status);
        assert!(event["durationMs"].is_u64());
        assert_eq!(event.as_object().unwrap().len(), 6);
    }
    stop(&mut process, "-TERM");
    assert_eq!(
        logs.recv_timeout(Duration::from_secs(5)).unwrap(),
        "{\"event\":\"server_stopped\"}"
    );
    let (mut restarted, logs) = start(&database, port);
    stop(&mut restarted, "-INT");
    assert_eq!(
        logs.recv_timeout(Duration::from_secs(5)).unwrap(),
        "{\"event\":\"server_stopped\"}"
    );
}

#[test]
fn startup_errors_do_not_echo_configuration() {
    let output = Command::new(env!("CARGO_BIN_EXE_vmanyware-server"))
        .env_clear()
        .env("VMANYWARE_PORT", "configuration-secret-marker")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stderr).unwrap(),
        serde_json::json!({
            "event": "server_failed", "reason": "invalid_port_configuration"
        })
    );
}

#[tokio::test]
async fn stalled_http_body_cannot_prevent_signal_exit_or_database_restart() {
    use std::io::{Read, Write};
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("stalled.sqlite");
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let (mut process, logs) = start(&database, port);
    let mut stalled = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    stalled
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    stalled.write_all(b"POST /api/v1/definitions HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/yaml\r\nContent-Length: 1024\r\n\r\nversion:").unwrap();
    // Verify the service is accepting real requests while the first body remains
    // unfinished. The stalled socket stays owned/open throughout shutdown.
    let response = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap()
        .get(format!("http://127.0.0.1:{port}/api/v1/health"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let health: Value =
        serde_json::from_str(&logs.recv_timeout(Duration::from_secs(2)).unwrap()).unwrap();
    assert_eq!(health["event"], "request_complete");
    let signalled_at = Instant::now();
    stop(&mut process, "-TERM");
    assert!(
        signalled_at.elapsed() >= Duration::from_secs(5),
        "unfinished request was not held through the drain period"
    );
    assert!(signalled_at.elapsed() < Duration::from_secs(8));
    assert_eq!(
        logs.recv_timeout(Duration::from_secs(1)).unwrap(),
        "{\"event\":\"server_drain_timed_out\"}"
    );
    assert_eq!(
        logs.recv_timeout(Duration::from_secs(1)).unwrap(),
        "{\"event\":\"server_stopped\"}"
    );
    let mut buffer = [0; 1];
    match stalled.read(&mut buffer) {
        Ok(0) => {}
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
            ) => {}
        result => panic!("stalled socket remained usable after exit: {result:?}"),
    }
    let (mut restarted, logs) = start(&database, port);
    let fast_stop = Instant::now();
    stop(&mut restarted, "-INT");
    assert!(
        fast_stop.elapsed() < Duration::from_secs(2),
        "idle shutdown waited for the drain deadline"
    );
    assert_eq!(
        logs.recv_timeout(Duration::from_secs(1)).unwrap(),
        "{\"event\":\"server_stopped\"}"
    );
}

/// A generated HTTPS peer; the second read waits until the service has stopped.
struct InstanceProvider {
    endpoint: String,
    ca: String,
    requests: tokio::sync::mpsc::UnboundedReceiver<()>,
    release: std::sync::Arc<tokio::sync::Notify>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for InstanceProvider {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl InstanceProvider {
    async fn new() -> Self {
        use std::sync::Arc;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio_rustls::{rustls, TlsAcceptor};
        let key = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let ca = key.cert.pem();
        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![key.cert.der().clone()],
                rustls::pki_types::PrivatePkcs8KeyDer::from(key.signing_key.serialize_der()).into(),
            )
            .unwrap();
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!(
            "https://localhost:{}",
            listener.local_addr().unwrap().port()
        );
        let (sender, requests) = tokio::sync::mpsc::unbounded_channel();
        let release = Arc::new(tokio::sync::Notify::new());
        let gate = release.clone();
        let task = tokio::spawn(async move {
            let mut connections = tokio::task::JoinSet::new();
            let mut reads = 0;
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        let (socket, _) = accepted.unwrap();
                        reads += 1;
                        let index = reads;
                        let acceptor = acceptor.clone(); let sender = sender.clone(); let gate = gate.clone();
                        connections.spawn(async move {
                            let mut stream = acceptor.accept(socket).await.unwrap();
                            let mut request = Vec::new(); let mut bytes = [0; 1024];
                            while !request.windows(4).any(|value| value == b"\r\n\r\n") {
                                let count = stream.read(&mut bytes).await.unwrap();
                                assert!(count > 0 && request.len() < 8192);
                                request.extend_from_slice(&bytes[..count]);
                            }
                            let request = String::from_utf8(request).unwrap();
                            assert!(request.starts_with("GET /api2/json/nodes/pve-test/qemu/202/status/current HTTP/1.1\r\n"));
                            assert!(request.to_ascii_lowercase().contains("authorization: pveapitoken=reader@pve!fixture=fixture-token-secret\r\n"));
                            sender.send(()).unwrap();
                            if index == 2 { gate.notified().await; }
                            let body = r#"{"data":{"vmid":202,"status":"running","qmpstatus":"paused"}}"#;
                            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                            // The second connection may already be closed by graceful service exit.
                            let _ = stream.write_all(response.as_bytes()).await;
                        });
                    },
                    completed = connections.join_next(), if !connections.is_empty() => {
                        completed.unwrap().unwrap();
                    },
                }
            }
        });
        Self {
            endpoint,
            ca,
            requests,
            release,
            task,
        }
    }
}

async fn operation_state(client: &reqwest::Client, url: &str, expected: &str) -> Value {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let response = client.get(url).send().await.unwrap();
            assert_eq!(response.status(), 200);
            let value: Value = response.json().await.unwrap();
            if value["status"] == expected {
                return value;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_binary_observes_instance_and_preserves_interruption_after_restart() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workflow-sensitive-marker.sqlite");
    let ca = directory.path().join("fixture-ca.pem");
    let mut provider = InstanceProvider::new().await;
    std::fs::write(&ca, &provider.ca).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let root = format!("http://127.0.0.1:{port}");
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let (mut process, logs) = start_configured(&database, port, Some((&provider.endpoint, &ca)));
    let response = client
        .post(format!("{root}/api/v1/definitions"))
        .header("content-type", "application/yaml")
        .body(include_str!("../../examples/kernel-lab.yaml"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 201);
    let definition: Value = response.json().await.unwrap();
    let response = client.post(format!("{root}/api/v1/environments")).json(&serde_json::json!({
        "definitionName":"kernel-lab","definitionHash":definition["hash"],"node":"pve-test","vmid":202,
    })).send().await.unwrap();
    assert_eq!(response.status(), 201);
    let instance: Value = response.json().await.unwrap();
    let id = instance["id"].as_str().unwrap();
    assert!(provider.requests.try_recv().is_err());
    let refresh = format!("{root}/api/v1/environments/{id}/refresh");
    let response = client
        .post(&refresh)
        .header("idempotency-key", "completed-read")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 202);
    let location = response.headers()["location"].to_str().unwrap().to_owned();
    let completed = operation_state(&client, &format!("{root}{location}"), "succeeded").await;
    assert_eq!(completed["result"]["instanceId"], id);
    assert_eq!(completed["result"]["observation"]["qmpStatus"], "paused");
    provider.requests.recv().await.unwrap();
    let retry: Value = client
        .post(&refresh)
        .header("idempotency-key", "completed-read")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(retry["id"], completed["id"]);
    assert_eq!(retry["status"], "succeeded");
    assert!(provider.requests.try_recv().is_err());
    let response = client
        .post(&refresh)
        .header("idempotency-key", "interrupted-read")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 202);
    let interrupted_path = response.headers()["location"].to_str().unwrap().to_owned();
    tokio::time::timeout(Duration::from_secs(5), provider.requests.recv())
        .await
        .unwrap()
        .unwrap();
    operation_state(&client, &format!("{root}{interrupted_path}"), "running").await;
    stop(&mut process, "-TERM");
    provider.release.notify_one();
    assert!(logs
        .try_iter()
        .all(|line| !line.contains("fixture-token-secret") && !line.contains("sensitive-marker")));
    let (mut restarted, logs) = start_configured(&database, port, Some((&provider.endpoint, &ca)));
    let recovered =
        operation_state(&client, &format!("{root}{interrupted_path}"), "interrupted").await;
    assert_eq!(recovered["failure"], "process_interrupted");
    assert!(recovered["result"].is_null());
    let retry: Value = client
        .post(&refresh)
        .header("idempotency-key", "interrupted-read")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(retry["id"], recovered["id"]);
    assert_eq!(retry["status"], "interrupted");
    assert!(provider.requests.try_recv().is_err());
    let retained: Value = client
        .get(format!("{root}/api/v1/environments/{id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(retained, instance);
    stop(&mut restarted, "-INT");
    assert!(logs
        .try_iter()
        .all(|line| !line.contains("fixture-token-secret") && !line.contains("sensitive-marker")));
}

#[test]
fn binary_rejects_invalid_lifecycle_configuration_before_creating_database() {
    for value in ["", "TRUE", "1", " private-lifecycle-marker "] {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("not-created.sqlite");
        let output = Command::new(env!("CARGO_BIN_EXE_vmanyware-server"))
            .env_clear()
            .env("VMANYWARE_DATABASE", &database)
            .env("VMANYWARE_LIFECYCLE_ENABLED", value)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let event: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(
            event,
            serde_json::json!({"event":"server_failed", "reason":"invalid_lifecycle_configuration"})
        );
        assert!(!database.exists());
        assert!(!directory.path().join("not-created.sqlite.lock").exists());
    }
}

#[tokio::test]
async fn binary_lifecycle_configuration_reaches_worker_without_a_provider() {
    use vmanyware_server::{
        definition::Definition,
        instance::Registration,
        lifecycle::{Action, Approval},
        store::Store,
    };
    for (setting, expected) in [
        (None, "lifecycle_disabled"),
        (Some("false"), "lifecycle_disabled"),
        (Some("true"), "provider_not_configured"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("configured.sqlite");
        let store = Store::open(&database).unwrap();
        let definition = Definition::parse(include_str!("../../examples/kernel-lab.yaml")).unwrap();
        store.create(&definition).unwrap();
        let instance = store
            .register_instance(
                &Registration {
                    definition_name: definition.metadata.name.clone(),
                    definition_hash: definition.hash().unwrap(),
                    node: "pve-fixture".into(),
                    vmid: 202,
                },
                &"a".repeat(64),
            )
            .unwrap();
        let action = store
            .enqueue_lifecycle(
                "binary-action",
                &Approval {
                    action: Action::Start,
                    instance_id: instance.summary.id,
                    provider_scope_id: "a".repeat(64),
                    node: "pve-fixture".into(),
                    vmid: 202,
                    definition_hash: definition.hash().unwrap(),
                    config_digest: "b".repeat(40),
                    confirmed: true,
                },
            )
            .unwrap();
        let read = store.enqueue_inventory("binary-read").unwrap();
        drop(store);
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let (mut process, logs) = start_with_lifecycle(&database, port, None, setting);
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        for (id, failure) in [
            (&read.id, "provider_not_configured"),
            (&action.id, expected),
        ] {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                let result: Value = client
                    .get(format!("http://127.0.0.1:{port}/api/v1/operations/{id}"))
                    .send()
                    .await
                    .unwrap()
                    .json()
                    .await
                    .unwrap();
                if result["status"] == "failed" {
                    assert_eq!(result["failure"], failure);
                    break;
                }
                assert!(Instant::now() < deadline);
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }
        stop(&mut process, "-TERM");
        let persisted = Store::open(&database)
            .unwrap()
            .lifecycle_job(&action.id)
            .unwrap()
            .unwrap();
        assert!(!persisted.dispatch_started);
        assert!(persisted.provider_task.is_none());
        for log in logs.try_iter() {
            assert!(!log.contains("pve-fixture"));
            assert!(!log.contains("binary-action"));
        }
    }
}

/// Generated HTTPS lifecycle peer. A single selected reply can be held across exit.
struct LifecycleProvider {
    endpoint: String,
    ca: String,
    marker: std::sync::Arc<std::sync::Mutex<String>>,
    requests: tokio::sync::mpsc::UnboundedReceiver<String>,
    release: std::sync::Arc<tokio::sync::Notify>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for LifecycleProvider {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl LifecycleProvider {
    async fn new(hold: &'static str) -> Self {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        };
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio_rustls::{rustls, TlsAcceptor};
        let key = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let ca = key.cert.pem();
        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![key.cert.der().clone()],
                rustls::pki_types::PrivatePkcs8KeyDer::from(key.signing_key.serialize_der()).into(),
            )
            .unwrap();
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!(
            "https://localhost:{}",
            listener.local_addr().unwrap().port()
        );
        let marker = Arc::new(Mutex::new(String::new()));
        let retained_marker = marker.clone();
        let release = Arc::new(tokio::sync::Notify::new());
        let gate = release.clone();
        let (sender, requests) = tokio::sync::mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            let power = Arc::new(AtomicBool::new(false));
            let held = Arc::new(AtomicBool::new(false));
            let mut connections = tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        let (socket, _) = accepted.unwrap();
                        let acceptor = acceptor.clone(); let power = power.clone();
                        let held = held.clone(); let marker = retained_marker.clone();
                        let sender = sender.clone(); let gate = gate.clone();
                        connections.spawn(async move {
                            let mut stream = acceptor.accept(socket).await.unwrap();
                            let mut request = Vec::new(); let mut bytes = [0; 1024];
                            while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                                let n = stream.read(&mut bytes).await.unwrap();
                                assert!(n > 0 && request.len() < 8192);
                                request.extend_from_slice(&bytes[..n]);
                            }
                            let text = String::from_utf8(request).unwrap();
                            assert!(text.to_ascii_lowercase().contains("authorization: pveapitoken=reader@pve!fixture=fixture-token-secret\r\n"));
                            let first = text.lines().next().unwrap();
                            let method = first.split_whitespace().next().unwrap();
                            let path = first.split_whitespace().nth(1).unwrap();
                            let upid = "UPID:pve-test:00000001:00000002:00000003:qmstart:202:fixture@pve:";
                            let (stage, data) = if method == "POST" {
                                assert_eq!(path, "/api2/json/nodes/pve-test/qemu/202/status/start");
                                assert_eq!(text.split_once("\r\n\r\n").unwrap().1, "");
                                power.store(true, Ordering::SeqCst);
                                ("post", serde_json::json!(upid))
                            } else {
                                assert_eq!(method, "GET");
                                if path.ends_with("/config?current=1") {
                                    ("config", serde_json::json!({"digest":"b".repeat(40), "tags":*marker.lock().unwrap(), "template":0}))
                                } else if path.ends_with("/pending") {
                                    ("pending", serde_json::json!([]))
                                } else if path.contains("/tasks/") {
                                    ("task", serde_json::json!({"node":"pve-test","upid":upid,"status":"stopped","exitstatus":"OK"}))
                                } else {
                                    assert_eq!(path, "/api2/json/nodes/pve-test/qemu/202/status/current");
                                    let running = power.load(Ordering::SeqCst);
                                    ("power", serde_json::json!({"vmid":202,"status":if running {"running"} else {"stopped"},"qmpstatus":if running {Some("running")} else {None}}))
                                }
                            };
                            sender.send(format!("{method} {stage}")).unwrap();
                            if stage == hold && !held.swap(true, Ordering::SeqCst) { gate.notified().await; }
                            let body = serde_json::json!({"data":data}).to_string();
                            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                            let _ = stream.write_all(response.as_bytes()).await;
                        });
                    },
                    completed = connections.join_next(), if !connections.is_empty() => { completed.unwrap().unwrap(); },
                }
            }
        });
        Self {
            endpoint,
            ca,
            marker,
            requests,
            release,
            task,
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_binary_lifecycle_success_and_restart_never_repeat_submission() {
    for hold in ["none", "post", "task"] {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("lifecycle.sqlite");
        let ca = directory.path().join("generated-ca.pem");
        let mut provider = LifecycleProvider::new(hold).await;
        std::fs::write(&ca, &provider.ca).unwrap();
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let root = format!("http://127.0.0.1:{port}");
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        let (mut process, logs) = start_with_lifecycle(
            &database,
            port,
            Some((&provider.endpoint, &ca)),
            Some("true"),
        );
        let response = client
            .post(format!("{root}/api/v1/definitions"))
            .header("content-type", "application/yaml")
            .body(include_str!("../../examples/kernel-lab.yaml"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 201);
        let definition: Value = response.json().await.unwrap();
        let response = client.post(format!("{root}/api/v1/environments")).json(&serde_json::json!({"definitionName":"kernel-lab","definitionHash":definition["hash"],"node":"pve-test","vmid":202})).send().await.unwrap();
        assert_eq!(response.status(), 201);
        let instance: Value = response.json().await.unwrap();
        let id = instance["id"].as_str().unwrap();
        *provider.marker.lock().unwrap() = format!("vmanyware-{id}");
        let report: Value = client
            .get(format!(
                "{root}/api/v1/environments/{id}/lifecycle-preflight"
            ))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(report["executionEnabled"], true);
        assert_eq!(report["startPreconditions"], "passed");
        let approval = serde_json::json!({"action":"start","instanceId":id,"providerScopeId":report["providerScopeId"],"node":"pve-test","vmid":202,"definitionHash":definition["hash"],"configDigest":report["configDigest"],"confirmed":true});
        let url = format!("{root}/api/v1/environments/{id}/start");
        let response = client
            .post(&url)
            .header("idempotency-key", "confirmed-start")
            .json(&approval)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 202);
        let location = response.headers()["location"].to_str().unwrap().to_owned();
        let operation: Value = response.json().await.unwrap();
        let operation_url = format!("{root}{location}");
        let mut requests = Vec::new();
        if hold != "none" {
            loop {
                let request =
                    tokio::time::timeout(Duration::from_secs(5), provider.requests.recv())
                        .await
                        .unwrap()
                        .unwrap();
                let held =
                    request == format!("{} {hold}", if hold == "post" { "POST" } else { "GET" });
                requests.push(request);
                if held {
                    break;
                }
            }
            operation_state(&client, &operation_url, "running").await;
            stop(&mut process, "-TERM");
            let store = vmanyware_server::store::Store::open(&database).unwrap();
            let retained = store
                .lifecycle_job(operation["id"].as_str().unwrap())
                .unwrap()
                .unwrap();
            assert!(retained.dispatch_started);
            assert_eq!(retained.provider_task.is_some(), hold == "task");
            drop(store);
            provider.release.notify_one();
            assert!(logs
                .try_iter()
                .all(|log| !log.contains("fixture-token-secret") && !log.contains("UPID:")));
            let (restarted, restarted_logs) = start_with_lifecycle(
                &database,
                port,
                Some((&provider.endpoint, &ca)),
                Some(if hold == "task" { "false" } else { "true" }),
            );
            process = restarted;
            let result = operation_state(
                &client,
                &operation_url,
                if hold == "post" {
                    "uncertain"
                } else {
                    "succeeded"
                },
            )
            .await;
            if hold == "post" {
                assert_eq!(result["failure"], "provider_submission_unconfirmed");
                assert!(result["result"].is_null());
                let conflict = client
                    .post(&url)
                    .header("idempotency-key", "fresh-key")
                    .json(&approval)
                    .send()
                    .await
                    .unwrap();
                assert_eq!(conflict.status(), 409);
                let retry: Value = client
                    .post(&url)
                    .header("idempotency-key", "confirmed-start")
                    .json(&approval)
                    .send()
                    .await
                    .unwrap()
                    .json()
                    .await
                    .unwrap();
                assert_eq!(retry["id"], operation["id"]);
                assert_eq!(retry["status"], "uncertain");
                // Observe more than two idle worker intervals, rather than exiting
                // immediately after the recovery response and missing a replay.
                assert!(
                    tokio::time::timeout(Duration::from_millis(600), provider.requests.recv())
                        .await
                        .is_err()
                );
            }
            stop(&mut process, "-INT");
            assert!(restarted_logs
                .try_iter()
                .all(|log| !log.contains("fixture-token-secret") && !log.contains("UPID:")));
        } else {
            let result = operation_state(&client, &operation_url, "succeeded").await;
            assert_eq!(result["result"]["taskStatus"], "succeeded");
            assert_eq!(result["result"]["observation"]["status"], "running");
            let duplicate: Value = client
                .post(&url)
                .header("idempotency-key", "confirmed-start")
                .json(&approval)
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            assert_eq!(duplicate["id"], operation["id"]);
            assert_eq!(duplicate["status"], "succeeded");
            stop(&mut process, "-INT");
            assert!(logs
                .try_iter()
                .all(|log| !log.contains("fixture-token-secret") && !log.contains("UPID:")));
        }
        while let Ok(request) = provider.requests.try_recv() {
            requests.push(request);
        }
        assert_eq!(
            requests.iter().filter(|r| r.starts_with("POST ")).count(),
            1,
            "{hold}: {requests:?}"
        );
        assert_eq!(
            requests.iter().filter(|r| r.as_str() == "GET task").count(),
            match hold {
                "post" => 0,
                "task" => 2,
                _ => 1,
            }
        );
        let store = vmanyware_server::store::Store::open(&database).unwrap();
        assert_eq!(
            serde_json::to_value(store.instance(id).unwrap().unwrap()).unwrap(),
            instance
        );
    }
}
