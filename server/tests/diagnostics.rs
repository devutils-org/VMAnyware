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
    let mut child = Command::new(env!("CARGO_BIN_EXE_pocketlab-server"))
        .env_clear()
        .env("POCKETLAB_PORT", port.to_string())
        .env("POCKETLAB_DATABASE", database)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
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
    assert!(Command::new("kill")
        .args([signal, &process.0.id().to_string()])
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
    let output = Command::new(env!("CARGO_BIN_EXE_pocketlab-server"))
        .env_clear()
        .env("POCKETLAB_PORT", "configuration-secret-marker")
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
