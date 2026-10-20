use super::Fixture;
use crate::proxmox::{Error, VmReference};
use serde_json::{json, Value};
use std::time::Duration;
fn permissions() -> Value {
    json!({"data":{"/vms/202":{"VM.Audit":0}}})
}
async fn fixture(before: Value, ha: Value, jobs: Value, after: Value) -> Fixture {
    let bodies = [before, ha, jobs, after].map(|value| value.to_string());
    Fixture::responses(
        "200 OK",
        &[
            ("*", &bodies[0]),
            ("*", &bodies[1]),
            ("*", &bodies[2]),
            ("*", &bodies[3]),
        ],
        "",
        Duration::ZERO,
        true,
    )
    .await
}
#[tokio::test]
async fn deletion_management_proves_visibility_with_nonpropagated_audit_and_fixed_gets() {
    let mut after = permissions();
    after["data"]["/vms/202"]["VM.Audit"] = json!(false);
    let mut fixture = fixture(
        permissions(),
        json!({"data":[{"sid":"ct:203","comment":"private-ha"}]}),
        json!({"data":[{"id":"203-0","guest":"203","privateExtra":"private-job"}]}),
        after,
    )
    .await;
    let result = fixture
        .client()
        .deletion_management(&VmReference::new("pve-lab", 202).unwrap())
        .await
        .unwrap();
    assert!(!result.ha_managed && result.replication_jobs.is_empty());
    assert!(!serde_json::to_string(&result).unwrap().contains("private"));
    for path in [
        "/api2/json/access/permissions?path=%2Fvms%2F202",
        "/api2/json/cluster/ha/resources",
        "/api2/json/cluster/replication",
        "/api2/json/access/permissions?path=%2Fvms%2F202",
    ] {
        assert!(fixture
            .requests
            .recv()
            .await
            .unwrap()
            .starts_with(&format!("GET {path} HTTP/1.1")));
    }
    assert!(fixture.requests.try_recv().is_err());
}
#[tokio::test]
async fn deletion_management_refuses_missing_revoked_or_malformed_visibility() {
    for variant in ["missing", "wrong-path", "revoked", "malformed"] {
        let mut before = permissions();
        let mut after = permissions();
        match variant {
            "missing" => before = json!({"data":{"/vms/202":{}}}),
            "wrong-path" => before = json!({"data":{"/vms/203":{"VM.Audit":1}}}),
            "revoked" => after = json!({"data":{}}),
            _ => before["data"]["/vms/202"]["VM.Audit"] = json!("private-invalid"),
        }
        let fixture = fixture(before, json!({"data":[]}), json!({"data":[]}), after).await;
        let error = fixture
            .client()
            .deletion_management(&VmReference::new("pve-lab", 202).unwrap())
            .await
            .unwrap_err();
        assert_eq!(
            error,
            if variant == "malformed" {
                Error::InvalidResponse
            } else {
                Error::Forbidden
            },
            "{variant}"
        );
    }
    let mut fixture = Fixture::new("403 Forbidden", "private-denied", "", Duration::ZERO).await;
    assert_eq!(
        fixture
            .client()
            .deletion_management(&VmReference::new("pve-lab", 202).unwrap())
            .await
            .unwrap_err(),
        Error::Forbidden
    );
    assert!(fixture
        .requests
        .recv()
        .await
        .unwrap()
        .starts_with("GET /api2/json/access/permissions?path="));
    assert!(fixture.requests.try_recv().is_err());
}
#[tokio::test]
async fn deletion_management_rejects_ambiguous_duplicate_and_oversized_dependency_lists() {
    for variant in [
        "ha-duplicate",
        "ha-malformed",
        "ha-count",
        "job-duplicate",
        "job-mismatch",
        "job-guest",
        "job-count",
    ] {
        let mut ha = json!({"data":[]});
        let mut jobs = json!({"data":[]});
        match variant {
            "ha-duplicate" => ha = json!({"data":[{"sid":"vm:202"},{"sid":"vm:202"}]}),
            "ha-malformed" => ha = json!({"data":[{"sid":"vm:0202"}]}),
            "ha-count" => {
                ha = json!({"data":(0..4097).map(|index|json!({"sid":format!("vm:{}",index+100)})).collect::<Vec<_>>()})
            }
            "job-duplicate" => {
                jobs = json!({"data":[{"id":"202-0","guest":202},{"id":"202-0","guest":202}]})
            }
            "job-mismatch" => jobs = json!({"data":[{"id":"203-0","guest":202}]}),
            "job-guest" => jobs = json!({"data":[{"id":"202-0","guest":"0202"}]}),
            _ => {
                jobs = json!({"data":(0..4097).map(|index|json!({"id":format!("202-{index}"),"guest":202})).collect::<Vec<_>>()})
            }
        }
        let fixture = fixture(permissions(), ha, jobs, permissions()).await;
        assert_eq!(
            fixture
                .client()
                .deletion_management(&VmReference::new("pve-lab", 202).unwrap())
                .await
                .unwrap_err(),
            if variant.ends_with("count") {
                Error::ResponseTooLarge
            } else {
                Error::InvalidResponse
            },
            "{variant}"
        );
    }
}
