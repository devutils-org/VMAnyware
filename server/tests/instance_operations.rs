use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;
use vmanyware_server::{
    definition::Definition,
    instance::Registration,
    operation::{Failure, Status},
    router_with_services,
    store::{Error, RegistrationError, Store},
    worker,
};

fn register(store: &Store, vmid: u32) -> String {
    let definition = Definition::parse(include_str!("../../examples/kernel-lab.yaml")).unwrap();
    if store.get(&definition.metadata.name).unwrap().is_none() {
        store.create(&definition).unwrap();
    }
    store
        .register_instance(
            &Registration {
                definition_name: definition.metadata.name.clone(),
                definition_hash: definition.hash().unwrap(),
                node: "pve-test".into(),
                vmid,
            },
            &"a".repeat(64),
        )
        .unwrap()
        .summary
        .id
}

#[test]
fn instance_jobs_bind_keys_to_kind_and_target_and_preserve_interruption() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("instance-jobs.sqlite");
    let store = Store::open(&path).unwrap();
    let first = register(&store, 202);
    let second = register(&store, 203);
    let operation = store
        .enqueue_instance_observation("shared", &first)
        .unwrap();
    assert_eq!(
        store
            .enqueue_instance_observation("shared", &first)
            .unwrap()
            .id,
        operation.id
    );
    assert_eq!(
        store
            .enqueue_instance_observation("shared", &second)
            .unwrap_err(),
        RegistrationError::Conflict
    );
    assert_eq!(
        store.enqueue_inventory("shared").unwrap_err(),
        Error::Conflict
    );
    assert_eq!(
        store
            .enqueue_instance_observation("other-key", &first)
            .unwrap_err(),
        RegistrationError::Conflict
    );
    assert_eq!(store.claim_operation().unwrap().unwrap().id, operation.id);
    drop(store);
    let reopened = Store::open(&path).unwrap();
    assert_eq!(reopened.recover_interrupted_operations().unwrap(), 1);
    let interrupted = reopened.operation(&operation.id).unwrap().unwrap();
    assert_eq!(interrupted.instance_id.as_deref(), Some(first.as_str()));
    assert_eq!(interrupted.status, Status::Interrupted);
    assert_eq!(interrupted.failure, Some(Failure::ProcessInterrupted));
    assert_eq!(
        reopened
            .enqueue_instance_observation("shared", &first)
            .unwrap()
            .id,
        operation.id
    );
    assert!(reopened.claim_operation().unwrap().is_none());
    let inventory = reopened.enqueue_inventory("inventory-key").unwrap();
    assert_eq!(
        reopened
            .enqueue_instance_observation("inventory-key", &first)
            .unwrap_err(),
        RegistrationError::Conflict
    );
    assert!(inventory.instance_id.is_none());
    let new_attempt = reopened
        .enqueue_instance_observation("new-attempt", &first)
        .unwrap();
    assert_ne!(new_attempt.id, operation.id);
    assert_eq!(reopened.recent_operations().unwrap().len(), 3);
}

#[test]
fn concurrent_instance_requests_share_one_id_and_distinct_targets_can_queue() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("concurrent-jobs.sqlite");
    let store = Store::open(&path).unwrap();
    let first = register(&store, 202);
    let second = register(&store, 203);
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let path = path.clone();
            let first = first.clone();
            std::thread::spawn(move || {
                Store::open(path)
                    .unwrap()
                    .enqueue_instance_observation("same", &first)
                    .unwrap()
                    .id
            })
        })
        .collect();
    let ids: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert!(ids.iter().all(|id| id == &ids[0]));
    store
        .enqueue_instance_observation("second", &second)
        .unwrap();
    store.enqueue_inventory("inventory").unwrap();
    let mut claimed = Vec::new();
    while let Some(operation) = store.claim_operation().unwrap() {
        assert!(!claimed.contains(&operation.id));
        store
            .finish_operation(&operation.id, Err(Failure::ProviderNotConfigured))
            .unwrap();
        claimed.push(operation.id);
    }
    assert_eq!(claimed.len(), 3);
}

fn refresh(id: &str, key: Option<&str>) -> Request<Body> {
    let mut request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/environments/{id}/refresh"));
    if let Some(key) = key {
        request = request.header("idempotency-key", key);
    }
    request.body(Body::empty()).unwrap()
}
async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap()
}

#[tokio::test]
async fn instance_refresh_api_returns_durable_operation_before_provider_work() {
    let store = Store::memory().unwrap();
    let id = register(&store, 202);
    let app = router_with_services(None, store.clone());
    let response = app
        .clone()
        .oneshot(refresh(&id, Some("request-1")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let location = response.headers()["location"].to_str().unwrap().to_owned();
    let queued = body(response).await;
    assert_eq!(queued["status"], "queued");
    assert_eq!(queued["kind"], "instance_observe");
    assert_eq!(queued["instanceId"], id);
    assert!(queued["result"].is_null());
    assert!(worker::run_one(&store, None).await.unwrap());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(location)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let terminal = body(response).await;
    assert_eq!(terminal["status"], "failed");
    assert_eq!(terminal["failure"], "provider_not_configured");
    assert_eq!(terminal["id"], queued["id"]);
    assert_eq!(
        body(app.oneshot(refresh(&id, Some("request-1"))).await.unwrap()).await,
        terminal
    );
    assert!(!worker::run_one(&store, None).await.unwrap());
}

#[tokio::test]
async fn rejected_instance_refreshes_do_not_create_work() {
    let store = Store::memory().unwrap();
    let id = register(&store, 202);
    let app = router_with_services(None, store.clone());
    for (target, key, origin, expected) in [
        (id.as_str(), None, false, StatusCode::BAD_REQUEST),
        (id.as_str(), Some("bad key"), false, StatusCode::BAD_REQUEST),
        (id.as_str(), Some("good"), true, StatusCode::FORBIDDEN),
        (
            "bad-secret-marker",
            Some("good"),
            false,
            StatusCode::NOT_FOUND,
        ),
        (
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            Some("good"),
            false,
            StatusCode::NOT_FOUND,
        ),
    ] {
        let mut request = refresh(target, key);
        if origin {
            request
                .headers_mut()
                .insert("origin", "https://untrusted.invalid".parse().unwrap());
        }
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), expected);
        assert!(!body(response).await.to_string().contains("secret-marker"));
    }
    assert!(store.recent_operations().unwrap().is_empty());
    assert!(store.claim_operation().unwrap().is_none());
}

#[test]
fn version_three_upgrade_retains_instance_links_and_inventory_results() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("version-three.sqlite");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(include_str!("../../migrations/0001_definitions.sql"))
        .unwrap();
    connection
        .execute_batch(include_str!("../../migrations/0002_operations.sql"))
        .unwrap();
    connection
        .execute_batch(include_str!("../../migrations/0003_instances.sql"))
        .unwrap();
    let definition = Definition::parse(include_str!("../../examples/kernel-lab.yaml")).unwrap();
    connection.execute("INSERT INTO definitions (name,definition_yaml,definition_hash,revision) VALUES ('kernel-lab',?1,?2,1)", rusqlite::params![definition.canonical_yaml().unwrap(),definition.hash().unwrap()]).unwrap();
    let instance_id: String = connection.query_row("INSERT INTO instances (definition_name,definition_hash,definition_revision,definition_yaml,provider_scope,node,vmid) SELECT name,definition_hash,revision,definition_yaml,?1,'pve-test',202 FROM definitions RETURNING id", ["a".repeat(64)], |row| row.get(0)).unwrap();
    let operation_id: String = connection.query_row("INSERT INTO operations (request_key,kind,status,result_json) VALUES ('old','inventory_refresh','succeeded','{\"nodes\":[]}') RETURNING id", [], |row| row.get(0)).unwrap();
    connection.pragma_update(None, "user_version", 3).unwrap();
    drop(connection);
    let store = Store::open(&path).unwrap();
    assert_eq!(
        store
            .instance(&instance_id)
            .unwrap()
            .unwrap()
            .summary
            .provider_reference
            .vmid,
        202
    );
    let retained = store.operation(&operation_id).unwrap().unwrap();
    assert_eq!(retained.status, Status::Succeeded);
    assert!(retained.instance_id.is_none());
    assert_eq!(retained.result.unwrap(), serde_json::json!({"nodes":[]}));
    assert_eq!(store.enqueue_inventory("old").unwrap().id, operation_id);
    assert_eq!(
        store
            .enqueue_instance_observation("new", &instance_id)
            .unwrap()
            .instance_id
            .as_deref(),
        Some(instance_id.as_str())
    );
}
