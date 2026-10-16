use super::{inspect_control_plane, list_definitions, save_definition};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::Duration,
};

fn response(
    method: &str,
    path: &str,
    status: &str,
    body: &str,
) -> (u16, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let expected = format!("{method} {path} ");
    let payload = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let task = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let n = stream.read(&mut buffer).unwrap();
            assert!(n > 0 && request.len() < 100_000);
            request.extend_from_slice(&buffer[..n]);
            if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
                let length: usize = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length: "))
                    .unwrap_or("0")
                    .parse()
                    .unwrap();
                if request.len() >= end + 4 + length {
                    break;
                }
            }
        }
        let request = String::from_utf8(request).unwrap();
        assert!(request.starts_with(&expected));
        stream.write_all(payload.as_bytes()).unwrap();
        request
    });
    (port, task)
}

#[test]
fn local_inventory_round_trip() {
    let (port, task) = response(
        "GET",
        "/api/v1/provider/inventory",
        "200 OK",
        r#"{"provider":"proxmox","version":"9.0","nodes":[{"name":"pve","status":"online"}],"templates":[]}"#,
    );
    let inventory = tauri::async_runtime::block_on(inspect_control_plane(port)).unwrap();
    task.join().unwrap();
    assert_eq!(inventory.version, "9.0");
    assert_eq!(inventory.nodes[0].name, "pve");
}

#[test]
fn local_connection_rejects_invalid_port_and_safe_errors_hide_body() {
    assert!(tauri::async_runtime::block_on(inspect_control_plane(0)).is_err());
    for (status, body) in [
        ("503 Service Unavailable", "secret-provider-body"),
        ("200 OK", "invalid-json"),
    ] {
        let (port, task) = response("GET", "/api/v1/provider/inventory", status, body);
        let error = tauri::async_runtime::block_on(inspect_control_plane(port))
            .err()
            .unwrap();
        assert!(!error.contains(body));
        task.join().unwrap();
    }
}

#[test]
fn definitions_use_yaml_post_and_list_saved_records() {
    let sample = include_str!("../../../../../examples/kernel-lab.yaml");
    let record = serde_json::json!({"name":"kernel-lab","yaml":sample,"hash":"test-hash","revision":1,"createdAt":"2026-10-16","updatedAt":"2026-10-16"});
    let (port, task) = response(
        "POST",
        "/api/v1/definitions",
        "201 Created",
        &record.to_string(),
    );
    let saved = tauri::async_runtime::block_on(save_definition(port, sample.into())).unwrap();
    assert_eq!(saved.name, "kernel-lab");
    let request = task.join().unwrap();
    assert!(request
        .to_ascii_lowercase()
        .contains("content-type: application/yaml\r\n"));
    assert!(request.ends_with(sample));
    let (port, task) = response(
        "GET",
        "/api/v1/definitions",
        "200 OK",
        &format!("[{record}]"),
    );
    assert_eq!(
        tauri::async_runtime::block_on(list_definitions(port)).unwrap()[0].name,
        "kernel-lab"
    );
    task.join().unwrap();
    assert!(tauri::async_runtime::block_on(save_definition(port, "x".repeat(65537))).is_err());
}
