#[test]
fn invalid_snapshot_capability_refuses_startup_before_database_creation_without_echoing_input() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("not-created.sqlite");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_vmanyware-server"))
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("VMANYWARE_SNAPSHOTS_ENABLED", "private-invalid-value")
        .env("VMANYWARE_DATABASE", &database)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let logs = String::from_utf8_lossy(&output.stderr);
    assert!(logs.contains("invalid_snapshot_configuration"));
    assert!(!logs.contains("private-invalid-value"));
    assert!(!database.exists());
}

#[test]
fn actual_server_worker_applies_snapshot_capability_to_retained_intents() {
    use vmanyware_server::{
        definition::Definition,
        instance::Registration,
        operation::{Failure, Status},
        snapshot::{Approval, CreateRequest},
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
            request: CreateRequest {
                name: "before-work".into(),
                description: "Owned process fixture".into(),
            },
            confirmed: true,
        };
        let operation = store.enqueue_snapshot("owned-intent", &approval).unwrap();
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
            command.env("VMANYWARE_SNAPSHOTS_ENABLED", enabled);
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
                        Failure::SnapshotDisabled
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
                "Owned server did not service the snapshot queue"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            !store
                .snapshot_job(&operation.id)
                .unwrap()
                .unwrap()
                .dispatch_started
        );
    }
}
