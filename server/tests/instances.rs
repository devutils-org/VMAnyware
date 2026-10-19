use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use vmanyware_server::{
    definition::Definition,
    instance::Registration,
    proxmox::Proxmox,
    router_with_services,
    store::{RegistrationError, Store},
};

const SAMPLE: &str = include_str!("../../examples/kernel-lab.yaml");

fn registration() -> Registration {
    Registration {
        definition_name: "kernel-lab".into(),
        definition_hash: Definition::parse(SAMPLE).unwrap().hash().unwrap(),
        node: "pve-test".into(),
        vmid: 202,
    }
}

#[test]
fn association_snapshots_definition_and_survives_reopen_without_claiming_readiness() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("instances.sqlite");
    let store = Store::open(&path).unwrap();
    let original = store.create(&Definition::parse(SAMPLE).unwrap()).unwrap();
    let instance = store
        .register_instance(&registration(), &"a".repeat(64))
        .unwrap();
    let id = instance.summary.id;
    assert_eq!(instance.definition_yaml, original.yaml);
    assert_eq!(instance.summary.definition_hash, original.hash);
    assert_eq!(instance.summary.definition_revision, 1);
    drop(store);
    // A future definition edit must not rewrite the retained instance snapshot.
    let updated = Definition::parse(&SAMPLE.replace("cpu: 4", "cpu: 8")).unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE definitions SET definition_yaml=?1, definition_hash=?2, revision=2",
            rusqlite::params![updated.canonical_yaml().unwrap(), updated.hash().unwrap()],
        )
        .unwrap();
    drop(connection);
    let reopened = Store::open(&path).unwrap();
    let retained = reopened.instance(&id).unwrap().unwrap();
    assert_eq!(retained.definition_yaml, original.yaml);
    assert_eq!(retained.summary.definition_hash, original.hash);
    assert_eq!(retained.summary.definition_revision, 1);
    let value = serde_json::to_value(retained).unwrap();
    assert_eq!(value["status"], "registered");
    assert_eq!(
        value["providerReference"],
        json!({"provider":"proxmox", "scopeId":"a".repeat(64), "node":"pve-test", "vmid":202})
    );
    for field in ["ready", "observedState", "operationId", "ownershipVerified"] {
        assert!(value.get(field).is_none());
    }
    assert_eq!(reopened.recent_instances().unwrap().len(), 1);
}

#[test]
fn registration_rejects_missing_changed_or_invalid_inputs_without_creating_records() {
    let store = Store::memory().unwrap();
    let mut request = registration();
    let scope = "a".repeat(64);
    assert_eq!(
        store.register_instance(&request, &scope).unwrap_err(),
        RegistrationError::DefinitionNotFound
    );
    store.create(&Definition::parse(SAMPLE).unwrap()).unwrap();
    request.definition_hash = "b".repeat(64);
    assert_eq!(
        store.register_instance(&request, &scope).unwrap_err(),
        RegistrationError::DefinitionChanged
    );
    for value in ["", "A", &"g".repeat(64), &"A".repeat(64)] {
        request.definition_hash = value.into();
        assert_eq!(
            store.register_instance(&request, &scope).unwrap_err(),
            RegistrationError::InvalidRequest
        );
    }
    for name in ["", "../secret", "Upper", "-name", "name-", &"n".repeat(64)] {
        request = registration();
        request.definition_name = name.into();
        assert!(!request.is_valid());
    }
    for node in ["", "..", "-node", "pve/secret"] {
        request = registration();
        request.node = node.into();
        assert!(!request.is_valid());
    }
    request = registration();
    request.vmid = 99;
    assert!(!request.is_valid());
    assert_eq!(
        store
            .register_instance(&registration(), "not-a-scope")
            .unwrap_err(),
        RegistrationError::InvalidRequest
    );
    assert!(store.recent_instances().unwrap().is_empty());
}

#[test]
fn concurrent_connections_enforce_scope_vmid_identity_across_nodes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("concurrent.sqlite");
    let store = Store::open(&path).unwrap();
    store.create(&Definition::parse(SAMPLE).unwrap()).unwrap();
    let workers: Vec<_> = (0..8)
        .map(|index| {
            let path = path.clone();
            std::thread::spawn(move || {
                let store = Store::open(path).unwrap();
                let mut request = registration();
                request.node = format!("pve-{index}");
                store.register_instance(&request, &"a".repeat(64))
            })
        })
        .collect();
    let mut count = 0;
    for worker in workers {
        match worker.join().unwrap() {
            Ok(_) => count += 1,
            Err(error) => assert_eq!(error, RegistrationError::Conflict),
        }
    }
    assert_eq!(count, 1);
    store
        .register_instance(&registration(), &"b".repeat(64))
        .unwrap();
    let mut request = registration();
    request.vmid = 203;
    store.register_instance(&request, &"a".repeat(64)).unwrap();
    assert_eq!(store.recent_instances().unwrap().len(), 3);
}

#[test]
fn version_two_migrates_preserving_definitions_and_operations() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("v2.sqlite");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(include_str!("../../migrations/0001_definitions.sql"))
        .unwrap();
    connection
        .execute_batch(include_str!("../../migrations/0002_operations.sql"))
        .unwrap();
    let definition = Definition::parse(SAMPLE).unwrap();
    connection.execute("INSERT INTO definitions (name, definition_yaml, definition_hash, revision) VALUES ('kernel-lab',?1,?2,1)", rusqlite::params![definition.canonical_yaml().unwrap(), definition.hash().unwrap()]).unwrap();
    connection.execute("INSERT INTO operations (request_key,kind,status) VALUES ('retained','inventory_refresh','queued')", []).unwrap();
    connection.pragma_update(None, "user_version", 2).unwrap();
    drop(connection);
    let store = Store::open(&path).unwrap();
    store
        .register_instance(&registration(), &"a".repeat(64))
        .unwrap();
    assert_eq!(
        store.enqueue_inventory("retained").unwrap().status,
        vmanyware_server::operation::Status::Queued
    );
    assert_eq!(
        store.get("kernel-lab").unwrap().unwrap().hash,
        definition.hash().unwrap()
    );
    drop(store);
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        10
    );
}

async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 100_000).await.unwrap()).unwrap()
}
fn post(value: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/api/v1/environments")
        .header("content-type", "application/json")
        .body(Body::from(value.to_string()))
        .unwrap()
}
fn payload() -> Value {
    json!({"definitionName":"kernel-lab", "definitionHash": registration().definition_hash,"node":"pve-test","vmid":202})
}

#[tokio::test]
async fn api_registers_lists_and_reads_without_contacting_provider() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!(
        "https://localhost:{}",
        listener.local_addr().unwrap().port()
    );
    let provider = Proxmox::new(&endpoint, "reader@pve!test", "fake-secret", None).unwrap();
    let scope = provider.scope_id();
    let store = Store::memory().unwrap();
    store.create(&Definition::parse(SAMPLE).unwrap()).unwrap();
    let app = router_with_services(Some(provider), store.clone());
    let response = app.clone().oneshot(post(payload())).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let location = response.headers()["location"].to_str().unwrap().to_owned();
    let instance = body(response).await;
    assert_eq!(instance["status"], "registered");
    assert_eq!(instance["providerReference"]["scopeId"], scope);
    assert_eq!(
        instance["definitionYaml"],
        Definition::parse(SAMPLE).unwrap().canonical_yaml().unwrap()
    );
    assert!(!instance.to_string().contains("fake-secret"));
    assert!(!instance.to_string().contains(&endpoint));
    assert_eq!(
        app.clone().oneshot(post(payload())).await.unwrap().status(),
        StatusCode::CONFLICT
    );
    // History remains readable even if provider configuration is removed.
    let offline = router_with_services(None, store);
    let response = offline
        .clone()
        .oneshot(
            Request::builder()
                .uri(&location)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body(response).await, instance);
    let list = body(
        offline
            .oneshot(
                Request::builder()
                    .uri("/api/v1/environments")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert!(list[0].get("definitionYaml").is_none());
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(25), listener.accept())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn api_rejects_unsafe_registration_and_malformed_details_without_writes() {
    let provider = Proxmox::new(
        "https://localhost:9",
        "reader@pve!test",
        "fake-secret",
        None,
    )
    .unwrap();
    let store = Store::memory().unwrap();
    let app = router_with_services(Some(provider), store.clone());
    assert_eq!(
        app.clone().oneshot(post(payload())).await.unwrap().status(),
        StatusCode::NOT_FOUND
    );
    store.create(&Definition::parse(SAMPLE).unwrap()).unwrap();
    let mut stale = payload();
    stale["definitionHash"] = json!("a".repeat(64));
    let response = app.clone().oneshot(post(stale)).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(body(response).await["error"]["code"], "definition_changed");
    for field in ["providerScope", "status", "definitionYaml"] {
        let mut value = payload();
        value[field] = json!("secret-marker");
        let response = app.clone().oneshot(post(value)).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert!(!body(response).await.to_string().contains("secret-marker"));
    }
    let mut value = payload();
    value["node"] = json!("../secret-marker");
    assert_eq!(
        app.clone().oneshot(post(value)).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );
    let mut request = post(payload());
    request
        .headers_mut()
        .insert("origin", "https://untrusted.invalid".parse().unwrap());
    assert_eq!(
        app.clone().oneshot(request).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    for (content_type, input, expected) in [
        (
            "text/plain",
            "{}".into(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
        (
            "application/json",
            "{secret-marker".into(),
            StatusCode::BAD_REQUEST,
        ),
        (
            "application/json",
            "x".repeat(65537),
            StatusCode::PAYLOAD_TOO_LARGE,
        ),
    ] {
        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/environments")
            .header("content-type", content_type)
            .body(Body::from(input))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), expected);
        assert!(!body(response).await.to_string().contains("secret-marker"));
    }
    for id in ["secret-marker", &"a".repeat(32), "%FF"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/environments/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(body(response).await["error"]["code"], "instance_not_found");
    }
    assert!(store.recent_instances().unwrap().is_empty());
    let offline = router_with_services(None, store);
    assert_eq!(
        offline.oneshot(post(payload())).await.unwrap().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[test]
fn provider_scope_is_canonical_and_independent_of_credentials() {
    let first = Proxmox::new("https://PVE.EXAMPLE:443", "one@pve!test", "first", None).unwrap();
    let rotated = Proxmox::new("https://pve.example/", "two@pve!test", "second", None).unwrap();
    assert_eq!(first.scope_id(), rotated.scope_id());
    for endpoint in ["https://pve.example:8006", "https://other.example"] {
        assert_ne!(
            first.scope_id(),
            Proxmox::new(endpoint, "one@pve!test", "first", None)
                .unwrap()
                .scope_id()
        );
    }
}

#[test]
fn recent_associations_are_bounded_and_old_details_remain_available() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("bounded.sqlite");
    let store = Store::open(&path).unwrap();
    store.create(&Definition::parse(SAMPLE).unwrap()).unwrap();
    let mut ids = Vec::new();
    for vmid in 100..205 {
        let mut request = registration();
        request.vmid = vmid;
        ids.push(
            store
                .register_instance(&request, &"a".repeat(64))
                .unwrap()
                .summary
                .id,
        );
    }
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE instances SET created_at='2026-10-18T00:00:00.000Z'",
            [],
        )
        .unwrap();
    drop(connection);
    ids.sort();
    ids.reverse();
    let summaries = store.recent_instances().unwrap();
    assert_eq!(summaries.len(), 100);
    assert_eq!(
        summaries
            .iter()
            .map(|entry| entry.id.clone())
            .collect::<Vec<_>>(),
        ids[..100]
    );
    assert!(store.instance(&ids[104]).unwrap().is_some());
    assert!(serde_json::to_value(summaries).unwrap()[0]
        .get("definitionYaml")
        .is_none());
}

#[tokio::test]
async fn status_of_unknown_instance_is_safe_and_does_not_require_provider() {
    let app = router_with_services(None, Store::memory().unwrap());
    for id in ["secret-marker", &"a".repeat(32), "%FF"] {
        let uri = format!("/api/v1/environments/{id}/status");
        let response = app
            .clone()
            .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let json = body(response).await;
        assert_eq!(json["error"]["code"], "instance_not_found");
        assert!(!json.to_string().contains("secret-marker"));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}

#[tokio::test]
async fn lifecycle_preflight_resolves_instance_before_provider_configuration() {
    let store = Store::memory().unwrap();
    store.create(&Definition::parse(SAMPLE).unwrap()).unwrap();
    let id = store
        .register_instance(&registration(), &"a".repeat(64))
        .unwrap()
        .summary
        .id;
    let app = router_with_services(None, store);
    for (target, expected) in [
        (id.as_str(), StatusCode::SERVICE_UNAVAILABLE),
        ("missing-secret-marker", StatusCode::NOT_FOUND),
        ("%FF", StatusCode::NOT_FOUND),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/environments/{target}/lifecycle-preflight"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        assert!(!body(response).await.to_string().contains("secret-marker"));
    }
}
