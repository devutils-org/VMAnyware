//! Confirmed loopback creation, sharing the remote plan/readiness validation contract.
use super::{request, Payload};
use crate::remote_connection::create::{validate_operation, Operation, Plan};

fn checked(plan: &Plan) -> Result<(), String> {
    if !plan.valid(plan.target_vmid()) {
        return Err("Invalid creation plan. Preview the exact target again.".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn begin_local_creation(
    port: u16,
    plan: Plan,
    request_key: String,
    confirmed: bool,
) -> Result<Operation, String> {
    checked(&plan)?;
    if !confirmed
        || !plan.enabled()
        || request_key.is_empty()
        || request_key.len() > 64
        || !request_key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err("Confirm the enabled plan and retain a valid request key.".into());
    }
    // The server independently recomputes and binds this approval to the saved definition.
    let operation: Operation = request(
        port,
        "/api/v1/provisioning/create",
        Some(Payload::ConfirmedJson(request_key, plan.approval())),
    )
    .await?;
    validate_operation(&operation, &plan)?;
    Ok(operation)
}

#[tauri::command]
pub async fn read_local_creation(port: u16, id: String, plan: Plan) -> Result<Operation, String> {
    checked(&plan)?;
    if id.len() != 32
        || !id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("Invalid creation operation identifier.".into());
    }
    let operation: Operation = request(port, &format!("/api/v1/operations/{id}"), None).await?;
    if operation.id() != id {
        return Err("The control plane returned an unrelated creation operation.".into());
    }
    validate_operation(&operation, &plan)?;
    Ok(operation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control_plane::tests::response;
    use serde_json::{json, Value};
    fn plan() -> Value {
        json!({"definitionName":"home-smoke","definitionHash":"a".repeat(64),
            "planHash":"b".repeat(64),"providerScopeId":"c".repeat(64),
            "source":{"templateVmid":9000,"templateVersion":"fixture-v1"},
            "target":{"node":"pve-test","vmid":9001,"storage":"local-lvm","bridge":"vmbr0",
            "guestUser":"debian","resources":{"cpu":2,"memoryMiB":2048,"diskGiB":12}},
            "creationEnabled":true,"sshKeyCount":1,"packages":[],"snapshotAfterProvisioning":false})
    }
    fn queued() -> Value {
        json!({"id":"d".repeat(32),"kind":"vm_provision","instanceId":"e".repeat(32),
            "status":"queued","result":null,"failure":null})
    }
    #[test]
    fn local_creation_posts_exact_approval_and_retained_key() {
        let (port, task) = response(
            "POST",
            "/api/v1/provisioning/create",
            "202 Accepted",
            &queued().to_string(),
        );
        tauri::async_runtime::block_on(begin_local_creation(
            port,
            serde_json::from_value(plan()).unwrap(),
            "same-key".into(),
            true,
        ))
        .unwrap();
        let request = task.join().unwrap();
        assert!(request
            .to_ascii_lowercase()
            .contains("idempotency-key: same-key"));
        let body: Value = serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        assert_eq!(
            body,
            json!({"definitionName":"home-smoke","definitionHash":"a".repeat(64),
            "planHash":"b".repeat(64),"providerScopeId":"c".repeat(64),"targetNode":"pve-test",
            "targetVmid":9001,"sourceVmid":9000,"storage":"local-lvm","bridge":"vmbr0","confirmed":true})
        );
    }
    #[test]
    fn local_creation_refuses_unconfirmed_disabled_invalid_inputs_before_transport() {
        for variant in ["unconfirmed", "disabled", "scope", "key"] {
            let mut value = plan();
            if variant == "disabled" {
                value["creationEnabled"] = json!(false);
            }
            if variant == "scope" {
                value["providerScopeId"] = json!("invalid");
            }
            let result = tauri::async_runtime::block_on(begin_local_creation(
                0,
                serde_json::from_value(value).unwrap(),
                if variant == "key" { "bad key" } else { "key" }.into(),
                variant != "unconfirmed",
            ));
            let error = result.err().unwrap();
            assert!(!error.contains("port"));
        }
    }
    #[test]
    fn local_creation_status_uses_exact_get_and_rejects_unrelated_results() {
        for wrong in [false, true] {
            let id = "d".repeat(32);
            let mut operation = queued();
            if wrong {
                operation["id"] = json!("f".repeat(32));
            }
            let (port, task) = response(
                "GET",
                &format!("/api/v1/operations/{id}"),
                "200 OK",
                &operation.to_string(),
            );
            assert_eq!(
                tauri::async_runtime::block_on(read_local_creation(
                    port,
                    id,
                    serde_json::from_value(plan()).unwrap()
                ))
                .is_ok(),
                !wrong
            );
            task.join().unwrap();
        }
        let mut operation = queued();
        operation["kind"] = json!("vm_start");
        let (port, task) = response(
            "POST",
            "/api/v1/provisioning/create",
            "202 Accepted",
            &operation.to_string(),
        );
        assert!(tauri::async_runtime::block_on(begin_local_creation(
            port,
            serde_json::from_value(plan()).unwrap(),
            "key".into(),
            true
        ))
        .is_err());
        task.join().unwrap();
    }
}
