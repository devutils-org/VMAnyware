//! Complete local API/worker journey against an owned generated-HTTPS provider.
use super::*;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

async fn provider_fixture() -> Fixture {
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
    let (sender, requests) = mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let clone_upid = "UPID:pve-test:00000001:00000002:00000003:qmclone:9000:fixture@pve:";
        let start_upid = "UPID:pve-test:00000001:00000002:00000003:qmstart:202:fixture@pve:";
        let mut target = serde_json::json!({"name":"kernel-lab","template":0,"digest":"d".repeat(40),"scsi0":"local-lvm:vm-202-disk-0,size=8G","ide2":"local-lvm:vm-202-cloudinit,media=cdrom,size=4M","net0":"virtio=BC:24:11:12:34:56,bridge=old-br,firewall=1"});
        let mut running = false;
        let mut snapshots = Vec::<serde_json::Value>::new();
        while let Ok((socket, _)) = listener.accept().await {
            let Ok(mut stream) = acceptor.accept(socket).await else {
                continue;
            };
            let mut request = Vec::new();
            let mut bytes = [0; 2048];
            let end = loop {
                let n = stream.read(&mut bytes).await.unwrap_or(0);
                if n == 0 {
                    break None;
                }
                request.extend_from_slice(&bytes[..n]);
                if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    break Some(end);
                }
                if request.len() > 16384 {
                    break None;
                }
            };
            let Some(end) = end else {
                continue;
            };
            let length = String::from_utf8_lossy(&request[..end])
                .lines()
                .find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    if key.eq_ignore_ascii_case("content-length") {
                        value.trim().parse::<usize>().ok()
                    } else {
                        None
                    }
                })
                .unwrap_or(0);
            while request.len() < end + 4 + length && request.len() < 16384 {
                let n = stream.read(&mut bytes).await.unwrap_or(0);
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&bytes[..n]);
            }
            let request = String::from_utf8_lossy(&request).into_owned();
            let mut words = request.split_whitespace();
            let method = words.next().unwrap_or("");
            let path = words.next().unwrap_or("");
            let body = request.split_once("\r\n\r\n").unwrap().1;
            let form: std::collections::BTreeMap<_, _> = form_urlencoded::parse(body.as_bytes())
                .into_owned()
                .collect();
            let data = if path.ends_with("/version") {
                serde_json::json!({"version":"9.2.2","release":"fixture"})
            } else if path.ends_with("/nodes") {
                serde_json::json!([{"node":"pve-test","status":"online"}])
            } else if path.ends_with("/qemu") {
                serde_json::json!([{"vmid":9000,"name":"fixture-template","status":"stopped","template":1}])
            } else if path.contains("/qemu/9000/config") {
                serde_json::json!({"template":1,"digest":"c".repeat(40),"scsi0":"local-lvm:base-9000-disk-0,size=8G","ide2":"local-lvm:vm-9000-cloudinit,media=cdrom"})
            } else if path.ends_with("/storage") {
                serde_json::json!([{"storage":"local-lvm","enabled":1,"active":1,"content":"images","avail":1099511627776_u64}])
            } else if path.ends_with("/network") {
                serde_json::json!([{"iface":"vmbr0","active":1,"type":"bridge"}])
            } else if path.contains("/cluster/nextid") {
                serde_json::json!(202)
            } else if path.ends_with("/pending") {
                serde_json::json!([])
            } else if path.ends_with("/qemu/9000/status/current") {
                serde_json::json!({"vmid":9000,"status":"stopped"})
            } else if path.ends_with("/qemu/9000/clone") && method == "POST" {
                target["description"] = serde_json::json!(form["description"]);
                serde_json::json!(clone_upid)
            } else if path.contains("/tasks/") {
                let upid = if path.contains("qmclone") {
                    clone_upid
                } else if path.contains("resize") {
                    RESIZE_TASK
                } else if path.contains("qmsnapshot") {
                    "UPID:pve-test:00000001:00000002:00000003:qmsnapshot:202:fixture@pve:"
                } else {
                    start_upid
                };
                serde_json::json!({"node":"pve-test","upid":upid,"status":"stopped","exitstatus":"OK"})
            } else if path.contains("/qemu/202/config") && method == "GET" {
                target.clone()
            } else if path.ends_with("/qemu/202/resize") && method == "PUT" {
                assert_eq!(form["size"], "40G");
                target["scsi0"] = serde_json::json!("local-lvm:vm-202-disk-0,size=40G");
                target["digest"] = serde_json::json!("a".repeat(40));
                serde_json::json!(RESIZE_TASK)
            } else if path.ends_with("/qemu/202/config") && method == "PUT" {
                if form.contains_key("ciupgrade") {
                    for key in ["net0", "ipconfig0", "citype", "sshkeys"] {
                        target[key] = serde_json::json!(form[key]);
                    }
                    target["ciupgrade"] = serde_json::json!(0);
                    target.as_object_mut().unwrap().remove("ciuser");
                    target["digest"] = serde_json::json!("f".repeat(40));
                } else {
                    target["tags"] = serde_json::json!(form["tags"]);
                    for key in ["sockets", "cores", "vcpus", "cpulimit", "memory", "balloon"] {
                        target[key] = serde_json::json!(form[key].parse::<u64>().unwrap());
                    }
                    target["digest"] = serde_json::json!("e".repeat(40));
                }
                serde_json::Value::Null
            } else if path.ends_with("/qemu/202/status/current") {
                serde_json::json!({"vmid":202,"status":if running{"running"}else{"stopped"},"qmpstatus":if running{"running"}else{"stopped"}})
            } else if path.ends_with("/cloudinit/dump?type=user") {
                serde_json::json!(seed_user(true).replace("user: {name: debian}\n", ""))
            } else if path.ends_with("/cloudinit/dump?type=network") {
                serde_json::json!(SEED_NETWORK)
            } else if path.ends_with("/cloudinit") && method == "PUT" {
                serde_json::Value::Null
            } else if path.ends_with("/status/start") && method == "POST" {
                running = true;
                serde_json::json!(start_upid)
            } else if path.ends_with("/agent/network-get-interfaces") {
                guest_network()["data"].clone()
            } else if path.ends_with("/agent/exec") && method == "POST" {
                let command = serde_json::from_str::<serde_json::Value>(body).unwrap();
                if command
                    == serde_json::json!({"command":["/usr/bin/cloud-init","status","--format","json"]})
                {
                    serde_json::json!({"pid":42})
                } else {
                    assert_eq!(command["command"][0], "/usr/bin/timeout");
                    assert!(command["command"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|v| v == "build-essential"));
                    serde_json::json!({"pid":43})
                }
            } else if path.ends_with("/agent/exec-status?pid=43") {
                serde_json::json!({"exited":1,"exitcode":0})
            } else if path.ends_with("/feature?feature=snapshot") {
                serde_json::json!({"hasFeature":1})
            } else if path.ends_with("/qemu/202/snapshot") && method == "POST" {
                snapshots.push(serde_json::json!({"name":form["snapname"],"description":form["description"],"vmstate":0,"snaptime":1700000000}));
                target["digest"] = serde_json::json!("b".repeat(40));
                serde_json::json!(
                    "UPID:pve-test:00000001:00000002:00000003:qmsnapshot:202:fixture@pve:"
                )
            } else if path.ends_with("/qemu/202/snapshot") {
                serde_json::json!(snapshots)
            } else if path.ends_with("/agent/exec-status?pid=42") {
                serde_json::json!({"exited":1,"exitcode":0,"out-data":clean_cloud_status().to_string()})
            } else {
                panic!("Unexpected fixture request: {method} {path}");
            };
            let body = serde_json::json!({"data":data}).to_string();
            let response=format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
            sender.send(request).unwrap();
            stream.write_all(response.as_bytes()).await.unwrap();
            let _ = stream.shutdown().await;
        }
    });
    Fixture {
        endpoint,
        ca,
        requests,
        task,
    }
}
#[tokio::test]
async fn confirmed_create_api_runs_production_worker_to_clean_guest_with_restart_and_no_replay() {
    run_confirmed_flow(false, false, false).await;
}

#[tokio::test]
async fn confirmed_recreation_api_runs_worker_with_fresh_identity_retained_source_and_no_replay() {
    run_confirmed_flow(true, false, false).await;
}

async fn run_confirmed_flow(recreation: bool, packages: bool, snapshot: bool) {
    use crate::{operation::Status, provisioning_intent::Approval};
    for disk_gib in [8, 40] {
        let mut fixture = provider_fixture().await;
        let provider = fixture.client();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("flow.sqlite");
        let mut store = crate::store::Store::open(&path).unwrap();
        let mut definition = crate::definition::Definition::parse(include_str!(
            "../../../../examples/kernel-lab.yaml"
        ))
        .unwrap();
        definition.spec.resources.disk_gib = disk_gib;
        if !packages {
            definition.spec.provisioning.packages.clear();
        }
        definition.spec.lifecycle.snapshot_after_provisioning = snapshot;
        store.create(&definition).unwrap();
        let catalog=crate::provisioning::Catalog::parse(&format!("{}    imageDefaultUser: {{name: debian, templateVersion: debian-13-cloud-20261016, passwordLocked: true, noAuthorizedKeys: true}}\n",include_str!("../../../../examples/templates.yaml"))).unwrap();
        let source = if recreation {
            let instance = store
                .register_instance(
                    &crate::instance::Registration {
                        definition_name: definition.metadata.name.clone(),
                        definition_hash: definition.hash().unwrap(),
                        node: "pve-test".into(),
                        vmid: 202,
                    },
                    &provider.scope_id(),
                )
                .unwrap();
            // Construct only a retired source fixture. Real deletion has separate
            // task/absence acceptance; this test exercises subsequent recreation.
            rusqlite::Connection::open(&path)
                .unwrap()
                .execute(
                    "UPDATE instances SET status='deleted' WHERE id=?1",
                    [&instance.summary.id],
                )
                .unwrap();
            Some(
                serde_json::to_value(store.instance(&instance.summary.id).unwrap().unwrap())
                    .unwrap(),
            )
        } else {
            None
        };
        let plan = catalog.plan(&definition, 202).unwrap();
        let approval = Approval {
            definition_name: "kernel-lab".into(),
            definition_hash: plan.definition_hash,
            provider_scope_id: provider.scope_id(),
            plan_hash: plan.plan_hash,
            target_node: "pve-test".into(),
            target_vmid: 202,
            source_vmid: 9000,
            storage: "local-lvm".into(),
            bridge: "vmbr0".into(),
            confirmed: true,
        };
        let request = || {
            let (route, body) = if let Some(source) = &source {
                (
                    format!(
                        "/api/v1/environments/{}/recreate",
                        source["id"].as_str().unwrap()
                    ),
                    serde_json::json!({"sourceInstanceId":source["id"],"approval":approval})
                        .to_string(),
                )
            } else {
                (
                    "/api/v1/provisioning/create".into(),
                    serde_json::to_string(&approval).unwrap(),
                )
            };
            Request::post(route)
                .header("content-type", "application/json")
                .header("idempotency-key", "complete-flow")
                .header(
                    "authorization",
                    "Bearer owned-declared-flow-token-0000000000",
                )
                .body(Body::from(body))
                .unwrap()
        };
        let app = crate::router_with_snapshots(
            Some(provider.clone()),
            store.clone(),
            Some(catalog.clone()),
            false,
            true,
            snapshot,
        );
        let app = crate::auth::protect(
            app,
            Some(crate::auth::AccessToken::parse("owned-declared-flow-token-0000000000").unwrap()),
        );
        let response = app.oneshot(request()).await.unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let operation: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
        let id = operation["id"].as_str().unwrap().to_owned();
        if let Some(source) = &source {
            assert_ne!(operation["instanceId"], source["id"]);
            let target = store
                .instance(operation["instanceId"].as_str().unwrap())
                .unwrap()
                .unwrap();
            assert_eq!(target.definition_yaml, source["definitionYaml"]);
            assert_eq!(target.summary.definition_hash, source["definitionHash"]);
            assert_eq!(
                target.summary.definition_revision,
                source["definitionRevision"]
            );
        }
        assert!(fixture.requests.try_recv().is_err());
        let inventory = store.enqueue_inventory("fair-inventory").unwrap();
        for round in 0..12 {
            assert!(crate::worker::run_snapshot_round(
                &store,
                Some(&provider),
                Some(&catalog),
                false,
                true,
                snapshot
            )
            .await
            .unwrap());
            if round == 0 {
                assert_eq!(
                    store.operation(&inventory.id).unwrap().unwrap().status,
                    Status::Succeeded
                );
                drop(store);
                store = crate::store::Store::open(&path).unwrap();
                store.recover_interrupted_operations().unwrap();
            }
            if store.operation(&id).unwrap().unwrap().status == Status::Succeeded {
                break;
            }
        }
        let completed = store.operation(&id).unwrap().unwrap();
        assert_eq!(completed.status, Status::Succeeded, "{completed:?}");
        assert_eq!(completed.result.as_ref().unwrap()["guestReady"], true);
        assert_eq!(
            completed.result.as_ref().unwrap()["guest"]["ipv4"],
            "192.0.2.42"
        );
        assert!(store
            .provisioning_job(&id)
            .unwrap()
            .unwrap()
            .stages
            .iter()
            .all(|s| s.status == Status::Succeeded));
        let job = store.provisioning_job(&id).unwrap().unwrap();
        assert_eq!(job.installed_packages().is_some(), packages);
        assert_eq!(job.stages.iter().filter(|s|s.kind=="snapshot_after_provisioning" && s.status==Status::Succeeded).count(),usize::from(snapshot));
        // The progress read must work from retained data with provider/catalog/writes absent.
        let progress_app = crate::auth::protect(
            crate::router_with_services(None, store.clone()),
            Some(crate::auth::AccessToken::parse("owned-declared-flow-token-0000000000").unwrap()),
        );
        let response = progress_app
            .oneshot(
                Request::get(format!("/api/v1/operations/{id}/provisioning"))
                    .header(
                        "authorization",
                        "Bearer owned-declared-flow-token-0000000000",
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let progress: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
        assert_eq!(progress["operationId"], id);
        assert_eq!(
            progress["instanceId"],
            completed.instance_id.as_ref().unwrap().as_str()
        );
        assert_eq!(progress["status"], "succeeded");
        assert_eq!(progress["planHash"], approval.plan_hash);
        assert_eq!(
            progress["sourceInstanceId"],
            source
                .as_ref()
                .map(|s| s["id"].clone())
                .unwrap_or(serde_json::Value::Null)
        );
        assert!(progress["stages"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["status"] == "succeeded"));
        for private in [
            "UPID:",
            "approval",
            "requestKey",
            "journal",
            "pid",
            "resultJson",
        ] {
            assert!(!progress.to_string().contains(private));
        }
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        for request in &requests {
            assert!(request
                .to_ascii_lowercase()
                .contains("authorization: pveapitoken=reader@pve!vmanyware=fake-test-secret\r\n"));
        }
        if let Some(source) = &source {
            let clone = requests
                .iter()
                .find(|request| {
                    request.starts_with("POST ") && request.contains("/qemu/9000/clone ")
                })
                .unwrap();
            let fields: std::collections::BTreeMap<_, _> =
                form_urlencoded::parse(clone.split_once("\r\n\r\n").unwrap().1.as_bytes())
                    .into_owned()
                    .collect();
            let target_id = operation["instanceId"].as_str().unwrap();
            assert!(fields["description"].contains(&format!("VMAnyware instance {target_id}\n")));
            assert!(!fields["description"].contains(source["id"].as_str().unwrap()));
            assert_eq!(fields["newid"], "202");
            assert_eq!(
                requests
                    .iter()
                    .filter(|request| !request.starts_with("GET "))
                    .count(),
                6 + usize::from(disk_gib == 40) + usize::from(packages) + usize::from(snapshot)
            );
            assert_eq!(
                store.get("kernel-lab").unwrap().unwrap().yaml,
                source["definitionYaml"]
            );
        }
        for (method, suffix) in [
            ("POST", "/qemu/9000/clone"),
            ("PUT", "/qemu/202/cloudinit"),
            ("POST", "/status/start"),
            ("POST", "/agent/exec"),
        ] {
            assert_eq!(
                requests
                    .iter()
                    .filter(|r| r.starts_with(method)
                        && r.split_whitespace().nth(1).unwrap().ends_with(suffix))
                    .count(),
                1 + usize::from(suffix == "/agent/exec" && packages),
                "{suffix}"
            );
        }
        assert_eq!(
            requests
                .iter()
                .filter(|r| r.starts_with("PUT ") && r.contains("/qemu/202/config "))
                .count(),
            2
        );
        assert_eq!(
            requests
                .iter()
                .filter(|r| r.starts_with("PUT ") && r.contains("/qemu/202/resize "))
                .count(),
            usize::from(disk_gib == 40)
        );
        let app = crate::router_with_snapshots(
            Some(provider.clone()),
            store.clone(),
            Some(catalog.clone()),
            false,
            true,
            snapshot,
        );
        let app = crate::auth::protect(
            app,
            Some(crate::auth::AccessToken::parse("owned-declared-flow-token-0000000000").unwrap()),
        );
        let response = app.oneshot(request()).await.unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        assert!(!crate::worker::run_snapshot_round(
            &store,
            Some(&provider),
            Some(&catalog),
            false,
            true,
            snapshot
        )
        .await
        .unwrap());
        assert!(fixture.requests.try_recv().is_err());
        if let Some(source) = &source {
            let source_id = source["id"].as_str().unwrap();
            let saved = serde_json::to_value(store.operation(&id).unwrap().unwrap()).unwrap();
            drop(store);
            let store = crate::store::Store::open(&path).unwrap();
            let app = crate::router_with_services(None, store.clone());
            let response = app
                .oneshot(
                    Request::get(format!(
                        "/api/v1/environments/{source_id}/recreation-operations/{id}"
                    ))
                    .body(Body::empty())
                    .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let recovered: serde_json::Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap())
                    .unwrap();
            assert_eq!(recovered["operation"], saved);
            assert_eq!(recovered["sourceInstanceId"], source_id);
            assert_eq!(
                serde_json::to_value(store.instance(source_id).unwrap().unwrap()).unwrap(),
                *source
            );
            assert_eq!(
                store
                    .provisioning_job(&id)
                    .unwrap()
                    .unwrap()
                    .recreated_from
                    .as_deref(),
                Some(source_id)
            );
            assert_eq!(store.recent_instances().unwrap().len(), 2);
            assert_eq!(
                store
                    .recent_operations()
                    .unwrap()
                    .iter()
                    .filter(|op| op.kind == "vm_provision")
                    .count(),
                1
            );
            assert!(
                fixture.requests.try_recv().is_err(),
                "Saved recovery contacted provider"
            );
        }
    }
}

#[tokio::test]
async fn declared_create_api_runs_packages_and_final_snapshot_combinations() {
    for (packages, snapshot) in [(true, false), (false, true), (true, true)] {
        run_confirmed_flow(false, packages, snapshot).await;
    }
}
#[tokio::test]
async fn declared_recreation_api_runs_packages_and_final_snapshot_combinations() {
    for (packages, snapshot) in [(true, false), (false, true), (true, true)] {
        run_confirmed_flow(true, packages, snapshot).await;
    }
}
