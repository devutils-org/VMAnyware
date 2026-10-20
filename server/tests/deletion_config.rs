//! Owned process/HTTPS fixtures only; never contacts configured infrastructure.
use serde_json::{json, Value};
use std::{
    path::Path,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use vmanyware_server::{
    definition::Definition,
    deletion::{Approval, Plan},
    instance::Registration,
    operation::{Failure, Status},
    proxmox::Proxmox,
    store::Store,
};
const UPID: &str = "UPID:pve-lab:00000001:00000002:00000003:qmdestroy:202:fixture@pve:";
const ACCESS: &str = "fixture-only-access-token-0123456789abcdef";
struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn command(database: &Path, port: u16) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vmanyware-server"));
    command
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("VMANYWARE_DATABASE", database)
        .env("VMANYWARE_PORT", port.to_string())
        .env("VMANYWARE_ACCESS_TOKEN", ACCESS)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}
fn port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}
fn register(store: &Store, scope: &str, vmid: u32) -> vmanyware_server::instance::Instance {
    let definition = Definition::parse(include_str!("../../examples/kernel-lab.yaml")).unwrap();
    if store.get(&definition.metadata.name).unwrap().is_none() {
        store.create(&definition).unwrap();
    }
    store
        .register_instance(
            &Registration {
                definition_name: definition.metadata.name.clone(),
                definition_hash: definition.hash().unwrap(),
                node: "pve-lab".into(),
                vmid,
            },
            scope,
        )
        .unwrap()
}
fn minimal_approval(store: &Store) -> Approval {
    let instance = register(store, &"a".repeat(64), 202);
    serde_json::from_value(json!({"instanceId":instance.summary.id,"providerScopeId":"a".repeat(64),"definitionHash":instance.summary.definition_hash,"node":"pve-lab","vmid":202,"configDigest":"b".repeat(40),"confirmed":true,"acknowledgeDataLoss":true,"plan":{"scopeHash":"c".repeat(64),"volumes":[],"snapshots":[],"preservedMedia":[],"preservedOwnedVolumes":[],"effects":{"version":1,"removeVmConfiguration":true,"removeSnapshotMetadata":true,"removeVmPermissions":true,"removeVmFirewall":true,"cleanupIpam":true,"cleanupRecordedFleecingImages":true,"purgeJobs":false,"destroyUnreferencedDisks":false,"retainDefinitionAndHistory":true}}})).unwrap()
}
#[test]
fn invalid_deletion_configuration_refuses_startup_without_creating_database_or_echoing_input() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("not-created.sqlite");
    let output = Command::new(env!("CARGO_BIN_EXE_vmanyware-server"))
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("VMANYWARE_DATABASE", &database)
        .env("VMANYWARE_DELETIONS_ENABLED", "private-invalid-value")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let logs = String::from_utf8_lossy(&output.stderr);
    assert!(logs.contains("invalid_deletion_configuration"));
    assert!(!logs.contains("private-invalid-value"));
    assert!(!database.exists());
}
async fn wait_for(
    store: &Store,
    id: &str,
    child: &mut OwnedChild,
    status: Status,
) -> vmanyware_server::operation::Operation {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let operation = store.operation(id).unwrap().unwrap();
        if operation.status == status {
            return operation;
        }
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "Owned server exited prematurely"
        );
        assert!(
            Instant::now() < deadline,
            "Owned server did not process deletion: {:?} {:?}",
            operation.status,
            operation.failure
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
#[tokio::test]
async fn actual_worker_uses_independent_default_off_deletion_capability() {
    for enabled in [None, Some("false"), Some("true")] {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("flags.sqlite");
        let store = Store::open(&database).unwrap();
        let approval = minimal_approval(&store);
        let operation = store
            .enqueue_deletion("owned-flag-intent", &approval)
            .unwrap();
        let mut command = command(&database, port());
        // Every other write capability is on: none may implicitly authorize deletion.
        for key in [
            "VMANYWARE_LIFECYCLE_ENABLED",
            "VMANYWARE_PROVISIONING_ENABLED",
            "VMANYWARE_SNAPSHOTS_ENABLED",
            "VMANYWARE_RESTORES_ENABLED",
        ] {
            command.env(key, "true");
        }
        if let Some(value) = enabled {
            command.env("VMANYWARE_DELETIONS_ENABLED", value);
        }
        let mut child = OwnedChild(command.spawn().unwrap());
        let failed = wait_for(&store, &operation.id, &mut child, Status::Failed).await;
        assert_eq!(
            failed.failure,
            Some(if enabled == Some("true") {
                Failure::ProviderNotConfigured
            } else {
                Failure::DeletionDisabled
            })
        );
        assert!(
            !store
                .deletion_job(&operation.id)
                .unwrap()
                .unwrap()
                .dispatch_started
        );
    }
}
#[tokio::test]
async fn deletion_round_services_earlier_queues_and_delete_without_short_circuit_starvation() {
    let store = Store::memory().unwrap();
    let deletion = minimal_approval(&store);
    let read = store.enqueue_inventory("fair-read").unwrap();
    let delete = store.enqueue_deletion("fair-delete", &deletion).unwrap();
    let mut operations = vec![
        (read.id, Failure::ProviderNotConfigured),
        (delete.id, Failure::DeletionDisabled),
    ];
    for (vmid, kind) in [(203, "lifecycle"), (204, "snapshot"), (205, "restore")] {
        let instance = register(&store, &deletion.provider_scope_id, vmid);
        let target = vmanyware_server::lifecycle::Approval {
            action: vmanyware_server::lifecycle::Action::Start,
            instance_id: instance.summary.id.clone(),
            provider_scope_id: deletion.provider_scope_id.clone(),
            definition_hash: deletion.definition_hash.clone(),
            node: "pve-lab".into(),
            vmid,
            config_digest: "b".repeat(40),
            confirmed: true,
        };
        let (operation, failure) = match kind {
            "lifecycle" => (
                store.enqueue_lifecycle("fair-lifecycle", &target).unwrap(),
                Failure::LifecycleDisabled,
            ),
            "snapshot" => (
                store
                    .enqueue_snapshot(
                        "fair-snapshot",
                        &vmanyware_server::snapshot::Approval {
                            instance_id: target.instance_id,
                            provider_scope_id: target.provider_scope_id,
                            definition_hash: target.definition_hash,
                            node: target.node,
                            vmid,
                            config_digest: target.config_digest,
                            request: vmanyware_server::snapshot::CreateRequest {
                                name: "before-work".into(),
                                description: "Before".into(),
                            },
                            confirmed: true,
                        },
                    )
                    .unwrap(),
                Failure::SnapshotDisabled,
            ),
            _ => (
                store
                    .enqueue_restore(
                        "fair-restore",
                        &vmanyware_server::snapshot_restore::Approval {
                            instance_id: target.instance_id,
                            provider_scope_id: target.provider_scope_id,
                            definition_hash: target.definition_hash,
                            node: target.node,
                            vmid,
                            config_digest: target.config_digest,
                            request: vmanyware_server::snapshot_restore::Request {
                                snapshot_name: "before-work".into(),
                            },
                            snapshot_config_hash: "c".repeat(64),
                            confirmed: true,
                            acknowledge_data_loss: true,
                        },
                    )
                    .unwrap(),
                Failure::RestoreDisabled,
            ),
        };
        operations.push((operation.id, failure));
    }
    assert!(vmanyware_server::worker::run_deletion_round(
        &store, None, None, false, false, false, false, false
    )
    .await
    .unwrap());
    for (id, failure) in operations {
        let operation = store.operation(&id).unwrap().unwrap();
        assert_eq!(operation.status, Status::Failed);
        assert_eq!(operation.failure, Some(failure));
    }
    assert!(!vmanyware_server::worker::run_deletion_round(
        &store, None, None, false, false, false, false, false
    )
    .await
    .unwrap());
}

struct ProviderState {
    instance_id: String,
    deleted: bool,
    task_running: bool,
    requests: Vec<String>,
}
struct Provider {
    endpoint: String,
    ca: Vec<u8>,
    state: Arc<Mutex<ProviderState>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Provider {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Provider {
    async fn new() -> Self {
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };
        use tokio_rustls::{rustls, TlsAcceptor};
        let key = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let ca = key.cert.pem().into_bytes();
        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![key.cert.der().clone()],
                rustls::pki_types::PrivatePkcs8KeyDer::from(key.signing_key.serialize_der()).into(),
            )
            .unwrap();
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!(
            "https://localhost:{}",
            listener.local_addr().unwrap().port()
        );
        let state = Arc::new(Mutex::new(ProviderState {
            instance_id: String::new(),
            deleted: false,
            task_running: false,
            requests: vec![],
        }));
        let shared = state.clone();
        let task = tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                let Ok(mut stream) = acceptor.accept(socket).await else {
                    continue;
                };
                let mut request = vec![];
                let mut buffer = [0; 1024];
                while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                    let Ok(n) = stream.read(&mut buffer).await else {
                        break;
                    };
                    if n == 0 || request.len() > 8192 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..n]);
                }
                let request = String::from_utf8(request).unwrap();
                let (status, body) = {
                    let mut state = shared.lock().unwrap();
                    state.requests.push(request.clone());
                    assert!(request.to_ascii_lowercase().contains(
                        "authorization: pveapitoken=fixture@pve!delete=fixture-only-secret\r\n"
                    ));
                    let mut first = request.split_whitespace();
                    let method = first.next().unwrap();
                    let path = first.next().unwrap();
                    let mut status = "200 OK";
                    let data=match (method,path) {
                        ("GET","/api2/json/access/permissions?path=%2Fvms%2F202")=>json!({"/vms/202":{"VM.Audit":0}}),
                        ("GET","/api2/json/access/permissions?path=%2Fstorage%2Fstorage")=>json!({"/storage/storage":{"Datastore.Allocate":0}}),
                        ("GET","/api2/json/cluster/ha/resources"|"/api2/json/cluster/replication"|"/api2/json/nodes/pve-lab/qemu/202/pending")=>json!([]),
                        ("GET","/api2/json/nodes/pve-lab/qemu/202/config?current=1") if !state.deleted=>json!({"digest":"b".repeat(40),"tags":format!("vmanyware-{}",state.instance_id),"template":0,"scsi0":"storage:vm-202-disk-0,size=8G"}),
                        ("GET","/api2/json/nodes/pve-lab/qemu/202/snapshot") if !state.deleted=>json!([{"name":"current"}]),
                        ("GET","/api2/json/nodes/pve-lab/qemu/202/status/current") if !state.deleted=>json!({"vmid":202,"status":"stopped"}),
                        ("GET","/api2/json/nodes/pve-lab/storage/storage/content?content=images&vmid=202"|"/api2/json/nodes/pve-lab/storage/storage/content?content=images")=>if state.deleted{json!([])}else{json!([{"volid":"storage:vm-202-disk-0","vmid":202,"format":"raw"}])},
                        ("GET","/api2/json/cluster/nextid?vmid=202") if state.deleted=>json!(202),
                        ("GET",value) if value==format!("/api2/json/nodes/pve-lab/tasks/{UPID}/status")=>json!({"node":"pve-lab","upid":UPID,"status":if state.task_running{"running"}else{"stopped"},"exitstatus":if state.task_running{None}else{Some("OK")}}),
                        ("DELETE","/api2/json/nodes/pve-lab/qemu/202?purge=0&destroy-unreferenced-disks=0") if !state.deleted=>{state.deleted=true;json!(UPID)},
                        _=>{status="404 Not Found";json!(null)},
                    };
                    (status, json!({"data":data}).to_string())
                };
                let response=format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
                let _ = stream.write_all(response.as_bytes()).await;
                let _ = stream.shutdown().await;
            }
        });
        Self {
            endpoint,
            ca,
            state,
            task,
        }
    }
    fn client(&self) -> Proxmox {
        Proxmox::new(
            &self.endpoint,
            "fixture@pve!delete",
            "fixture-only-secret",
            Some(&self.ca),
        )
        .unwrap()
    }
    fn requests(&self) -> Vec<String> {
        std::mem::take(&mut self.state.lock().unwrap().requests)
    }
}
async fn prepare(store: &Store, provider: &Provider) -> Approval {
    let instance = register(store, &provider.client().scope_id(), 202);
    provider.state.lock().unwrap().instance_id = instance.summary.id.clone();
    let report = instance
        .summary
        .deletion_preflight(&provider.client())
        .await
        .unwrap();
    assert_eq!(
        report.deletion_preconditions,
        vmanyware_server::lifecycle::Status::Passed
    );
    Approval {
        instance_id: instance.summary.id,
        provider_scope_id: provider.client().scope_id(),
        definition_hash: report.definition_hash,
        node: report.node,
        vmid: 202,
        config_digest: report.config_digest,
        confirmed: true,
        acknowledge_data_loss: true,
        plan: Plan {
            scope_hash: report.scope_hash,
            volumes: report.volumes,
            snapshots: report.snapshots,
            preserved_media: report.preserved_media,
            preserved_owned_volumes: report.preserved_owned_volumes,
            effects: report.effects,
        },
    }
}
fn start(
    database: &Path,
    port: u16,
    directory: &Path,
    provider: &Provider,
    enabled: bool,
) -> OwnedChild {
    let ca = directory.join("generated-public-ca.pem");
    std::fs::write(&ca, &provider.ca).unwrap();
    let mut command = command(database, port);
    command
        .env("VMANYWARE_PROXMOX_URL", &provider.endpoint)
        .env("VMANYWARE_PROXMOX_TOKEN_ID", "fixture@pve!delete")
        .env("VMANYWARE_PROXMOX_TOKEN_SECRET", "fixture-only-secret")
        .env("VMANYWARE_PROXMOX_CA", ca)
        .env(
            "VMANYWARE_DELETIONS_ENABLED",
            if enabled { "true" } else { "false" },
        );
    OwnedChild(command.spawn().unwrap())
}
#[tokio::test]
async fn actual_server_deletes_once_and_restarts_known_task_with_get_only_writes_off() {
    for recovery in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("process.sqlite");
        let provider = Provider::new().await;
        let store = Store::open(&database).unwrap();
        let approval = prepare(&store, &provider).await;
        let operation = store
            .enqueue_deletion("owned-real-process", &approval)
            .unwrap();
        provider.state.lock().unwrap().task_running = recovery;
        provider.requests();
        let mut service_port = port();
        let mut child = start(&database, service_port, directory.path(), &provider, true);
        if recovery {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                let job = store.deletion_job(&operation.id).unwrap().unwrap();
                let polled = provider
                    .state
                    .lock()
                    .unwrap()
                    .requests
                    .iter()
                    .any(|request| request.contains(&format!("/tasks/{UPID}/status")));
                if job.provider_task.is_some() && job.operation.status == Status::Running && polled
                {
                    break;
                }
                assert!(child.0.try_wait().unwrap().is_none());
                assert!(
                    Instant::now() < deadline,
                    "First server never retained/polled the running task"
                );
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            // Abruptly stop this owned process in its live task window.
            drop(child);
            let initial = provider.requests();
            assert_eq!(
                initial
                    .iter()
                    .filter(|request| request.starts_with("DELETE "))
                    .count(),
                1
            );
            provider.state.lock().unwrap().task_running = false;
            service_port = port();
            child = start(&database, service_port, directory.path(), &provider, false);
        }
        let completed = wait_for(&store, &operation.id, &mut child, Status::Succeeded).await;
        assert_eq!(
            store
                .instance(&approval.instance_id)
                .unwrap()
                .unwrap()
                .summary
                .status,
            vmanyware_server::instance::Status::Deleted
        );
        assert!(completed.result.as_ref().unwrap()["vmAbsenceVerified"]
            .as_bool()
            .unwrap());
        let requests = provider.requests();
        assert_eq!(
            requests.iter().filter(|r| r.starts_with("DELETE ")).count(),
            usize::from(!recovery)
        );
        if recovery {
            assert_eq!(requests.len(), 7);
            assert!(requests.iter().all(|r| r.starts_with("GET ")));
        }
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let recovered: Value = client
            .get(format!(
                "http://127.0.0.1:{service_port}/api/v1/operations/{}",
                operation.id
            ))
            .bearer_auth(ACCESS)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(recovered["status"], "succeeded");
        assert!(provider.requests().is_empty());
    }
}

#[tokio::test]
async fn actual_router_reports_deletion_flag_without_enqueuing_or_enabling_other_actions() {
    for enabled in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("capability.sqlite");
        let provider = Provider::new().await;
        let store = Store::open(&database).unwrap();
        let approval = prepare(&store, &provider).await;
        provider.requests();
        let service_port = port();
        let mut child = start(
            &database,
            service_port,
            directory.path(),
            &provider,
            enabled,
        );
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        let url=format!("http://127.0.0.1:{service_port}/api/v1/environments/{}/delete-preflight?purge=1&enabled=true",approval.instance_id);
        let deadline = Instant::now() + Duration::from_secs(5);
        let response = loop {
            match client.get(&url).bearer_auth(ACCESS).send().await {
                Ok(response) => break response,
                Err(_) => {
                    assert!(child.0.try_wait().unwrap().is_none());
                    assert!(
                        Instant::now() < deadline,
                        "Owned server did not bind its router"
                    );
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
        };
        assert!(response.status().is_success());
        let report: Value = response.json().await.unwrap();
        assert_eq!(report["executionEnabled"], enabled);
        assert_eq!(report["purge"], false);
        assert_eq!(report["destroyUnreferencedDisks"], false);
        assert_eq!(report["instanceId"], approval.instance_id);
        assert_eq!(report["deletionPreconditions"], "passed");
        assert!(store.recent_operations().unwrap().is_empty());
        assert_eq!(
            store
                .instance(&approval.instance_id)
                .unwrap()
                .unwrap()
                .summary
                .status,
            vmanyware_server::instance::Status::Registered
        );
        let reads = provider.requests();
        assert!(!reads.is_empty() && reads.iter().all(|request| request.starts_with("GET ")));
    }
}
