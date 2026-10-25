use super::Fixture;
use crate::proxmox::Error;
use std::time::Duration;

#[tokio::test]
async fn vm_privileges_reads_only_selected_path_and_keeps_nonpropagated_grants() {
    let mut fixture = Fixture::new(
        "200 OK",
        r#"{"data":{"/vms/9001":{"VM.Audit":0,"VM.PowerMgmt":false,"VM.Clone":1}}}"#,
        "",
        Duration::ZERO,
    )
    .await;
    assert_eq!(
        fixture.client().vm_privileges(9001).await.unwrap(),
        ["VM.Audit", "VM.Clone", "VM.PowerMgmt"]
    );
    assert!(fixture
        .requests
        .recv()
        .await
        .unwrap()
        .starts_with("GET /api2/json/access/permissions?path=%2Fvms%2F9001 HTTP/1.1"));
    assert!(fixture.requests.try_recv().is_err());
}

#[tokio::test]
async fn vm_privileges_refuses_malformed_scope_and_propagation() {
    for body in [
        r#"{"data":{"/vms/100":{"VM.Audit":1}}}"#,
        r#"{"data":{"/vms/9001":{"VM.Audit":2}}}"#,
        r#"{"data":{"/vms/9001":{"VM.Audit":"secret"}}}"#,
        r#"{"data":{"/vms/9001":{"bad\nname":1}}}"#,
    ] {
        let fixture = Fixture::new("200 OK", body, "", Duration::ZERO).await;
        assert_eq!(
            fixture.client().vm_privileges(9001).await,
            Err(Error::InvalidResponse)
        );
    }
    let mut fixture = Fixture::new("200 OK", r#"{"data":{}}"#, "", Duration::ZERO).await;
    assert_eq!(
        fixture.client().vm_privileges(99).await,
        Err(Error::Configuration)
    );
    assert!(fixture.requests.try_recv().is_err());
    assert!(fixture
        .client()
        .vm_privileges(9001)
        .await
        .unwrap()
        .is_empty());
}
