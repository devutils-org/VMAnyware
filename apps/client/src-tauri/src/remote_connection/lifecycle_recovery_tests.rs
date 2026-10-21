use super::*;
use crate::remote_connection::{
    tests::{fixture, fixture_with_gate},
    Profile,
};
use serde_json::{json, Value};
use std::sync::Arc;
const TOKEN: &str = "fixture-only-lifecycle-token-0123456789abcdef";
const AUTH: &str = "x-vmanyware-authenticated: bearer\r\n";
fn manager(endpoint: &str, ca: reqwest::Certificate) -> (Arc<RemoteConnection>, u64) {
    let state = Arc::new(RemoteConnection::default());
    let id = state.begin().unwrap();
    state
        .complete(id, Profile::new(endpoint, TOKEN, Some(ca)).unwrap())
        .unwrap();
    (state, id)
}
fn entry() -> Value {
    json!({"id":"e".repeat(32),"action":"start","status":"succeeded","failure":null,"providerTaskKnown":true,"createdAt":"2026-10-21T00:00:00Z"})
}
fn history() -> Value {
    json!({"instanceId":"a".repeat(32),"operations":[entry()]})
}
fn saved() -> Value {
    json!({"action":"start","submissionStarted":true,"target":{"instanceId":"a".repeat(32),"providerScopeId":"b".repeat(64),"definitionHash":"c".repeat(64),"node":"saved-node","vmid":202},"operation":{"id":"e".repeat(32),"kind":"vm_start","instanceId":"a".repeat(32),"status":"succeeded","providerTaskKnown":true,"failure":null,"createdAt":"2026-10-21T00:00:00Z","updatedAt":"2026-10-21T00:00:01Z","result":{"action":"start","instanceId":"a".repeat(32),"providerScopeId":"b".repeat(64),"taskStatus":"succeeded","observation":{"node":"saved-node","vmid":202,"status":"running","qmpStatus":null}}}})
}
#[tokio::test]
async fn native_lifecycle_recovery_uses_fixed_gets_and_returns_only_historical_evidence() {
    for detail in [false, true] {
        let value = if detail { saved() } else { history() };
        let (endpoint, ca, server) = fixture("200 OK", &value.to_string(), AUTH).await;
        let (state, id) = manager(&endpoint, ca);
        let returned = if detail {
            serde_json::to_value(
                state
                    .recover_lifecycle(id, &"a".repeat(32), &"e".repeat(32))
                    .await
                    .unwrap(),
            )
            .unwrap()
        } else {
            serde_json::to_value(state.lifecycle_history(id, &"a".repeat(32)).await.unwrap())
                .unwrap()
        };
        assert!(returned.get("confirmed").is_none() && returned.get("requestKey").is_none());
        if detail {
            assert_eq!(returned["target"]["providerScopeId"], "b".repeat(64));
            assert_eq!(returned["target"]["node"], "saved-node");
        }
        let request = server.await.unwrap();
        assert!(request.starts_with(&format!(
            "GET /api/v1/environments/{}/lifecycle-operations{} ",
            "a".repeat(32),
            if detail {
                format!("/{}", "e".repeat(32))
            } else {
                String::new()
            }
        )));
        assert!(!request.to_lowercase().contains("idempotency-key:"));
    }
}
#[tokio::test]
async fn native_lifecycle_recovery_history_refuses_scope_duplicates_order_limits_and_bad_status() {
    for variant in 0..7 {
        let mut value = history();
        match variant {
            0 => value["instanceId"] = json!("f".repeat(32)),
            1 => value["operations"] = json!([entry(), entry()]),
            2 => {
                let mut older = entry();
                older["id"] = json!("f".repeat(32));
                value["operations"] = json!([entry(), older]);
            }
            3 => {
                value["operations"] = json!((0..101)
                    .rev()
                    .map(|index| {
                        let mut row = entry();
                        row["id"] = json!(format!("{index:032x}"));
                        row
                    })
                    .collect::<Vec<_>>())
            }
            4 => value["operations"][0]["status"] = json!("unexpected"),
            5 => value["operations"][0]["providerTaskKnown"] = json!(false),
            _ => value["operations"][0]["createdAt"] = json!("bad\nmetadata"),
        }
        let (endpoint, ca, server) = fixture("200 OK", &value.to_string(), AUTH).await;
        let (state, id) = manager(&endpoint, ca);
        assert!(state.lifecycle_history(id, &"a".repeat(32)).await.is_err());
        server.await.unwrap();
    }
}
#[tokio::test]
async fn native_lifecycle_recovery_detail_refuses_unrelated_or_inconsistent_evidence() {
    for variant in 0..9 {
        let mut value = saved();
        match variant {
            0 => value["operation"]["id"] = json!("f".repeat(32)),
            1 => value["target"]["instanceId"] = json!("f".repeat(32)),
            2 => value["target"]["providerScopeId"] = json!("f".repeat(64)),
            3 => value["operation"]["result"]["observation"]["vmid"] = json!(203),
            4 => value["operation"]["result"]["observation"]["status"] = json!("stopped"),
            5 => value["action"] = json!("shutdown"),
            6 => value["submissionStarted"] = json!(false),
            7 => value["target"]["definitionHash"] = json!("bad"),
            _ => value["operation"]["failure"] = json!("PRIVATE_SERVER_FAILURE"),
        }
        let (endpoint, ca, server) = fixture("200 OK", &value.to_string(), AUTH).await;
        let (state, id) = manager(&endpoint, ca);
        let error = state
            .recover_lifecycle(id, &"a".repeat(32), &"e".repeat(32))
            .await
            .err()
            .unwrap();
        assert!(!error.contains("PRIVATE_SERVER_FAILURE"));
        server.await.unwrap();
    }
}
#[tokio::test]
async fn native_lifecycle_recovery_preserves_unknown_recorded_work_and_sanitizes_extra_fields() {
    let mut value = saved();
    value["operation"]["status"] = json!("uncertain");
    value["operation"]["result"] = Value::Null;
    value["operation"]["failure"] = json!("power_state_unconfirmed");
    value["confirmed"] = json!(true);
    value["unexpected"] = json!("PRIVATE_EXTRA");
    let (endpoint, ca, server) = fixture("200 OK", &value.to_string(), AUTH).await;
    let (state, id) = manager(&endpoint, ca);
    let returned = serde_json::to_value(
        state
            .recover_lifecycle(id, &"a".repeat(32), &"e".repeat(32))
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(returned["operation"]["status"], "uncertain");
    assert!(returned.get("confirmed").is_none());
    assert!(!returned.to_string().contains("PRIVATE_EXTRA"));
    server.await.unwrap();
}
#[tokio::test]
async fn native_lifecycle_recovery_refuses_auth_marker_http_failure_and_oversized_reply_without_echo(
) {
    for (status, headers, body) in [
        ("401 Unauthorized", "", "PRIVATE_BODY".into()),
        ("429 Too Many Requests", "", "PRIVATE_BODY".into()),
        ("503 Service Unavailable", AUTH, "PRIVATE_BODY".into()),
        ("200 OK", "", saved().to_string()),
        ("200 OK", AUTH, "x".repeat(256 * 1024 + 1)),
    ] {
        let (endpoint, ca, server) = fixture(status, &body, headers).await;
        let (state, id) = manager(&endpoint, ca);
        let error = state
            .recover_lifecycle(id, &"a".repeat(32), &"e".repeat(32))
            .await
            .err()
            .unwrap();
        assert!(!error.contains("PRIVATE_BODY") && !error.contains(TOKEN));
        server.await.unwrap();
    }
}
#[tokio::test]
async fn native_lifecycle_recovery_discards_disconnected_reply() {
    let (seen, observed) = tokio::sync::oneshot::channel();
    let (release, wait) = tokio::sync::oneshot::channel();
    let (endpoint, ca, server) =
        fixture_with_gate("200 OK", &saved().to_string(), AUTH, Some((seen, wait))).await;
    let (state, id) = manager(&endpoint, ca);
    let worker = state.clone();
    let task = tokio::spawn(async move {
        worker
            .recover_lifecycle(id, &"a".repeat(32), &"e".repeat(32))
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), observed)
        .await
        .unwrap()
        .unwrap();
    state.begin().unwrap();
    release.send(()).unwrap();
    assert!(task.await.unwrap().is_err());
    server.await.unwrap();
}
