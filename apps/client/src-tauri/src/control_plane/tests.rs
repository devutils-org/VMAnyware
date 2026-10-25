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

pub(super) fn response(
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
        "definitionName":"kernel-lab", "definitionHash":"definition-hash", "planHash":"plan-hash", "provider":"proxmox", "providerScopeId":"c".repeat(64),
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
    assert_eq!(
        serde_json::to_value(&result).unwrap()["providerScopeId"],
        "c".repeat(64)
    );
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
    assert!(error.contains("Proxmox provider") && error.contains("VMANYWARE_TEMPLATES"));
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

#[test]
fn history_reads_typed_summaries_without_inventory_payloads() {
    let body = serde_json::json!([{
        "id": "a".repeat(32), "kind": "inventory_refresh", "status": "interrupted",
        "failure": "process_interrupted", "createdAt": "2026-10-18", "updatedAt": "2026-10-18"
    }]);
    let (port, task) = response("GET", "/api/v1/operations", "200 OK", &body.to_string());
    let summaries = tauri::async_runtime::block_on(super::list_operations(port)).unwrap();
    assert_eq!(summaries.len(), 1);
    assert!(matches!(summaries[0].status, OperationStatus::Interrupted));
    assert_eq!(summaries[0].failure.as_deref(), Some("process_interrupted"));
    task.join().unwrap();
}

#[test]
fn instance_observation_commands_bind_operation_and_target_identity() {
    let id = "a".repeat(32);
    let operation_id = "b".repeat(32);
    let operation = serde_json::json!({"id":operation_id,"kind":"instance_observe","instanceId":id,"status":"queued","result":null,"failure":null,"createdAt":"2026-10-18","updatedAt":"2026-10-18"});
    let (port, task) = response(
        "POST",
        &format!("/api/v1/environments/{id}/refresh"),
        "202 Accepted",
        &operation.to_string(),
    );
    let queued = tauri::async_runtime::block_on(super::begin_instance_refresh(
        port,
        id.clone(),
        "instance-request".into(),
    ))
    .unwrap();
    assert_eq!(queued.instance_id, id);
    assert!(task
        .join()
        .unwrap()
        .to_ascii_lowercase()
        .contains("idempotency-key: instance-request\r\n"));
    let mut unrelated = operation.clone();
    unrelated["instanceId"] = serde_json::json!("c".repeat(32));
    let (port, task) = response(
        "GET",
        &format!("/api/v1/operations/{operation_id}"),
        "200 OK",
        &unrelated.to_string(),
    );
    assert!(
        tauri::async_runtime::block_on(super::read_instance_operation(
            port,
            operation_id.clone(),
            id.clone()
        ))
        .is_err()
    );
    task.join().unwrap();
    assert!(
        tauri::async_runtime::block_on(super::begin_instance_refresh(
            1,
            "../escape".into(),
            "valid".into()
        ))
        .is_err()
    );
    assert!(
        tauri::async_runtime::block_on(super::begin_instance_refresh(1, id, "bad key".into()))
            .is_err()
    );
}

#[test]
fn lifecycle_command_reads_fixed_path_accepts_boolean_capability_and_rejects_unrelated_reports() {
    let id = "a".repeat(32);
    let report = serde_json::json!({"instanceId":id,"providerScopeId":"b".repeat(64),"definitionHash":"c".repeat(64),
        "expectedMarker":format!("vmanyware-{id}"),"configDigest":"d".repeat(40),
        "observation":{"node":"pve-test","vmid":202,"status":"stopped","qmpStatus":null},
        "checks":[{"code":"instance_marker","status":"failed","message":"Fixed check"},{"code":"not_template","status":"passed","message":"Fixed check"},{"code":"unlocked","status":"passed","message":"Fixed check"},{"code":"no_pending_changes","status":"passed","message":"Fixed check"},{"code":"configuration_unchanged","status":"passed","message":"Fixed check"}],"startPreconditions":"failed","shutdownPreconditions":"unknown","executionEnabled":false});
    let path = format!("/api/v1/environments/{id}/lifecycle-preflight");
    let (port, task) = response("GET", &path, "200 OK", &report.to_string());
    let checked =
        tauri::async_runtime::block_on(super::check_instance_lifecycle(port, id.clone())).unwrap();
    assert!(!checked.execution_enabled);
    let mut enabled = report.clone();
    enabled["executionEnabled"] = serde_json::json!(true);
    let (enabled_port, enabled_task) = response("GET", &path, "200 OK", &enabled.to_string());
    assert!(
        tauri::async_runtime::block_on(super::check_instance_lifecycle(enabled_port, id.clone()))
            .unwrap()
            .execution_enabled
    );
    enabled_task.join().unwrap();
    assert_eq!(checked.instance_id, id);
    task.join().unwrap();
    for (field, value) in [
        ("instanceId", serde_json::json!("f".repeat(32))),
        ("executionEnabled", serde_json::json!("true")),
    ] {
        let mut unrelated = report.clone();
        unrelated[field] = value;
        let (port, task) = response("GET", &path, "200 OK", &unrelated.to_string());
        assert!(
            tauri::async_runtime::block_on(super::check_instance_lifecycle(port, id.clone()))
                .is_err()
        );
        task.join().unwrap();
    }
    assert!(
        tauri::async_runtime::block_on(super::check_instance_lifecycle(1, "../escape".into()))
            .is_err()
    );
}

fn lifecycle_approval(action: &str) -> serde_json::Value {
    serde_json::json!({"action":action,"instanceId":"a".repeat(32),"providerScopeId":"b".repeat(64),
        "node":"pve-test","vmid":202,"definitionHash":"c".repeat(64),"configDigest":"d".repeat(40),"confirmed":true})
}
fn lifecycle_job(action: &str, succeeded: bool) -> serde_json::Value {
    serde_json::json!({"id":"e".repeat(32),"kind":if action == "start" {"vm_start"} else {"vm_shutdown"},
        "instanceId":"a".repeat(32),"status":if succeeded {"succeeded"} else {"uncertain"},
        "providerTaskKnown":succeeded,"failure":if succeeded {None} else {Some("provider_submission_unconfirmed")},
        "result":if succeeded {serde_json::json!({"action":action,"instanceId":"a".repeat(32),"providerScopeId":"b".repeat(64),
            "taskStatus":"succeeded","observation":{"node":"pve-test","vmid":202,"status":if action == "start" {"running"} else {"stopped"},"qmpStatus":null}})} else {serde_json::Value::Null},
        "createdAt":"2026-10-18","updatedAt":"2026-10-18"})
}

#[test]
fn native_lifecycle_posts_fixed_action_full_approval_and_same_request_key() {
    for action in ["start", "shutdown"] {
        let approval = lifecycle_approval(action);
        let mut queued = lifecycle_job(action, false);
        queued["status"] = serde_json::json!("queued");
        queued["failure"] = serde_json::Value::Null;
        let path = format!("/api/v1/environments/{}/{action}", "a".repeat(32));
        let (port, task) = response("POST", &path, "202 Accepted", &queued.to_string());
        let operation = tauri::async_runtime::block_on(super::begin_lifecycle_action(
            port,
            serde_json::from_value(approval.clone()).unwrap(),
            "confirmed-key".into(),
        ))
        .unwrap();
        assert_eq!(operation.status, OperationStatus::Queued);
        let request = task.join().unwrap();
        assert!(request
            .to_ascii_lowercase()
            .contains("idempotency-key: confirmed-key\r\n"));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(request.split_once("\r\n\r\n").unwrap().1)
                .unwrap(),
            approval
        );
        assert!(!request.to_ascii_lowercase().contains("origin:"));
    }
}

#[test]
fn native_lifecycle_get_resumes_uncertain_and_verifies_separate_task_power_evidence() {
    let id = "e".repeat(32);
    for (action, succeeded) in [("start", false), ("start", true), ("shutdown", true)] {
        let body = lifecycle_job(action, succeeded);
        let (port, task) = response(
            "GET",
            &format!("/api/v1/operations/{id}"),
            "200 OK",
            &body.to_string(),
        );
        let operation = tauri::async_runtime::block_on(super::read_lifecycle_operation(
            port,
            id.clone(),
            serde_json::from_value(lifecycle_approval(action)).unwrap(),
        ))
        .unwrap();
        assert_eq!(
            operation.status,
            if succeeded {
                OperationStatus::Succeeded
            } else {
                OperationStatus::Uncertain
            }
        );
        let request = task.join().unwrap();
        assert!(!request.to_ascii_lowercase().contains("idempotency-key:"));
    }
    for (field, value) in [
        ("id", serde_json::json!("f".repeat(32))),
        ("kind", serde_json::json!("vm_shutdown")),
        ("instanceId", serde_json::json!("f".repeat(32))),
        ("providerTaskKnown", serde_json::json!(false)),
        ("failure", serde_json::json!("private message")),
        ("status", serde_json::json!("running")),
    ] {
        let mut body = lifecycle_job("start", true);
        body[field] = value;
        let (port, task) = response(
            "GET",
            &format!("/api/v1/operations/{id}"),
            "200 OK",
            &body.to_string(),
        );
        assert!(
            tauri::async_runtime::block_on(super::read_lifecycle_operation(
                port,
                id.clone(),
                serde_json::from_value(lifecycle_approval("start")).unwrap()
            ))
            .is_err()
        );
        task.join().unwrap();
    }
    for (field, value) in [
        ("providerScopeId", serde_json::json!("f".repeat(64))),
        ("action", serde_json::json!("shutdown")),
        ("taskStatus", serde_json::json!("running")),
        (
            "observation",
            serde_json::json!({"node":"pve-test","vmid":203,"status":"running","qmpStatus":null}),
        ),
        (
            "observation",
            serde_json::json!({"node":"pve-test","vmid":202,"status":"stopped","qmpStatus":null}),
        ),
    ] {
        let mut body = lifecycle_job("start", true);
        body["result"][field] = value;
        let (port, task) = response(
            "GET",
            &format!("/api/v1/operations/{id}"),
            "200 OK",
            &body.to_string(),
        );
        assert!(
            tauri::async_runtime::block_on(super::read_lifecycle_operation(
                port,
                id.clone(),
                serde_json::from_value(lifecycle_approval("start")).unwrap()
            ))
            .is_err()
        );
        task.join().unwrap();
    }
}

#[test]
fn native_lifecycle_rejects_invalid_confirmation_before_transport_and_owns_error_messages() {
    for (field, value) in [
        ("confirmed", serde_json::json!(false)),
        ("node", serde_json::json!("../escape")),
        ("vmid", serde_json::json!(99)),
        ("configDigest", serde_json::json!("bad")),
        ("providerScopeId", serde_json::json!("F".repeat(64))),
    ] {
        let mut approval = lifecycle_approval("start");
        approval[field] = value;
        let error = tauri::async_runtime::block_on(super::begin_lifecycle_action(
            1,
            serde_json::from_value(approval).unwrap(),
            "key".into(),
        ))
        .err()
        .unwrap();
        assert_eq!(error, "Invalid lifecycle confirmation or request key.");
    }
    let mut extra = lifecycle_approval("start");
    extra["forceStop"] = serde_json::json!(true);
    assert!(serde_json::from_value::<super::LifecycleApproval>(extra).is_err());
    for (code, expected) in [
        ("lifecycle_disabled", "disabled"),
        ("invalid_lifecycle_approval", "complete target"),
        ("lifecycle_target_changed", "changed"),
        ("operation_conflict", "conflicts"),
    ] {
        let body = serde_json::json!({"error":{"code":code,"message":"private provider message"}});
        let (port, task) = response(
            "POST",
            &format!("/api/v1/environments/{}/start", "a".repeat(32)),
            "403 Forbidden",
            &body.to_string(),
        );
        let error = tauri::async_runtime::block_on(super::begin_lifecycle_action(
            port,
            serde_json::from_value(lifecycle_approval("start")).unwrap(),
            "key".into(),
        ))
        .err()
        .unwrap();
        assert!(error.contains(expected));
        assert!(!error.contains("private"));
        task.join().unwrap();
    }
}

#[test]
fn retained_instance_list_reads_deleted_and_registered_associations() {
    let entries = ["registered", "deleted"].map(|status| serde_json::json!({
        "id":if status=="deleted"{"b".repeat(32)}else{"a".repeat(32)},
        "definitionName":"retained-definition", "definitionHash":"c".repeat(64),
        "definitionRevision":1,"createdAt":"2026-10-20T12:00:00Z",
        "providerReference":{"provider":"proxmox","scopeId":"d".repeat(64),"node":"fixture-node","vmid":202},"status":status
    }));
    let (port, task) = response(
        "GET",
        "/api/v1/environments",
        "200 OK",
        &serde_json::to_string(&entries).unwrap(),
    );
    let retained = tauri::async_runtime::block_on(super::list_instances(port)).unwrap();
    let decoded = serde_json::to_value(retained).unwrap();
    assert_eq!(decoded[0]["status"], "registered");
    assert_eq!(decoded[1]["status"], "deleted");
    assert_eq!(decoded[1]["definitionName"], "retained-definition");
    assert!(task
        .join()
        .unwrap()
        .starts_with("GET /api/v1/environments "));
}
