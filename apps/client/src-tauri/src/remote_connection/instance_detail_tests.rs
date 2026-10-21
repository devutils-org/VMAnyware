use super::*;
use crate::remote_connection::{
    tests::{fixture, fixture_with_gate},
    Profile,
};
use serde_json::{json, Value};
use std::sync::Arc;
const TOKEN: &str = "fixture-only-access-token-0123456789abcdef";
const AUTH: &str = "x-vmanyware-authenticated: bearer\r\n";
fn target() -> Target {
    Target {
        instance_id: "a".repeat(32),
        provider_scope_id: "b".repeat(64),
        node: "saved-node".into(),
        vmid: 202,
    }
}
fn response() -> Value {
    let yaml="apiVersion: vmanyware.dev/v1\nkind: Environment\nmetadata:\n  name: saved-lab\n  description: Définition <b>literal</b>\n";
    json!({"id":"a".repeat(32),"definitionName":"saved-lab","definitionHash":format!("{:x}",Sha256::digest(yaml.as_bytes())),"definitionRevision":3,"definitionYaml":yaml,"providerReference":{"provider":"proxmox","scopeId":"b".repeat(64),"node":"saved-node","vmid":202},"status":"registered"})
}
fn manager(endpoint: &str, ca: reqwest::Certificate) -> (Arc<RemoteConnection>, u64) {
    let state = Arc::new(RemoteConnection::default());
    let id = state.begin().unwrap();
    state
        .complete(id, Profile::new(endpoint, TOKEN, Some(ca)).unwrap())
        .unwrap();
    (state, id)
}
#[tokio::test]
async fn native_instance_detail_reads_exact_saved_yaml_hash_for_registered_or_deleted_target() {
    for status in ["registered", "deleted"] {
        let mut value = response();
        value["status"] = json!(status);
        value["guestReady"] = json!(true);
        value["unexpected"] = json!("PRIVATE_EXTRA");
        let (endpoint, ca, server) = fixture("200 OK", &value.to_string(), AUTH).await;
        let (state, id) = manager(&endpoint, ca);
        let result = serde_json::to_value(
            state
                .instance_detail(id, &target(), "saved-lab")
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(result["definitionYaml"], value["definitionYaml"]);
        assert_eq!(result["definitionRevision"], 3);
        assert_eq!(result["status"], status);
        assert!(result.get("guestReady").is_none());
        assert!(!result.to_string().contains("PRIVATE_EXTRA"));
        let request = server.await.unwrap();
        assert!(request.starts_with(&format!("GET /api/v1/environments/{} ", "a".repeat(32))));
        assert!(!request.to_lowercase().contains("idempotency-key:"));
    }
}
#[tokio::test]
async fn native_instance_detail_refuses_unrelated_metadata_targets_and_changed_yaml() {
    for variant in 0..10 {
        let mut value = response();
        match variant {
            0 => value["id"] = json!("f".repeat(32)),
            1 => value["definitionName"] = json!("other-lab"),
            2 => value["definitionRevision"] = json!(0),
            3 => value["definitionHash"] = json!("f".repeat(64)),
            4 => value["providerReference"]["provider"] = json!("other"),
            5 => value["providerReference"]["scopeId"] = json!("f".repeat(64)),
            6 => value["providerReference"]["node"] = json!("other-node"),
            7 => value["providerReference"]["vmid"] = json!(203),
            8 => value["status"] = json!("ready"),
            _ => value["definitionYaml"] = json!("PRIVATE_DAMAGED_YAML"),
        }
        let (endpoint, ca, server) = fixture("200 OK", &value.to_string(), AUTH).await;
        let (state, id) = manager(&endpoint, ca);
        let error = state
            .instance_detail(id, &target(), "saved-lab")
            .await
            .err()
            .unwrap();
        assert!(!error.contains("PRIVATE_DAMAGED_YAML"));
        server.await.unwrap();
    }
}
#[tokio::test]
async fn native_instance_detail_bounds_yaml_bytes_before_accepting_matching_hash() {
    for yaml in [String::new(), "é".repeat(32769)] {
        let mut value = response();
        value["definitionHash"] = json!(format!("{:x}", Sha256::digest(yaml.as_bytes())));
        value["definitionYaml"] = json!(yaml);
        let (endpoint, ca, server) = fixture("200 OK", &value.to_string(), AUTH).await;
        let (state, id) = manager(&endpoint, ca);
        assert!(state
            .instance_detail(id, &target(), "saved-lab")
            .await
            .is_err());
        server.await.unwrap();
    }
}
#[tokio::test]
async fn native_instance_detail_refuses_invalid_inputs_and_http_auth_size_failures_without_echo() {
    let state = RemoteConnection::default();
    let id = state.begin().unwrap();
    state
        .complete(
            id,
            Profile::new("https://fixture.invalid", TOKEN, None).unwrap(),
        )
        .unwrap();
    let mut bad = target();
    bad.instance_id = "../bad".into();
    assert!(state
        .instance_detail(id, &bad, "saved-lab")
        .await
        .err()
        .unwrap()
        .contains("Choose a valid"));
    assert!(state
        .instance_detail(id, &target(), "")
        .await
        .err()
        .unwrap()
        .contains("Choose a valid"));
    for (status, headers, body) in [
        ("401 Unauthorized", "", "PRIVATE_BODY".into()),
        ("429 Too Many Requests", "", "PRIVATE_BODY".into()),
        ("404 Not Found", AUTH, "PRIVATE_BODY".into()),
        ("503 Service Unavailable", AUTH, "PRIVATE_BODY".into()),
        ("200 OK", "", response().to_string()),
        ("200 OK", AUTH, "x".repeat(256 * 1024 + 1)),
    ] {
        let (endpoint, ca, server) = fixture(status, &body, headers).await;
        let (state, id) = manager(&endpoint, ca);
        let error = state
            .instance_detail(id, &target(), "saved-lab")
            .await
            .err()
            .unwrap();
        assert!(!error.contains("PRIVATE_BODY") && !error.contains(TOKEN));
        server.await.unwrap();
    }
}
#[tokio::test]
async fn native_instance_detail_discards_disconnected_saved_definition() {
    let (seen, observed) = tokio::sync::oneshot::channel();
    let (release, wait) = tokio::sync::oneshot::channel();
    let (endpoint, ca, server) =
        fixture_with_gate("200 OK", &response().to_string(), AUTH, Some((seen, wait))).await;
    let (state, id) = manager(&endpoint, ca);
    let worker = state.clone();
    let task =
        tokio::spawn(async move { worker.instance_detail(id, &target(), "saved-lab").await });
    tokio::time::timeout(std::time::Duration::from_secs(3), observed)
        .await
        .unwrap()
        .unwrap();
    state.begin().unwrap();
    release.send(()).unwrap();
    assert!(task.await.unwrap().is_err());
    server.await.unwrap();
}
