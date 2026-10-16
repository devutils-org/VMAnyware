use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use pocketlab_server::{
    operation::{Failure, Status},
    router_with_services,
    service_lock::ServiceLock,
    store::{Error, Store},
    worker,
};
use serde_json::{json, Value};
use tower::ServiceExt;

#[test]
fn operation_lifecycle_is_durable_and_terminal_results_are_immutable() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("operations.sqlite");
    let id;
    {
        let store = Store::open(&path).unwrap();
        let operation = store.enqueue_inventory("request-1").unwrap();
        id = operation.id;
        assert_eq!(operation.status, Status::Queued);
        assert_eq!(
            store.finish_inventory(&id, Ok(json!({}))).unwrap_err(),
            Error::InvalidOperation
        );
        assert_eq!(store.claim_inventory().unwrap().unwrap().id, id);
        assert!(store.claim_inventory().unwrap().is_none());
        let completed = store
            .finish_inventory(&id, Ok(json!({"nodes":[]})))
            .unwrap();
        assert_eq!(completed.status, Status::Succeeded);
        assert_eq!(
            store
                .finish_inventory(&id, Err(Failure::ProviderTimeout))
                .unwrap_err(),
            Error::InvalidOperation
        );
    }
    let store = Store::open(&path).unwrap();
    let operation = store.operation(&id).unwrap().unwrap();
    assert_eq!(operation.result, Some(json!({"nodes":[]})));
    assert_eq!(store.enqueue_inventory("request-1").unwrap().id, id);
    assert_eq!(store.recover_interrupted_operations().unwrap(), 0);
}

#[test]
fn startup_preserves_interruption_and_does_not_replay_running_work() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("operations.sqlite");
    let id;
    {
        let store = Store::open(&path).unwrap();
        id = store.enqueue_inventory("crash-request").unwrap().id;
        store.claim_inventory().unwrap().unwrap();
    }
    let store = Store::open(&path).unwrap();
    assert_eq!(
        store.operation(&id).unwrap().unwrap().status,
        Status::Running
    );
    assert_eq!(store.recover_interrupted_operations().unwrap(), 1);
    let operation = store.operation(&id).unwrap().unwrap();
    assert_eq!(operation.status, Status::Interrupted);
    assert_eq!(operation.failure, Some(Failure::ProcessInterrupted));
    assert!(operation.result.is_none());
    assert_eq!(store.enqueue_inventory("crash-request").unwrap().id, id);
    assert!(store.claim_inventory().unwrap().is_none());
    let queued = store.enqueue_inventory("new-read-only-attempt").unwrap();
    assert_eq!(store.recover_interrupted_operations().unwrap(), 0);
    assert_eq!(store.claim_inventory().unwrap().unwrap().id, queued.id);
}

#[test]
fn concurrent_requests_share_one_id_and_other_active_work_conflicts() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("operations.sqlite");
    Store::open(&path).unwrap();
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let path = path.clone();
            std::thread::spawn(move || {
                Store::open(path)
                    .unwrap()
                    .enqueue_inventory("shared-request")
                    .unwrap()
                    .id
            })
        })
        .collect();
    let ids: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert!(ids.iter().all(|id| id == &ids[0]));
    let store = Store::open(&path).unwrap();
    assert_eq!(
        store.enqueue_inventory("another-request").unwrap_err(),
        Error::Conflict
    );
    let claimers: Vec<_> = (0..8)
        .map(|_| {
            let path = path.clone();
            std::thread::spawn(move || Store::open(path).unwrap().claim_inventory().unwrap())
        })
        .collect();
    assert_eq!(
        claimers
            .into_iter()
            .filter_map(|worker| worker.join().unwrap())
            .count(),
        1
    );
}

#[test]
fn version_one_migrates_without_losing_definitions_and_service_lock_is_exclusive() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("operations.sqlite");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(include_str!("../../migrations/0001_definitions.sql"))
        .unwrap();
    connection.execute("INSERT INTO definitions (name, definition_yaml, definition_hash, revision) VALUES ('existing','source','hash',1)", []).unwrap();
    connection.pragma_update(None, "user_version", 1).unwrap();
    drop(connection);
    let lock = ServiceLock::acquire(&path).unwrap();
    assert!(ServiceLock::acquire(&path).is_err());
    let store = Store::open(&path).unwrap();
    assert_eq!(store.get("existing").unwrap().unwrap().hash, "hash");
    assert_eq!(
        store.enqueue_inventory("migrated-request").unwrap().status,
        Status::Queued
    );
    drop(lock);
    assert!(ServiceLock::acquire(&path).is_ok());
}

#[tokio::test]
async fn worker_records_missing_configuration_as_safe_terminal_failure() {
    let store = Store::memory().unwrap();
    let id = store.enqueue_inventory("missing-provider").unwrap().id;
    assert!(worker::run_one(&store, None).await.unwrap());
    let operation = store.operation(&id).unwrap().unwrap();
    assert_eq!(operation.status, Status::Failed);
    assert_eq!(operation.failure, Some(Failure::ProviderNotConfigured));
    assert!(!worker::run_one(&store, None).await.unwrap());
}

async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap()
}

#[tokio::test]
async fn api_accepts_idempotent_jobs_and_serves_persisted_status() {
    let store = Store::memory().unwrap();
    let app = router_with_services(None, store.clone());
    let request = || {
        Request::builder()
            .method("POST")
            .uri("/api/v1/provider/inventory/refresh")
            .header("idempotency-key", "same-key")
            .body(Body::empty())
            .unwrap()
    };
    let response = app.clone().oneshot(request()).await.unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let location = response.headers()["location"].to_str().unwrap().to_string();
    let original = body(response).await;
    assert_eq!(original["status"], "queued");
    assert_eq!(
        body(app.clone().oneshot(request()).await.unwrap()).await["id"],
        original["id"]
    );
    worker::run_one(&store, None).await.unwrap();
    let response = app
        .oneshot(
            Request::builder()
                .uri(location)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let status = body(response).await;
    assert_eq!(status["status"], "failed");
    assert_eq!(status["failure"], "provider_not_configured");
    assert!(status["result"].is_null());
}

#[tokio::test]
async fn rejected_operation_requests_create_no_work() {
    let store = Store::memory().unwrap();
    let app = router_with_services(None, store.clone());
    for (key, origin, expected) in [
        (None, false, StatusCode::BAD_REQUEST),
        (Some("bad key"), false, StatusCode::BAD_REQUEST),
        (Some("good"), true, StatusCode::FORBIDDEN),
    ] {
        let mut request = Request::builder()
            .method("POST")
            .uri("/api/v1/provider/inventory/refresh");
        if let Some(key) = key {
            request = request.header("idempotency-key", key);
        }
        if origin {
            request = request.header("origin", "https://untrusted.invalid");
        }
        assert_eq!(
            app.clone()
                .oneshot(request.body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            expected
        );
    }
    assert!(store.claim_inventory().unwrap().is_none());
    assert_eq!(
        app.oneshot(
            Request::builder()
                .uri("/api/v1/operations/invalid")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap()
        .status(),
        StatusCode::NOT_FOUND
    );
}
