use super::*;
use crate::proxmox::{PackageProgress, PackageRequest};

fn request(packages: &[&str]) -> Result<PackageRequest, Error> {
    PackageRequest::new(
        guest_readiness_request(),
        &packages.iter().map(|p| p.to_string()).collect::<Vec<_>>(),
    )
}
#[test]
fn package_request_rejects_options_shell_selectors_and_unbounded_input() {
    for packages in [
        vec![],
        vec!["--help"],
        vec!["git;id"],
        vec!["$(id)"],
        vec!["git\n"],
        vec!["git", "git"],
        vec!["git-"],
        vec!["git="],
        vec!["git=1=2"],
        vec!["git/stable"],
        vec!["git*"],
        vec!["git:amd64:other"],
    ] {
        assert!(request(&packages).is_err(), "{packages:?}");
    }
    assert!(request(&vec!["git"; 129]).is_err());
    assert!(request(&[&"a".repeat(129)]).is_err());
    assert!(request(&["git", "g++", "libc6:amd64=2:1.2.3-1~deb13"]).is_ok());
}
async fn fixture(status: &str) -> Fixture {
    let config = guest_guard_config();
    Fixture::responses(
        "200 OK",
        &[
            ("*", &config),
            (
                "*",
                r#"{"data":{"vmid":202,"status":"running","qmpstatus":"running"}}"#,
            ),
            ("*", r#"{"data":[]}"#),
            ("*", status),
            ("*", &config),
            (
                "*",
                r#"{"data":{"vmid":202,"status":"running","qmpstatus":"running"}}"#,
            ),
            ("*", r#"{"data":[]}"#),
        ],
        "",
        Duration::ZERO,
        true,
    )
    .await
}
#[tokio::test]
async fn package_dispatch_sends_fixed_bounded_command_with_positional_declarations() {
    let mut fixture = fixture(r#"{"data":{"pid":42}}"#).await;
    assert_eq!(
        fixture
            .client()
            .dispatch_packages(&request(&["git", "g++", "clang=1:19.0-1"]).unwrap())
            .await
            .unwrap(),
        42
    );
    let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
    assert_eq!(requests.len(), 4);
    assert!(requests[3].starts_with("POST /api2/json/nodes/pve-test/qemu/202/agent/exec "));
    let body: serde_json::Value =
        serde_json::from_str(requests[3].split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(body.as_object().unwrap().len(), 1);
    let command = body["command"].as_array().unwrap();
    assert_eq!(command[0], "/usr/bin/timeout");
    assert_eq!(command[2], "900");
    assert_eq!(
        &command[command.len() - 3..],
        &[
            serde_json::json!("git"),
            serde_json::json!("g++"),
            serde_json::json!("clang=1:19.0-1")
        ]
    );
    let script = command[8].as_str().unwrap();
    assert!(
        script.contains("--no-remove")
            && script.contains("\"$@\"")
            && script.contains("dpkg-query")
    );
    assert!(!script.contains("clang=1:19.0-1"));
    // Syntax validation is read-only; never execute the installer on this host.
    let mut child = std::process::Command::new("/bin/sh")
        .args(["-n", "-c", script])
        .spawn()
        .unwrap();
    assert!(child.wait().unwrap().success());
}
#[tokio::test]
async fn package_observation_returns_only_scoped_progress_without_output() {
    for (status, expected) in [
        (r#"{"data":{"exited":false}}"#, PackageProgress::Pending),
        (
            r#"{"data":{"exited":true,"exitcode":0}}"#,
            PackageProgress::Completed,
        ),
        (
            r#"{"data":{"exited":1,"exitcode":100}}"#,
            PackageProgress::Failed,
        ),
        (
            r#"{"data":{"exited":1,"signal":15}}"#,
            PackageProgress::Failed,
        ),
    ] {
        let mut fixture = fixture(status).await;
        assert_eq!(
            fixture
                .client()
                .observe_packages(&request(&["git"]).unwrap(), 42)
                .await
                .unwrap(),
            expected
        );
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert_eq!(requests.len(), 7);
        assert!(requests[3]
            .starts_with("GET /api2/json/nodes/pve-test/qemu/202/agent/exec-status?pid=42 "));
        assert!(requests.iter().all(|r| r.starts_with("GET ")));
    }
}
#[tokio::test]
async fn package_observation_rejects_inconsistent_or_leaking_guest_results() {
    for status in [
        r#"{"data":{"exited":0,"exitcode":0}}"#,
        r#"{"data":{"exited":1}}"#,
        r#"{"data":{"exited":1,"exitcode":0,"signal":15}}"#,
        r#"{"data":{"exited":1,"exitcode":0,"out-data":"private-repository-secret"}}"#,
        r#"{"data":{"exited":1,"exitcode":0,"err-truncated":1}}"#,
        r#"{"data":{"exited":2}}"#,
    ] {
        let fixture = fixture(status).await;
        assert_eq!(
            fixture
                .client()
                .observe_packages(&request(&["git"]).unwrap(), 42)
                .await
                .unwrap_err(),
            Error::InvalidResponse
        );
    }
}
#[tokio::test]
async fn package_dispatch_refuses_changed_identity_before_post_and_does_not_retry_lost_reply() {
    let changed = guest_guard_config().replace(&"f".repeat(40), &"e".repeat(40));
    let mut denied = Fixture::new("200 OK", &changed, "", Duration::ZERO).await;
    assert_eq!(
        denied
            .client()
            .dispatch_packages(&request(&["git"]).unwrap())
            .await
            .unwrap_err(),
        Error::InvalidResponse
    );
    assert_eq!(
        std::iter::from_fn(|| denied.requests.try_recv().ok()).count(),
        1
    );
    let mut lost = fixture(DROP_REPLY).await;
    assert!(lost
        .client()
        .dispatch_packages(&request(&["git"]).unwrap())
        .await
        .is_err());
    let requests: Vec<_> = std::iter::from_fn(|| lost.requests.try_recv().ok()).collect();
    assert_eq!(
        requests.iter().filter(|r| r.starts_with("POST ")).count(),
        1
    );
}
