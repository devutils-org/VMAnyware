use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use pocketlab_server::{
    definition::Definition,
    router,
    store::{Error, Store},
};
use serde_json::Value;
use tower::ServiceExt;

const SAMPLE: &str = include_str!("../../examples/kernel-lab.yaml");

#[test]
fn canonical_round_trip_includes_defaults_and_stable_hash() {
    let definition = Definition::parse(SAMPLE).unwrap();
    let yaml = definition.canonical_yaml().unwrap();
    let again = Definition::parse(&yaml).unwrap();
    assert_eq!(definition, again);
    assert_eq!(definition.hash().unwrap(), again.hash().unwrap());
    assert_eq!(definition.hash().unwrap().len(), 64);
    let changed = Definition::parse(&SAMPLE.replace("cpu: 4", "cpu: 5")).unwrap();
    assert_ne!(definition.hash().unwrap(), changed.hash().unwrap());
}

#[test]
fn schema_refuses_invalid_resources_versions_names_and_unknown_fields() {
    for yaml in [
        SAMPLE.replace("cpu: 4", "cpu: 0"),
        SAMPLE.replace("memoryMiB: 8192", "memoryMiB: 255"),
        SAMPLE.replace("diskGiB: 40", "diskGiB: 4097"),
        SAMPLE.replace("pocketlab.dev/v1", "pocketlab.dev/v99"),
        SAMPLE.replace("name: kernel-lab", "name: ../escape"),
        SAMPLE.replace("cloudInit: true", "cloudInit: false"),
        SAMPLE.replace("cpu: 4", "cpu: 4\n    typo: secret-value"),
        SAMPLE.replace("cpu: 4", "cpu: 4\n    cpu: 8"),
        SAMPLE.replace("- git", "- git;touch /tmp/unsafe"),
    ] {
        let error = Definition::parse(&yaml).unwrap_err();
        assert!(!error.to_string().contains("secret-value"));
    }
}

#[test]
fn private_keys_paths_duplicate_keys_and_oversized_documents_are_refused() {
    let public = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA pocketlab-example";
    for yaml in [
        SAMPLE.replace(public, "~/.ssh/id_ed25519.pub"),
        SAMPLE.replace(public, "-----BEGIN OPENSSH PRIVATE KEY-----"),
        SAMPLE.replace(public, "ssh-ed25519 invalid-base64"),
        SAMPLE.replace(public, &format!("{public}\n      - {public}")),
        "x".repeat(65537),
    ] {
        assert!(Definition::parse(&yaml).is_err());
    }
}

#[test]
fn definitions_survive_reopen_and_conflicts_do_not_overwrite() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("pocketlab.sqlite");
    let definition = Definition::parse(SAMPLE).unwrap();
    let original_hash;
    {
        let store = Store::open(&path).unwrap();
        let record = store.create(&definition).unwrap();
        assert_eq!(record.revision, 1);
        original_hash = record.hash;
        let changed = Definition::parse(&SAMPLE.replace("cpu: 4", "cpu: 8")).unwrap();
        assert_eq!(store.create(&changed).unwrap_err(), Error::Conflict);
    }
    let reopened = Store::open(&path).unwrap();
    assert_eq!(
        reopened.get("kernel-lab").unwrap().unwrap().hash,
        original_hash
    );
    assert_eq!(reopened.list().unwrap().len(), 1);
    assert!(reopened.get("' OR 1=1 --").unwrap().is_none());
}

#[test]
fn concurrent_duplicate_creates_have_one_winner() {
    let store = Store::memory().unwrap();
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let store = store.clone();
            std::thread::spawn(move || store.create(&Definition::parse(SAMPLE).unwrap()))
        })
        .collect();
    let mut successes = 0;
    for worker in workers {
        match worker.join().unwrap() {
            Ok(_) => successes += 1,
            Err(error) => assert_eq!(error, Error::Conflict),
        }
    }
    assert_eq!(successes, 1);
    assert_eq!(store.list().unwrap().len(), 1);
}

#[test]
fn newer_database_schema_is_refused_without_downgrade() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("newer.sqlite");
    rusqlite::Connection::open(&path)
        .unwrap()
        .pragma_update(None, "user_version", 99)
        .unwrap();
    assert!(Store::open(&path).is_err());
    let version: u32 = rusqlite::Connection::open(path)
        .unwrap()
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(version, 99);
}

async fn json(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 100_000).await.unwrap()).unwrap()
}

#[tokio::test]
async fn api_saves_lists_and_reads_definitions_without_a_provider() {
    let app = router();
    let create = || {
        Request::builder()
            .method("POST")
            .uri("/api/v1/definitions")
            .header("content-type", "application/yaml")
            .body(Body::from(SAMPLE))
            .unwrap()
    };
    let response = app.clone().oneshot(create()).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let saved = json(response).await;
    assert_eq!(saved["name"], "kernel-lab");
    assert_eq!(saved["revision"], 1);
    assert_eq!(
        app.clone().oneshot(create()).await.unwrap().status(),
        StatusCode::CONFLICT
    );
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/definitions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(json(response).await.as_array().unwrap().len(), 1);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/definitions/kernel-lab")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(json(response).await, saved);
}

#[tokio::test]
async fn validation_and_rejected_requests_never_save_definitions() {
    let app = router();
    for (path, content_type, origin, body, expected) in [
        (
            "/api/v1/definitions/validate",
            "application/yaml",
            false,
            SAMPLE.to_string(),
            StatusCode::OK,
        ),
        (
            "/api/v1/definitions",
            "text/plain",
            false,
            SAMPLE.to_string(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
        (
            "/api/v1/definitions",
            "application/yaml",
            true,
            SAMPLE.to_string(),
            StatusCode::FORBIDDEN,
        ),
        (
            "/api/v1/definitions",
            "application/yaml",
            false,
            SAMPLE.replace("cpu: 4", "cpu: 0"),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            "/api/v1/definitions",
            "application/yaml",
            false,
            "x".repeat(65537),
            StatusCode::PAYLOAD_TOO_LARGE,
        ),
    ] {
        let mut request = Request::builder()
            .method("POST")
            .uri(path)
            .header("content-type", content_type);
        if origin {
            request = request.header("origin", "https://untrusted.invalid");
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::from(body)).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/definitions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(json(response).await, serde_json::json!([]));
}

#[tokio::test]
async fn definition_named_validate_can_be_read_without_changing_validation_behavior() {
    let app = router();
    let read = || {
        Request::get("/api/v1/definitions/validate")
            .body(Body::empty())
            .unwrap()
    };
    let missing = app.clone().oneshot(read()).await.unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(json(missing).await["error"]["code"], "definition_not_found");

    let yaml = SAMPLE.replace("name: kernel-lab", "name: validate");
    let post = |path: &str| {
        Request::post(path)
            .header("content-type", "application/yaml")
            .body(Body::from(yaml.clone()))
            .unwrap()
    };
    let validated = app
        .clone()
        .oneshot(post("/api/v1/definitions/validate"))
        .await
        .unwrap();
    assert_eq!(validated.status(), StatusCode::OK);
    let validated = json(validated).await;
    // Validation must not create the definition, including for this route name.
    assert_eq!(
        app.clone().oneshot(read()).await.unwrap().status(),
        StatusCode::NOT_FOUND
    );
    let saved = app
        .clone()
        .oneshot(post("/api/v1/definitions"))
        .await
        .unwrap();
    assert_eq!(saved.status(), StatusCode::CREATED);
    let saved = json(saved).await;
    assert_eq!(saved["name"], "validate");
    assert_eq!(saved["hash"], validated["hash"]);
    let loaded = app.oneshot(read()).await.unwrap();
    assert_eq!(loaded.status(), StatusCode::OK);
    assert_eq!(json(loaded).await, saved);
}
