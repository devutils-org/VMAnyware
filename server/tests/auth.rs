use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use tower::ServiceExt;
use vmanyware_server::{
    auth::{protect, AccessToken},
    router,
};
const TOKEN: &str = "fixture-only-access-token-0123456789abcdef";
#[tokio::test]
async fn access_token_protects_reads_writes_fallbacks_and_rejects_ambiguous_headers() {
    let app = protect(router(), Some(AccessToken::parse(TOKEN).unwrap()));
    for (method, path) in [
        ("GET", "/api/v1/health"),
        ("GET", "/api/v1/definitions"),
        ("POST", "/api/v1/definitions"),
        ("POST", "/api/v1/environments"),
        ("POST", "/api/v1/environments/fixture/start"),
        ("GET", "/unknown"),
        ("OPTIONS", "/api/v1/health"),
    ] {
        for value in [
            None,
            Some("Bearer wrong"),
            Some("Basic fixture"),
            Some("Bearer fixture-only-access-token-0123456789abcdeg"),
        ] {
            let mut request = Request::builder().method(method).uri(path);
            if let Some(value) = value {
                request = request.header(header::AUTHORIZATION, value);
            }
            let response = app
                .clone()
                .oneshot(request.body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            assert_eq!(response.headers()[header::WWW_AUTHENTICATE], "Bearer");
            let body = to_bytes(response.into_body(), 1024).await.unwrap();
            let text = std::str::from_utf8(&body).unwrap();
            assert!(!text.contains(TOKEN));
            assert!(!text.contains("wrong"));
        }
    }
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
                .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
#[tokio::test]
async fn valid_access_reaches_real_routes_and_unconfigured_local_development_is_preserved() {
    let app = protect(router(), Some(AccessToken::parse(TOKEN).unwrap()));
    for (path, status) in [
        ("/api/v1/health", StatusCode::OK),
        ("/api/v1/definitions", StatusCode::OK),
        ("/unknown", StatusCode::NOT_FOUND),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header(header::AUTHORIZATION, format!("bEaReR {TOKEN}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        assert_eq!(response.headers()["x-vmanyware-authenticated"], "bearer");
    }
    let response = protect(router(), None)
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response
        .headers()
        .get("x-vmanyware-authenticated")
        .is_none());
}
#[test]
fn invalid_access_configuration_fails_without_echoing_values_and_debug_is_redacted() {
    for input in [
        "",
        "short",
        "token with whitespace-and-otherwise-enough-characters",
        &"a".repeat(257),
    ] {
        let error = AccessToken::parse(input).unwrap_err().to_string();
        assert!(error.starts_with("VMANYWARE_ACCESS_TOKEN must"));
        assert!(!error.contains(input) || input.is_empty());
    }
    assert!(!format!("{:?}", AccessToken::parse(TOKEN).unwrap()).contains(TOKEN));
}

#[tokio::test]
async fn running_binary_requires_token_and_rejects_invalid_startup_configuration() {
    use std::{
        net::TcpListener,
        process::{Command, Stdio},
        time::Duration,
    };
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let mut child = Child(
        Command::new(env!("CARGO_BIN_EXE_vmanyware-server"))
            .env_clear()
            .env("VMANYWARE_PORT", port.to_string())
            .env("VMANYWARE_DATABASE", directory.path().join("auth.sqlite"))
            .env("VMANYWARE_ACCESS_TOKEN", TOKEN)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(1))
        .build()
        .unwrap();
    let base = format!("http://127.0.0.1:{port}");
    let mut ready = false;
    for _ in 0..100 {
        if let Ok(response) = client.get(format!("{base}/api/v1/health")).send().await {
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            ready = true;
            break;
        }
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "Service exited before readiness"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(ready, "Service failed to start");
    assert_eq!(
        client
            .get(format!("{base}/api/v1/health"))
            .bearer_auth(TOKEN)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        client
            .post(format!("{base}/api/v1/definitions"))
            .body("fixture-not-a-definition")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let response = client
        .get(format!("{base}/api/v1/definitions"))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.text().await.unwrap(), "[]");
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    let mut logs = String::new();
    use std::io::Read;
    child
        .0
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut logs)
        .unwrap();
    assert!(!logs.contains(TOKEN));
    let invalid = Command::new(env!("CARGO_BIN_EXE_vmanyware-server"))
        .env_clear()
        .env("VMANYWARE_ACCESS_TOKEN", "short-private-input")
        .env(
            "VMANYWARE_DATABASE",
            directory.path().join("must-not-exist.sqlite"),
        )
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    assert!(!directory.path().join("must-not-exist.sqlite").exists());
    let logs = String::from_utf8_lossy(&invalid.stderr);
    assert!(logs.contains("invalid_access_configuration"));
    assert!(!logs.contains("short-private-input"));
}
