#[test]
fn invalid_restore_capability_refuses_startup_before_database_creation_without_echoing_input() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("not-created.sqlite");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_vmanyware-server"))
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("VMANYWARE_RESTORES_ENABLED", "private-invalid-value")
        .env("VMANYWARE_DATABASE", &database)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let logs = String::from_utf8_lossy(&output.stderr);
    assert!(logs.contains("invalid_restore_configuration"));
    assert!(!logs.contains("private-invalid-value"));
    assert!(!database.exists());
}

#[test]
fn actual_server_worker_applies_restore_capability_to_retained_intents() {
    use vmanyware_server::{
        definition::Definition,
        instance::Registration,
        operation::{Failure, Status},
        snapshot_restore::{Approval, Request},
        store::Store,
    };
    struct OwnedChild(std::process::Child);
    impl Drop for OwnedChild {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    for enabled in [None, Some("true")] {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("owned.sqlite");
        let store = Store::open(&database).unwrap();
        let definition = Definition::parse(include_str!("../../examples/kernel-lab.yaml")).unwrap();
        store.create(&definition).unwrap();
        let instance = store
            .register_instance(
                &Registration {
                    definition_name: definition.metadata.name.clone(),
                    definition_hash: definition.hash().unwrap(),
                    node: "fixture-node".into(),
                    vmid: 202,
                },
                &"a".repeat(64),
            )
            .unwrap();
        let approval = Approval {
            instance_id: instance.summary.id,
            provider_scope_id: "a".repeat(64),
            definition_hash: definition.hash().unwrap(),
            node: "fixture-node".into(),
            vmid: 202,
            config_digest: "b".repeat(40),
            request: Request {
                snapshot_name: "before-work".into(),
            },
            snapshot_config_hash: "c".repeat(64),
            acknowledge_data_loss: true,
            confirmed: true,
        };
        let operation = store.enqueue_restore("owned-intent", &approval).unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_vmanyware-server"));
        command
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("VMANYWARE_DATABASE", &database)
            .env("VMANYWARE_PORT", port.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if let Some(enabled) = enabled {
            command.env("VMANYWARE_RESTORES_ENABLED", enabled);
        }
        let mut child = OwnedChild(command.spawn().unwrap());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let result = store.operation(&operation.id).unwrap().unwrap();
            if result.status == Status::Failed {
                assert_eq!(
                    result.failure,
                    Some(if enabled.is_some() {
                        Failure::ProviderNotConfigured
                    } else {
                        Failure::RestoreDisabled
                    })
                );
                break;
            }
            assert!(
                child.0.try_wait().unwrap().is_none(),
                "Owned server exited before processing its journal"
            );
            assert!(
                std::time::Instant::now() < deadline,
                "Owned server did not service the restore queue"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        // The same production configuration must reach the HTTP router too.
        // A configured provider is deliberately absent, so this cannot mutate a VM.
        use std::io::{Read, Write};
        let body = serde_json::to_string(&approval).unwrap();
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();
        write!(stream, "POST /api/v1/environments/{}/snapshots/before-work/restore HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Type: application/json\r\nIdempotency-Key: owned-http\r\nContent-Length: {}\r\n\r\n{}", approval.instance_id, body.len(), body).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(
            response.starts_with(if enabled.is_some() {
                "HTTP/1.1 503"
            } else {
                "HTTP/1.1 403"
            }),
            "Production router did not receive restore capability"
        );
        assert_eq!(store.recent_operations().unwrap().len(), 1);
        assert!(
            !store
                .restore_job(&operation.id)
                .unwrap()
                .unwrap()
                .dispatch_started
        );
    }
}
