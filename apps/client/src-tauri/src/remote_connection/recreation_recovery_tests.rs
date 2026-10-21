use super::*;
use crate::remote_connection::{
    create::prepare_tests::sequence,
    tests::{fixture, fixture_with_gate},
    Profile,
};
use serde_json::{json, Value};
use std::sync::Arc;
const TOKEN: &str = "fixture-only-recreation-token-0123456789abcdef";
fn source() -> Value {
    json!({"id":"a".repeat(32),"definitionName":"mobile-lab","definitionHash":"b".repeat(64),"definitionRevision":1,"definitionYaml":"fixture retained YAML","status":"deleted"})
}
fn saved(status: &str) -> Value {
    json!({"sourceInstanceId":"a".repeat(32),"operation":{"id":"e".repeat(32),"kind":"vm_provision","instanceId":"f".repeat(32),"status":status,"requestKey":"PRIVATE_IGNORED_KEY","result":if status=="succeeded"{json!({"instanceId":"f".repeat(32),"providerScopeId":"d".repeat(64),"planHash":"c".repeat(64),"guestReady":true,"guest":{"ipv4":"192.0.2.42","interface":"eth0","cloudInitExitCode":0,"cloudInitClean":true}})}else{Value::Null}},"plan":{"definitionName":"mobile-lab","definitionHash":"b".repeat(64),"planHash":"c".repeat(64),"providerScopeId":"d".repeat(64),"source":{"templateVmid":9000,"templateVersion":"fixture-v1"},"target":{"node":"pve-test","vmid":202,"storage":"local-lvm","bridge":"vmbr0","guestUser":"debian","resources":{"cpu":2,"memoryMiB":2048,"diskGiB":8}},"creationEnabled":false,"sshKeyCount":1,"packages":[],"snapshotAfterProvisioning":false},"restoresPreviousDiskContents":false,"requiresNewInstanceIdentity":true,"confirmed":true})
}
fn history() -> Value {
    json!({"sourceInstanceId":"a".repeat(32),"operations":[{"id":"e".repeat(32),"instanceId":"f".repeat(32),"status":"uncertain","createdAt":"2026-10-21T00:00:00.000Z","requestKey":"PRIVATE_IGNORED_KEY"}]})
}
fn manager(endpoint: &str, ca: reqwest::Certificate) -> (Arc<RemoteConnection>, u64) {
    let manager = Arc::new(RemoteConnection::default());
    let id = manager.begin().unwrap();
    manager
        .complete(id, Profile::new(endpoint, TOKEN, Some(ca)).unwrap())
        .unwrap();
    (manager, id)
}
#[tokio::test]
async fn native_recreation_recovery_reads_fixed_gets_without_installing_consent() {
    let (endpoint, ca, task) = sequence(vec![
        ("200 OK", history().to_string()),
        ("200 OK", source().to_string()),
        ("200 OK", saved("succeeded").to_string()),
    ])
    .await;
    let (manager, id) = manager(&endpoint, ca);
    let history = manager
        .recreation_history(id, &"a".repeat(32))
        .await
        .unwrap();
    let result = manager
        .recover_recreation(id, &"a".repeat(32), &"e".repeat(32))
        .await
        .unwrap();
    assert!(!serde_json::to_string(&history)
        .unwrap()
        .contains("PRIVATE_IGNORED_KEY"));
    let json = serde_json::to_string(&result).unwrap();
    assert!(!json.contains("PRIVATE_IGNORED_KEY") && !json.contains("confirmed"));
    assert!(manager.recreation_state_is_empty());

    let requests = task.await.unwrap();
    assert_eq!(requests.len(), 3);
    assert!(requests.iter().all(|request| request.starts_with("GET ")
        && request
            .to_ascii_lowercase()
            .contains(&format!("authorization: bearer {TOKEN}"))));
    assert!(requests[0].starts_with(&format!(
        "GET /api/v1/environments/{}/recreation-operations HTTP/1.1",
        "a".repeat(32)
    )));
    assert!(requests[1].starts_with(&format!(
        "GET /api/v1/environments/{} HTTP/1.1",
        "a".repeat(32)
    )));
    assert!(requests[2].starts_with(&format!(
        "GET /api/v1/environments/{}/recreation-operations/{} HTTP/1.1",
        "a".repeat(32),
        "e".repeat(32)
    )));
}
#[tokio::test]
async fn native_recreation_recovery_preserves_nonterminal_and_failed_states_without_consent() {
    for status in ["queued", "running", "uncertain", "failed", "interrupted"] {
        let (endpoint, ca, task) = sequence(vec![
            ("200 OK", source().to_string()),
            ("200 OK", saved(status).to_string()),
        ])
        .await;
        let (manager, id) = manager(&endpoint, ca);
        let result = manager
            .recover_recreation(id, &"a".repeat(32), &"e".repeat(32))
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(result).unwrap()["operation"]["status"],
            status
        );
        assert!(manager.recreation_state_is_empty());
        task.await.unwrap();
    }
}
#[tokio::test]
async fn native_recreation_recovery_rejects_changed_source_plan_instance_and_false_success() {
    for pointer in [
        "/sourceInstanceId",
        "/operation/id",
        "/operation/kind",
        "/operation/instanceId",
        "/plan/definitionHash",
        "/plan/providerScopeId",
        "/operation/result/planHash",
        "/operation/result/guest/cloudInitClean",
        "/restoresPreviousDiskContents",
        "/requiresNewInstanceIdentity",
    ] {
        let mut value = saved("succeeded");
        let replacement = match pointer {
            "/operation/instanceId" => json!("a".repeat(32)),
            "/operation/result/guest/cloudInitClean" | "/requiresNewInstanceIdentity" => {
                json!(false)
            }
            "/restoresPreviousDiskContents" => json!(true),
            _ => json!("changed"),
        };
        *value.pointer_mut(pointer).unwrap() = replacement;
        let (endpoint, ca, task) = sequence(vec![
            ("200 OK", source().to_string()),
            ("200 OK", value.to_string()),
        ])
        .await;
        let (manager, id) = manager(&endpoint, ca);
        assert!(
            manager
                .recover_recreation(id, &"a".repeat(32), &"e".repeat(32))
                .await
                .is_err(),
            "{pointer}"
        );
        assert!(manager.recreation_state_is_empty());
        task.await.unwrap();
    }
}
#[tokio::test]
async fn native_recreation_history_rejects_wrong_source_duplicates_order_and_unknown_status() {
    for mode in 0..7 {
        let mut value = history();
        match mode {
            0 => value["sourceInstanceId"] = json!("b".repeat(32)),
            1 => {
                let entry = value["operations"][0].clone();
                value["operations"].as_array_mut().unwrap().push(entry);
            }
            2 => value["operations"][0]["status"] = json!("unsupported"),
            3 => value["operations"][0]["instanceId"] = json!("a".repeat(32)),
            4 => value["operations"][0]["createdAt"] = json!("bad\ntime"),
            5 => {
                let entry = value["operations"][0].clone();
                value["operations"] = json!(vec![entry; 101]);
            }
            _ => {
                let mut entry = value["operations"][0].clone();
                entry["id"] = json!("f".repeat(32));
                entry["instanceId"] = json!("9".repeat(32));
                value["operations"].as_array_mut().unwrap().push(entry);
            }
        }
        let (endpoint, ca, task) = sequence(vec![("200 OK", value.to_string())]).await;
        let (manager, id) = manager(&endpoint, ca);
        assert!(
            manager
                .recreation_history(id, &"a".repeat(32))
                .await
                .is_err(),
            "{mode}"
        );
        task.await.unwrap();
    }
}
#[tokio::test]
async fn native_recreation_recovery_rejects_active_source_and_invalid_ids_before_followup() {
    let mut value = source();
    value["status"] = json!("registered");
    let (endpoint, ca, task) = sequence(vec![("200 OK", value.to_string())]).await;
    let (manager, id) = manager(&endpoint, ca);
    assert!(manager.recreation_history(id, "../bad").await.is_err());
    assert!(manager
        .recover_recreation(id, &"a".repeat(32), "../bad")
        .await
        .is_err());
    assert!(manager
        .recover_recreation(id, &"a".repeat(32), &"e".repeat(32))
        .await
        .is_err());
    assert_eq!(task.await.unwrap().len(), 1);
}
#[tokio::test]
async fn native_recreation_history_requires_auth_and_redacts_errors() {
    for (status, body, headers) in [
        ("200 OK", history().to_string(), ""),
        (
            "503 Service Unavailable",
            "PRIVATE_SERVER_BODY".into(),
            "x-vmanyware-authenticated: bearer\r\n",
        ),
    ] {
        let (endpoint, ca, task) = fixture(status, &body, headers).await;
        let (manager, id) = manager(&endpoint, ca);
        let error = manager
            .recreation_history(id, &"a".repeat(32))
            .await
            .err()
            .unwrap();
        assert!(!error.contains(TOKEN) && !error.contains("PRIVATE_SERVER_BODY"));
        task.await.unwrap();
    }
}
#[tokio::test]
async fn native_recreation_recovery_discards_source_reply_after_disconnect() {
    let (seen, received) = tokio::sync::oneshot::channel();
    let (release, gate) = tokio::sync::oneshot::channel();
    let (endpoint, ca, task) = fixture_with_gate(
        "200 OK",
        &source().to_string(),
        "x-vmanyware-authenticated: bearer\r\n",
        Some((seen, gate)),
    )
    .await;
    let (manager, id) = manager(&endpoint, ca);
    let request_manager = manager.clone();
    let request = tokio::spawn(async move {
        request_manager
            .recover_recreation(id, &"a".repeat(32), &"e".repeat(32))
            .await
    });
    received.await.unwrap();
    manager.begin().unwrap();
    release.send(()).unwrap();
    assert!(request.await.unwrap().is_err());
    assert!(manager.recreation_state_is_empty());
    task.await.unwrap();
}

#[tokio::test]
async fn native_recreation_history_discards_late_response_and_accepts_explicit_empty_history() {
    let empty = json!({"sourceInstanceId":"a".repeat(32),"operations":[]});
    let (endpoint, ca, task) = sequence(vec![("200 OK", empty.to_string())]).await;
    let (connection, id) = manager(&endpoint, ca);
    assert!(connection
        .recreation_history(id, &"a".repeat(32))
        .await
        .unwrap()
        .operations
        .is_empty());
    task.await.unwrap();
    let (seen, received) = tokio::sync::oneshot::channel();
    let (release, gate) = tokio::sync::oneshot::channel();
    let (endpoint, ca, task) = fixture_with_gate(
        "200 OK",
        &history().to_string(),
        "x-vmanyware-authenticated: bearer\r\n",
        Some((seen, gate)),
    )
    .await;
    let (connection, id) = manager(&endpoint, ca);
    let pending = connection.clone();
    let request =
        tokio::spawn(async move { pending.recreation_history(id, &"a".repeat(32)).await });
    received.await.unwrap();
    connection.begin().unwrap();
    release.send(()).unwrap();
    assert!(request.await.unwrap().is_err());
    assert!(connection.recreation_state_is_empty());
    task.await.unwrap();
}
