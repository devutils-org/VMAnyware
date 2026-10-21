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
    json!({"instanceId":"a".repeat(32),"providerScopeId":"b".repeat(64),"observation":{"node":"saved-node","vmid":202,"status":"running","qmpStatus":"running"}})
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
async fn native_instance_status_reads_only_fixed_get_and_returns_power_without_readiness() {
    for (power, qmp) in [
        ("running", Some("running")),
        ("running", Some("paused")),
        ("running", Some("unknown")),
        ("stopped", None),
    ] {
        let mut value = response();
        value["observation"]["status"] = json!(power);
        value["observation"]["qmpStatus"] = json!(qmp);
        value["guestReady"] = json!(true);
        value["unexpected"] = json!("PRIVATE_EXTRA");
        let (endpoint, ca, server) = fixture("200 OK", &value.to_string(), AUTH).await;
        let (state, id) = manager(&endpoint, ca);
        let result =
            serde_json::to_value(state.instance_status(id, &target()).await.unwrap()).unwrap();
        assert_eq!(result["observation"]["status"], power);
        assert!(result.get("guestReady").is_none());
        assert!(!result.to_string().contains("PRIVATE_EXTRA"));
        let request = server.await.unwrap();
        assert!(request.starts_with(&format!(
            "GET /api/v1/environments/{}/status ",
            "a".repeat(32)
        )));
        assert!(!request.to_lowercase().contains("idempotency-key:"));
    }
}
#[tokio::test]
async fn native_instance_status_refuses_other_instance_scope_node_vmid_or_unknown_states() {
    for variant in 0..6 {
        let mut value = response();
        match variant {
            0 => value["instanceId"] = json!("f".repeat(32)),
            1 => value["providerScopeId"] = json!("f".repeat(64)),
            2 => value["observation"]["node"] = json!("other-node"),
            3 => value["observation"]["vmid"] = json!(203),
            4 => value["observation"]["status"] = json!("ready"),
            _ => value["observation"]["qmpStatus"] = json!("arbitrary-provider-text"),
        }
        let (endpoint, ca, server) = fixture("200 OK", &value.to_string(), AUTH).await;
        let (state, id) = manager(&endpoint, ca);
        assert!(state.instance_status(id, &target()).await.is_err());
        server.await.unwrap();
    }
}
#[tokio::test]
async fn native_instance_status_refuses_invalid_target_before_network() {
    let state = RemoteConnection::default();
    let id = state.begin().unwrap();
    state
        .complete(
            id,
            Profile::new("https://fixture.invalid", TOKEN, None).unwrap(),
        )
        .unwrap();
    for variant in 0..4 {
        let mut target = target();
        match variant {
            0 => target.instance_id = "../bad".into(),
            1 => target.provider_scope_id = "wrong".into(),
            2 => target.node = "path/bad".into(),
            _ => target.vmid = 99,
        }
        assert_eq!(
            state.instance_status(id, &target).await.err().unwrap(),
            "Choose a valid registered instance and provider target."
        );
    }
}
#[tokio::test]
async fn native_instance_status_refuses_http_auth_marker_or_size_failures_without_echo() {
    for (status, headers, body) in [
        ("401 Unauthorized", "", "PRIVATE_BODY".into()),
        ("429 Too Many Requests", "", "PRIVATE_BODY".into()),
        ("404 Not Found", AUTH, "PRIVATE_BODY".into()),
        ("503 Service Unavailable", AUTH, "PRIVATE_BODY".into()),
        ("200 OK", "", response().to_string()),
        ("200 OK", AUTH, "x".repeat(4097)),
    ] {
        let (endpoint, ca, server) = fixture(status, &body, headers).await;
        let (state, id) = manager(&endpoint, ca);
        let error = state.instance_status(id, &target()).await.err().unwrap();
        assert!(!error.contains("PRIVATE_BODY") && !error.contains(TOKEN));
        server.await.unwrap();
    }
}
#[tokio::test]
async fn native_instance_status_discards_disconnected_observation() {
    let (seen, observed) = tokio::sync::oneshot::channel();
    let (release, wait) = tokio::sync::oneshot::channel();
    let (endpoint, ca, server) =
        fixture_with_gate("200 OK", &response().to_string(), AUTH, Some((seen, wait))).await;
    let (state, id) = manager(&endpoint, ca);
    let worker = state.clone();
    let task = tokio::spawn(async move { worker.instance_status(id, &target()).await });
    tokio::time::timeout(std::time::Duration::from_secs(3), observed)
        .await
        .unwrap()
        .unwrap();
    state.begin().unwrap();
    release.send(()).unwrap();
    assert!(task.await.unwrap().is_err());
    server.await.unwrap();
}
