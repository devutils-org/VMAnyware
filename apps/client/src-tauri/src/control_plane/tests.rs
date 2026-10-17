use super::{
    begin_inventory_refresh, inspect_control_plane, list_definitions, preflight_provisioning,
    preview_provisioning, read_operation, save_definition, OperationStatus, PreflightStatus,
};
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
fn inventory_jobs_send_idempotency_key_and_poll_by_validated_id() {
    let id = "a".repeat(32);
    let job = serde_json::json!({"id":id,"kind":"inventory_refresh","status":"queued","result":null,"failure":null,"createdAt":"2026-10-16","updatedAt":"2026-10-16"});
    let (port, task) = response(
        "POST",
        "/api/v1/provider/inventory/refresh",
        "202 Accepted",
        &job.to_string(),
    );
    let queued =
        tauri::async_runtime::block_on(begin_inventory_refresh(port, "test-request-key".into()))
            .unwrap();
    assert!(matches!(queued.status, OperationStatus::Queued));
    assert!(task
        .join()
        .unwrap()
        .to_ascii_lowercase()
        .contains("idempotency-key: test-request-key\r\n"));
    let (port, task) = response(
        "GET",
        &format!("/api/v1/operations/{id}"),
        "200 OK",
        &job.to_string(),
    );
    assert_eq!(
        tauri::async_runtime::block_on(read_operation(port, id))
            .unwrap()
            .id,
        queued.id
    );
    task.join().unwrap();
    assert!(tauri::async_runtime::block_on(read_operation(port, "../escape".into())).is_err());
    assert!(
        tauri::async_runtime::block_on(begin_inventory_refresh(port, "bad key".into())).is_err()
    );
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

#[test]
fn provisioning_preview_posts_explicit_target_and_decodes_offline_plan() {
    let sample = include_str!("../../../../../examples/kernel-lab.yaml");
    let plan = serde_json::json!({
        "definitionName":"kernel-lab", "definitionHash":"definition-hash", "planHash":"plan-hash", "provider":"proxmox",
        "source":{"image":"debian-13","node":"pve-test","templateVmid":9000,"templateVersion":"debian-13-cloud-20261016","storage":"local-lvm","bridge":"vmbr0","diskDevice":"scsi0","sourceDiskGiB":8,"guestUser":"debian"},
        "target":{"node":"pve-test","vmid":201,"name":"kernel-lab","storage":"local-lvm","bridge":"vmbr0","resources":{"cpu":4,"memoryMiB":8192,"diskGiB":40},"guestUser":"debian","ipv4":"dhcp"},
        "sshKeyCount":1,"packages":["git"],"snapshotAfterProvisioning":false,"steps":["full_clone"],
        "liveVerified":false,"vmidReserved":false,"creationEnabled":false
    });
    let (port, task) = response(
        "POST",
        "/api/v1/provisioning/plan",
        "200 OK",
        &plan.to_string(),
    );
    let result =
        tauri::async_runtime::block_on(preview_provisioning(port, sample.into(), 201)).unwrap();
    assert_eq!(result.target.vmid, 201);
    assert!(!result.live_verified && !result.vmid_reserved && !result.creation_enabled);
    let request = task.join().unwrap();
    assert!(request
        .to_ascii_lowercase()
        .contains("content-type: application/json\r\n"));
    let payload: serde_json::Value =
        serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(payload["yaml"], sample);
    assert_eq!(payload["targetVmid"], 201);
}

#[test]
fn preview_validation_and_provider_error_codes_return_safe_actionable_messages() {
    for (code, text) in [
        ("image_not_mapped", "No template is mapped"),
        ("invalid_target_vmid", "different from the source"),
        ("disk_shrink_unsupported", "smaller than the source"),
    ] {
        let body = serde_json::json!({"error":{"code":code,"message":"must-not-echo"}});
        let (port, task) = response(
            "POST",
            "/api/v1/provisioning/plan",
            "422 Unprocessable Entity",
            &body.to_string(),
        );
        let message =
            tauri::async_runtime::block_on(preview_provisioning(port, "yaml".into(), 201))
                .err()
                .unwrap();
        assert!(message.contains(text));
        assert!(!message.contains("must-not-echo"));
        task.join().unwrap();
    }
    assert!(tauri::async_runtime::block_on(preview_provisioning(1, "yaml".into(), 99)).is_err());
    assert!(
        tauri::async_runtime::block_on(preview_provisioning(1, "x".repeat(65537), 201)).is_err()
    );
}

#[test]
fn preflight_posts_target_and_preserves_unknown_checks_without_enabling_creation() {
    let body = serde_json::json!({"planHash":"plan-hash", "checks":[
        {"code":"template", "status":"passed", "message":"Template is available."},
        {"code":"storage", "status":"failed", "message":"Storage is inactive."},
        {"code":"vmid", "status":"unknown", "message":"VMID availability could not be established."}
    ], "creationEnabled":false, "vmidReserved":false});
    let (port, task) = response(
        "POST",
        "/api/v1/provisioning/preflight",
        "200 OK",
        &body.to_string(),
    );
    let result =
        tauri::async_runtime::block_on(preflight_provisioning(port, "definition-yaml".into(), 201))
            .unwrap();
    assert_eq!(result.plan_hash, "plan-hash");
    assert!(matches!(result.checks[0].status, PreflightStatus::Passed));
    assert!(matches!(result.checks[1].status, PreflightStatus::Failed));
    assert!(matches!(result.checks[2].status, PreflightStatus::Unknown));
    assert!(!result.creation_enabled && !result.vmid_reserved);
    let request = task.join().unwrap();
    let payload: serde_json::Value =
        serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(
        payload,
        serde_json::json!({"yaml":"definition-yaml", "targetVmid":201})
    );
}

#[test]
fn preflight_bounds_and_configuration_errors_are_safe() {
    assert!(tauri::async_runtime::block_on(preflight_provisioning(1, "yaml".into(), 99)).is_err());
    assert!(
        tauri::async_runtime::block_on(preflight_provisioning(1, "x".repeat(65537), 201)).is_err()
    );
    assert!(tauri::async_runtime::block_on(preflight_provisioning(0, "yaml".into(), 201)).is_err());
    let (port, task) = response(
        "POST",
        "/api/v1/provisioning/preflight",
        "503 Service Unavailable",
        "private-provider-body",
    );
    let error = tauri::async_runtime::block_on(preflight_provisioning(port, "yaml".into(), 201))
        .err()
        .unwrap();
    task.join().unwrap();
    assert!(error.contains("Proxmox provider") && error.contains("POCKETLAB_TEMPLATES"));
    assert!(!error.contains("private-provider-body"));
}

#[test]
fn validation_uses_dedicated_endpoint_and_returns_canonical_definition() {
    let body = serde_json::json!({"name":"test-definition","hash":"canonical-hash","yaml":"canonical-yaml"});
    let (port, task) = response(
        "POST",
        "/api/v1/definitions/validate",
        "200 OK",
        &body.to_string(),
    );
    let validated =
        tauri::async_runtime::block_on(super::validate_definition(port, "input-yaml".into()))
            .unwrap();
    assert_eq!(validated.name, "test-definition");
    assert_eq!(validated.hash, "canonical-hash");
    assert_eq!(validated.yaml, "canonical-yaml");
    let request = task.join().unwrap();
    assert!(request
        .to_ascii_lowercase()
        .contains("content-type: application/yaml\r\n"));
    assert_eq!(request.split_once("\r\n\r\n").unwrap().1, "input-yaml");
}

#[test]
fn validation_limits_and_untrusted_error_bodies_are_safe() {
    assert!(
        tauri::async_runtime::block_on(super::validate_definition(1, "x".repeat(65537))).is_err()
    );
    let (port, task) = response(
        "POST",
        "/api/v1/definitions/validate",
        "422 Unprocessable Entity",
        "private-response-data",
    );
    let error = tauri::async_runtime::block_on(super::validate_definition(port, "yaml".into()))
        .err()
        .unwrap();
    task.join().unwrap();
    assert!(error.contains("Definition is invalid"));
    assert!(!error.contains("private-response-data"));
}

#[test]
fn definition_errors_select_only_known_field_messages() {
    for (field, expected) in [
        (
            "spec.resources.cpu",
            "spec.resources.cpu: choose 1–64 CPUs.",
        ),
        (
            "unknown-secret-field",
            "Definition is invalid. Check the v1 schema, required fields and public SSH keys.",
        ),
    ] {
        let body = serde_json::json!({"error":{"code":"invalid_definition","field":field,"message":"secret-provider-message"}});
        let (port, task) = response(
            "POST",
            "/api/v1/definitions/validate",
            "422 Unprocessable Entity",
            &body.to_string(),
        );
        let error = tauri::async_runtime::block_on(super::validate_definition(port, "yaml".into()))
            .err()
            .unwrap();
        task.join().unwrap();
        assert_eq!(error, expected);
        assert!(!error.contains("secret"));
    }
}

#[test]
fn service_health_is_independent_of_inventory_and_requires_ok_status() {
    for (body, healthy) in [
        (r#"{"status":"ok"}"#, true),
        (r#"{"status":"degraded"}"#, false),
        (r#"{"unexpected":"ok"}"#, false),
    ] {
        let (port, task) = response("GET", "/api/v1/health", "200 OK", body);
        assert_eq!(
            tauri::async_runtime::block_on(super::check_control_plane(port)).is_ok(),
            healthy
        );
        task.join().unwrap();
    }
    assert!(tauri::async_runtime::block_on(super::check_control_plane(0)).is_err());
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let error = tauri::async_runtime::block_on(super::check_control_plane(port)).unwrap_err();
    assert!(error.contains("Cannot reach the local control plane"));
}
