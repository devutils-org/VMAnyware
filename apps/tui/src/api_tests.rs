use super::*;
use crate::fixtures::*;
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

const TOKEN: &str = "fixture-only-access-token-0123456789abcdef";
const AUTH: &str = "X-VMAnyware-Authenticated: bearer\r\n";
async fn fixture(
    status: &str,
    body: String,
    headers: &str,
) -> (String, String, tokio::task::JoinHandle<String>) {
    let key = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let pem = key.cert.pem();
    let config = tokio_rustls::rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![key.cert.der().clone()],
            tokio_rustls::rustls::pki_types::PrivatePkcs8KeyDer::from(
                key.signing_key.serialize_der(),
            )
            .into(),
        )
        .unwrap();
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!(
        "https://localhost:{}",
        listener.local_addr().unwrap().port()
    );
    let reply = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}",
        body.len()
    );
    let task = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let Ok(mut stream) = acceptor.accept(socket).await else {
            return String::new();
        };
        let mut request = Vec::new();
        let mut bytes = [0; 1024];
        while !request.windows(4).any(|v| v == b"\r\n\r\n") {
            let count = stream.read(&mut bytes).await.unwrap();
            if count == 0 {
                break;
            }
            request.extend_from_slice(&bytes[..count]);
            assert!(request.len() < 16 * 1024);
        }
        let _ = stream.write_all(reply.as_bytes()).await;
        String::from_utf8(request).unwrap()
    });
    (endpoint, pem, task)
}

#[test]
fn rejects_unsafe_origins_tokens_and_ca_before_requests() {
    for origin in [
        "http://localhost",
        "https://user:pass@host",
        "https://host/path",
        "https://host?q=1",
        "https://host/#fragment",
    ] {
        assert!(Api::new(origin, TOKEN, None).is_err());
    }
    for token in [
        "short",
        " fixture-only-access-token-0123456789abcdef",
        "fixture-only-access-token-0123456789abcdef\n",
    ] {
        assert!(Api::new("https://localhost", token, None).is_err());
    }
    for pem in ["PRIVATE CONTENT".to_owned(), "A".repeat(17 * 1024)] {
        assert!(Api::new("https://localhost", TOKEN, Some(pem.as_bytes())).is_err());
    }
}

#[tokio::test]
async fn verifies_tls_and_authentication_and_never_follows_redirects() {
    let (url, ca, task) = fixture("200 OK", "{\"status\":\"ok\"}".into(), AUTH).await;
    Api::new(&url, TOKEN, Some(ca.as_bytes()))
        .unwrap()
        .check()
        .await
        .unwrap();
    let request = task.await.unwrap();
    assert!(request.starts_with("GET /api/v1/health HTTP/1.1\r\n"));
    assert!(request
        .to_lowercase()
        .contains(&format!("authorization: bearer {TOKEN}")));

    let (url, _, task) = fixture("200 OK", "{\"status\":\"ok\"}".into(), AUTH).await;
    assert!(Api::new(&url, TOKEN, None).unwrap().check().await.is_err());
    assert!(task.await.unwrap().is_empty());

    let (url, ca, task) = fixture(
        "302 Found",
        "private upstream body".into(),
        "Location: https://unrelated.invalid/\r\n",
    )
    .await;
    let error = Api::new(&url, TOKEN, Some(ca.as_bytes()))
        .unwrap()
        .check()
        .await
        .unwrap_err();
    assert!(!error.contains("private") && !error.contains(TOKEN));
    assert!(task.await.unwrap().starts_with("GET /api/v1/health "));
}

#[tokio::test]
async fn rejects_missing_auth_marker_rate_limits_and_oversized_or_malformed_responses() {
    for (status, body, headers) in [
        ("200 OK", "{\"status\":\"ok\"}".into(), ""),
        ("401 Unauthorized", "private upstream body".into(), ""),
        ("429 Too Many Requests", "private upstream body".into(), ""),
        ("200 OK", "private upstream body".into(), AUTH),
        (
            "200 OK",
            format!("{{\"status\":\"{}\"}}", "x".repeat(2048)),
            AUTH,
        ),
    ] {
        let (url, ca, task) = fixture(status, body, headers).await;
        let error = Api::new(&url, TOKEN, Some(ca.as_bytes()))
            .unwrap()
            .check()
            .await
            .unwrap_err();
        assert!(!error.contains("private upstream") && !error.contains(TOKEN));
        task.await.unwrap();
    }
}

#[tokio::test]
async fn reads_scoped_environment_power_history_and_progress_using_only_gets() {
    let eid = "e".repeat(32);
    let oid = "d".repeat(32);
    let e = environment(&eid);
    let op = operation(&oid, &eid);
    let mut detail = environment_value(&eid);
    detail["definitionYaml"] = YAML.into();
    let power = serde_json::json!({"instanceId":eid,"providerScopeId":"c".repeat(64),"observation":{"node":"pve-test","vmid":202,"status":"running","qmpStatus":"running"}});
    for (case, body, path) in [
        (
            0,
            serde_json::json!([environment_value(&eid)]),
            "api/v1/environments".into(),
        ),
        (1, detail, format!("api/v1/environments/{eid}")),
        (2, power, format!("api/v1/environments/{eid}/status")),
        (
            3,
            serde_json::json!([operation_value(&oid, &eid)]),
            "api/v1/operations".into(),
        ),
        (
            4,
            progress_value(&oid, &eid),
            format!("api/v1/operations/{oid}/provisioning"),
        ),
    ] {
        let (url, ca, task) = fixture("200 OK", body.to_string(), AUTH).await;
        let api = Api::new(&url, TOKEN, Some(ca.as_bytes())).unwrap();
        match case {
            0 => assert_eq!(api.environments().await.unwrap()[0].id, eid),
            1 => assert_eq!(api.detail(&e).await.unwrap().definition_yaml, YAML),
            2 => assert_eq!(api.power(&e).await.unwrap().observation.status, "running"),
            3 => assert_eq!(api.operations().await.unwrap()[0].id, oid),
            _ => assert_eq!(
                api.progress(&op, std::slice::from_ref(&e))
                    .await
                    .unwrap()
                    .stages
                    .len(),
                5
            ),
        }
        let request = task.await.unwrap();
        assert!(request.starts_with(&format!("GET /{path} HTTP/1.1\r\n")));
        assert!(request
            .to_lowercase()
            .contains(&format!("authorization: bearer {TOKEN}")));
    }
}

#[tokio::test]
async fn rejects_cross_target_observations_tampered_yaml_and_duplicate_lists() {
    let eid = "e".repeat(32);
    let e = environment(&eid);
    let mut tampered = environment_value(&eid);
    tampered["definitionYaml"] = "tampered".into();
    let (url, ca, task) = fixture("200 OK", tampered.to_string(), AUTH).await;
    assert!(Api::new(&url, TOKEN, Some(ca.as_bytes()))
        .unwrap()
        .detail(&e)
        .await
        .is_err());
    task.await.unwrap();
    let power = serde_json::json!({"instanceId":"a".repeat(32),"providerScopeId":"c".repeat(64),"observation":{"node":"pve-test","vmid":202,"status":"running"}});
    let (url, ca, task) = fixture("200 OK", power.to_string(), AUTH).await;
    assert!(Api::new(&url, TOKEN, Some(ca.as_bytes()))
        .unwrap()
        .power(&e)
        .await
        .is_err());
    task.await.unwrap();
    let (url, ca, task) = fixture(
        "200 OK",
        serde_json::json!([environment_value(&eid), environment_value(&eid)]).to_string(),
        AUTH,
    )
    .await;
    assert!(Api::new(&url, TOKEN, Some(ca.as_bytes()))
        .unwrap()
        .environments()
        .await
        .is_err());
    task.await.unwrap();
    let oid = "d".repeat(32);
    let op = operation(&oid, &eid);
    let mut progress = progress_value(&oid, &eid);
    progress["vmid"] = 999.into();
    let (url, ca, task) = fixture("200 OK", progress.to_string(), AUTH).await;
    assert!(Api::new(&url, TOKEN, Some(ca.as_bytes()))
        .unwrap()
        .progress(&op, &[e])
        .await
        .is_err());
    task.await.unwrap();
}

#[test]
fn progress_keeps_uncertainty_and_rejects_out_of_order_stages() {
    let eid = "e".repeat(32);
    let oid = "d".repeat(32);
    let e = environment(&eid);
    let op = operation(&oid, &eid);
    let mut progress = progress_value(&oid, &eid);
    progress["status"] = "uncertain".into();
    progress["stages"][1]["status"] = "uncertain".into();
    assert!(serde_json::from_value::<Progress>(progress.clone())
        .unwrap()
        .matches(&op, &e));
    progress["stages"][2]["submissionStarted"] = true.into();
    assert!(!serde_json::from_value::<Progress>(progress)
        .unwrap()
        .matches(&op, &e));
}
