//! Fixed create workflow; confirmation is reconstructed from the native retained plan.
use super::{hex, identifier, Profile, RemoteConnection};
use serde::{Deserialize, Serialize};
use serde_json::json;
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    definition_name: String,
    definition_hash: String,
    plan_hash: String,
    provider_scope_id: Option<String>,
    source: Source,
    target: Target,
    creation_enabled: bool,
    ssh_key_count: usize,
    packages: Vec<String>,
    snapshot_after_provisioning: bool,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Source {
    template_vmid: u32,
    template_version: String,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Target {
    node: String,
    vmid: u32,
    storage: String,
    bridge: String,
    guest_user: String,
    resources: Resources,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Resources {
    cpu: u16,
    #[serde(rename = "memoryMiB")]
    memory_mib: u32,
    #[serde(rename = "diskGiB")]
    disk_gib: u32,
}
#[derive(Clone)]
pub(super) struct Prepared {
    key: String,
    plan: Plan,
    submitted: bool,
    inflight: bool,
    operation: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    id: String,
    kind: String,
    #[serde(default)]
    instance_id: Option<String>,
    status: Status,
    #[serde(default)]
    result: Option<Ready>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Queued,
    Running,
    Succeeded,
    Failed,
    Interrupted,
    Uncertain,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Ready {
    instance_id: String,
    provider_scope_id: String,
    plan_hash: String,
    guest_ready: bool,
    guest: Guest,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Guest {
    ipv4: std::net::Ipv4Addr,
    interface: String,
    cloud_init_exit_code: u8,
    cloud_init_clean: bool,
}
#[derive(Deserialize)]
struct Record {
    name: String,
    hash: String,
}
fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 64
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
impl Plan {
    fn valid(&self, vmid: u32) -> bool {
        (!self.definition_name.is_empty()
            && self.definition_name.len() <= 63
            && !self.definition_name.starts_with('-')
            && !self.definition_name.ends_with('-')
            && self
                .definition_name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'))
            && hex(&self.definition_hash, 64)
            && hex(&self.plan_hash, 64)
            && self.provider_scope_id.as_ref().is_some_and(|s| hex(s, 64))
            && identifier(&self.target.node, 63)
            && self.target.vmid == vmid
            && (100..=999999999).contains(&vmid)
            && self.source.template_vmid != vmid
            && (100..=999999999).contains(&self.source.template_vmid)
            && identifier(&self.target.storage, 64)
            && identifier(&self.target.bridge, 15)
            && identifier(&self.target.guest_user, 32)
            && !self.source.template_version.is_empty()
            && self.source.template_version.len() <= 128
            && (1..=64).contains(&self.target.resources.cpu)
            && (256..=262144).contains(&self.target.resources.memory_mib)
            && (4..=4096).contains(&self.target.resources.disk_gib)
            && self.ssh_key_count > 0
            && self.ssh_key_count <= 32
            && self.packages.is_empty()
            && !self.snapshot_after_provisioning
    }
    fn approval(&self) -> serde_json::Value {
        json!({"definitionName":self.definition_name,"definitionHash":self.definition_hash,"providerScopeId":self.provider_scope_id,"planHash":self.plan_hash,"targetNode":self.target.node,"targetVmid":self.target.vmid,"sourceVmid":self.source.template_vmid,"storage":self.target.storage,"bridge":self.target.bridge,"confirmed":true})
    }
}
impl Profile {
    async fn create_response(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<(reqwest::StatusCode, Vec<u8>), String> {
        let mut response=request.header(reqwest::header::AUTHORIZATION,self.authorization.clone()).send().await.map_err(|_|"Server response unavailable. A submitted creation may still be running; retain the request key.")?;
        let status = response.status();
        if status.as_u16() == 401 {
            return Err("The server rejected the access token.".into());
        }
        if response
            .headers()
            .get("x-vmanyware-authenticated")
            .and_then(|h| h.to_str().ok())
            != Some("bearer")
        {
            return Err("The server does not confirm access-token authentication.".into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Server response interrupted; retain the request key.")?
        {
            if bytes.len() + chunk.len() > 256 * 1024 {
                return Err("Server response is too large.".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok((status, bytes))
    }
    async fn create_json<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<T, String> {
        let (status, bytes) = self.create_response(request).await?;
        if !status.is_success() {
            return Err(match status.as_u16(){403=>"Creation is disabled or access was denied.",409=>"Definition, target or request conflicts with retained work. Review the server operation before changing inputs.",422=>"Definition or template features are unsupported. Use empty packages and an attested template.",_=>"The server could not complete this request. Retain the request key if creation was submitted."}.into());
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| "The server returned an invalid creation response.".into())
    }
}
impl RemoteConnection {
    async fn prepare_create(
        &self,
        id: u64,
        key: String,
        yaml: String,
        vmid: u32,
    ) -> Result<Plan, String> {
        if !valid_key(&key)
            || yaml.is_empty()
            || yaml.len() > 65536
            || !(100..=999999999).contains(&vmid)
        {
            return Err("Supply a bounded definition, target VMID and request key.".into());
        }
        let profile = self.snapshot(id)?;
        {
            let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
            if state.generation != id {
                return Err("The connection changed.".into());
            }
            if state.prepared.as_ref().is_some_and(|p| p.submitted) {
                return Err("Resume the submitted request before preparing another target. Disconnecting does not cancel it.".into());
            }
            state.prepared = None;
        }
        let plan: Plan = profile
            .create_json(
                profile
                    .client
                    .post(
                        profile
                            .endpoint
                            .join("api/v1/provisioning/plan")
                            .map_err(|_| "Invalid server address.")?,
                    )
                    .json(&json!({"yaml":yaml,"targetVmid":vmid})),
            )
            .await?;
        self.current(id)?;
        if !plan.valid(vmid) {
            return Err("The server plan is invalid or unsupported for initial creation.".into());
        }
        if !plan.creation_enabled {
            return Err("Creation is disabled on this server or template.".into());
        }
        let (status, bytes) = profile
            .create_response(
                profile
                    .client
                    .post(
                        profile
                            .endpoint
                            .join("api/v1/definitions")
                            .map_err(|_| "Invalid server address.")?,
                    )
                    .header("content-type", "application/yaml")
                    .body(yaml),
            )
            .await?;
        self.current(id)?;
        let record: Record = if status.as_u16() == 409 {
            profile
                .create_json(
                    profile.client.get(
                        profile
                            .endpoint
                            .join(&format!("api/v1/definitions/{}", plan.definition_name))
                            .map_err(|_| "Invalid definition name.")?,
                    ),
                )
                .await?
        } else if status.is_success() {
            serde_json::from_slice(&bytes).map_err(|_| "Invalid saved definition response.")?
        } else {
            return Err("Cannot save this definition on the server.".into());
        };
        self.current(id)?;
        if record.name != plan.definition_name || record.hash != plan.definition_hash {
            return Err("A different definition is already saved with this name.".into());
        }
        let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != id
            || state.profile.is_none()
            || state.prepared.as_ref().is_some_and(|p| p.submitted)
        {
            return Err("The connection or creation changed. Prepare again.".into());
        }
        state.prepared = Some(Prepared {
            key,
            plan: plan.clone(),
            submitted: false,
            inflight: false,
            operation: None,
        });
        Ok(plan)
    }
    async fn confirm_create(
        &self,
        id: u64,
        key: &str,
        confirmed: bool,
    ) -> Result<Operation, String> {
        if !confirmed {
            return Err("Explicitly confirm the displayed target.".into());
        }
        let profile = self.snapshot(id)?;
        let plan = {
            let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
            if state.generation != id {
                return Err("The connection changed.".into());
            }
            let prepared = state
                .prepared
                .as_mut()
                .filter(|p| p.key == key)
                .ok_or("Prepare this request first.")?;
            if prepared.inflight {
                return Err("This request is already being submitted.".into());
            }
            prepared.submitted = true;
            prepared.inflight = true;
            prepared.plan.clone()
        };
        let result: Result<Operation, String> = profile
            .create_json(
                profile
                    .client
                    .post(
                        profile
                            .endpoint
                            .join("api/v1/provisioning/create")
                            .map_err(|_| "Invalid server address.")?,
                    )
                    .header("idempotency-key", key)
                    .json(&plan.approval()),
            )
            .await;
        let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != id {
            return Err(
                "The connection changed. The submitted request may continue on the server.".into(),
            );
        }
        let prepared = state
            .prepared
            .as_mut()
            .filter(|p| p.key == key)
            .ok_or("Creation state unavailable.")?;
        prepared.inflight = false;
        let operation = result?;
        validate_operation(&operation, &plan)?;
        prepared.operation = Some(operation.id.clone());
        Ok(operation)
    }
    async fn create_status(&self, id: u64, key: &str) -> Result<Operation, String> {
        let profile = self.snapshot(id)?;
        let prepared = {
            let state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
            state
                .prepared
                .as_ref()
                .filter(|p| p.key == key)
                .cloned()
                .ok_or("Creation state unavailable.")?
        };
        let operation = prepared
            .operation
            .clone()
            .ok_or("No operation ID was received. Retry the same confirmed request key.")?;
        let result = profile
            .create_json(
                profile.client.get(
                    profile
                        .endpoint
                        .join(&format!("api/v1/operations/{operation}"))
                        .map_err(|_| "Invalid operation ID.")?,
                ),
            )
            .await;
        self.current(id)?;
        let operation: Operation = result?;
        validate_operation(&operation, &prepared.plan)?;
        if Some(&operation.id) != prepared.operation.as_ref() {
            return Err("The server returned a different operation.".into());
        }
        Ok(operation)
    }
}
fn validate_read_operation(operation: &Operation) -> Result<(), String> {
    if !hex(&operation.id, 32)
        || operation.kind != "vm_provision"
        || operation
            .instance_id
            .as_ref()
            .is_some_and(|id| !hex(id, 32))
    {
        return Err("Invalid creation operation.".into());
    }
    if matches!(operation.status, Status::Succeeded) {
        let result = operation
            .result
            .as_ref()
            .ok_or("Missing guest readiness evidence.")?;
        let ip = result.guest.ipv4;
        if !hex(&result.instance_id, 32)
            || operation
                .instance_id
                .as_ref()
                .is_some_and(|id| id != &result.instance_id)
            || !hex(&result.provider_scope_id, 64)
            || !hex(&result.plan_hash, 64)
            || !result.guest_ready
            || !result.guest.cloud_init_clean
            || result.guest.cloud_init_exit_code != 0
            || ip.is_loopback()
            || ip.is_unspecified()
            || ip.is_link_local()
            || ip.is_broadcast()
            || ip.octets()[0] == 0
            || ip.octets()[0] >= 224
            || !identifier(&result.guest.interface, 15)
        {
            return Err("Invalid guest readiness evidence.".into());
        }
    }
    Ok(())
}
fn validate_operation(operation: &Operation, plan: &Plan) -> Result<(), String> {
    validate_read_operation(operation)?;
    if let Some(result) = operation
        .result
        .as_ref()
        .filter(|_| matches!(operation.status, Status::Succeeded))
    {
        if Some(&result.provider_scope_id) != plan.provider_scope_id.as_ref()
            || result.plan_hash != plan.plan_hash
        {
            return Err("Invalid guest readiness scope or plan.".into());
        }
    }
    Ok(())
}
#[tauri::command]
pub async fn prepare_remote_create(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
    yaml: String,
    target_vmid: u32,
) -> Result<Plan, String> {
    state
        .prepare_create(connection_id, request_key, yaml, target_vmid)
        .await
}
#[tauri::command]
pub async fn confirm_remote_create(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
    confirmed: bool,
) -> Result<Operation, String> {
    state
        .confirm_create(connection_id, &request_key, confirmed)
        .await
}
#[tauri::command]
pub async fn read_remote_create(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
) -> Result<Operation, String> {
    state.create_status(connection_id, &request_key).await
}

#[cfg(test)]
mod tests {
    use super::*;
    const TOKEN: &str = "fixture-only-access-token-0123456789abcdef";
    pub(super) fn plan() -> Plan {
        serde_json::from_value(json!({"definitionName":"mobile-lab","definitionHash":"a".repeat(64),"planHash":"b".repeat(64),"providerScopeId":"c".repeat(64),"source":{"templateVmid":9000,"templateVersion":"fixture-v1"},"target":{"node":"pve-test","vmid":202,"storage":"local-lvm","bridge":"vmbr0","guestUser":"debian","resources":{"cpu":2,"memoryMiB":2048,"diskGiB":8}},"creationEnabled":true,"sshKeyCount":1,"packages":[],"snapshotAfterProvisioning":false})).unwrap()
    }
    #[tokio::test]
    async fn remote_create_confirmation_posts_only_retained_approval_and_records_operation() {
        let body =
            json!({"id":"d".repeat(32),"kind":"vm_provision","status":"queued","result":null})
                .to_string();
        let (endpoint, ca, task) = super::super::tests::fixture(
            "202 Accepted",
            &body,
            "x-vmanyware-authenticated: bearer\r\n",
        )
        .await;
        let manager = RemoteConnection::default();
        let id = manager.begin().unwrap();
        manager
            .complete(id, Profile::new(&endpoint, TOKEN, Some(ca)).unwrap())
            .unwrap();
        let expected = plan();
        assert!(expected.valid(202));
        manager.0.lock().unwrap().prepared = Some(Prepared {
            key: "fixed-key".into(),
            plan: expected.clone(),
            submitted: false,
            inflight: false,
            operation: None,
        });
        assert!(manager.confirm_create(id, "wrong-key", true).await.is_err());
        assert!(manager
            .confirm_create(id, "fixed-key", false)
            .await
            .is_err());
        let result = manager.confirm_create(id, "fixed-key", true).await.unwrap();
        assert_eq!(result.id, "d".repeat(32));
        let request = task.await.unwrap();
        assert!(request.starts_with("POST /api/v1/provisioning/create "));
        assert!(request
            .to_lowercase()
            .contains("idempotency-key: fixed-key"));
        assert!(request.contains(TOKEN));
        let body: serde_json::Value =
            serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(body, expected.approval());
        assert!(
            manager
                .0
                .lock()
                .unwrap()
                .prepared
                .as_ref()
                .unwrap()
                .submitted
        );
        manager.begin().unwrap();
        assert!(manager.confirm_create(id, "fixed-key", true).await.is_err());
    }
    #[tokio::test]
    async fn remote_create_errors_keep_same_key_and_never_echo_server_body_or_token() {
        for headers in ["x-vmanyware-authenticated: bearer\r\n", ""] {
            let (endpoint, ca, task) = super::super::tests::fixture(
                "503 Service Unavailable",
                "PRIVATE_SERVER_BODY",
                headers,
            )
            .await;
            let manager = RemoteConnection::default();
            let id = manager.begin().unwrap();
            manager
                .complete(id, Profile::new(&endpoint, TOKEN, Some(ca)).unwrap())
                .unwrap();
            manager.0.lock().unwrap().prepared = Some(Prepared {
                key: "retained-key".into(),
                plan: plan(),
                submitted: false,
                inflight: false,
                operation: None,
            });
            let error = manager
                .confirm_create(id, "retained-key", true)
                .await
                .unwrap_err();
            assert!(!error.contains(TOKEN));
            assert!(!error.contains("PRIVATE_SERVER_BODY"));
            let request = task.await.unwrap();
            assert!(request
                .to_lowercase()
                .contains("idempotency-key: retained-key"));
            {
                let state = manager.0.lock().unwrap();
                let retained = state.prepared.as_ref().unwrap();
                assert!(retained.submitted);
                assert!(!retained.inflight);
                assert_eq!(retained.key, "retained-key");
            }
            assert!(manager
                .prepare_create(id, "new-key".into(), "ignored".into(), 203)
                .await
                .is_err());
        }
    }
    #[tokio::test]
    async fn remote_create_disconnect_discards_reply_without_cancelling_or_replaying_submission() {
        let (seen, observed) = tokio::sync::oneshot::channel();
        let (release, wait) = tokio::sync::oneshot::channel();
        let body =
            json!({"id":"d".repeat(32),"kind":"vm_provision","status":"queued","result":null})
                .to_string();
        let (endpoint, ca, server) = super::super::tests::fixture_with_gate(
            "202 Accepted",
            &body,
            "x-vmanyware-authenticated: bearer\r\n",
            Some((seen, wait)),
        )
        .await;
        let manager = std::sync::Arc::new(RemoteConnection::default());
        let id = manager.begin().unwrap();
        manager
            .complete(id, Profile::new(&endpoint, TOKEN, Some(ca)).unwrap())
            .unwrap();
        manager.0.lock().unwrap().prepared = Some(Prepared {
            key: "retained-key".into(),
            plan: plan(),
            submitted: false,
            inflight: false,
            operation: None,
        });
        let connection = manager.clone();
        let task =
            tokio::spawn(async move { connection.confirm_create(id, "retained-key", true).await });
        tokio::time::timeout(std::time::Duration::from_secs(3), observed)
            .await
            .unwrap()
            .unwrap();
        manager.begin().unwrap();
        release.send(()).unwrap();
        assert!(task.await.unwrap().unwrap_err().contains("may continue"));
        assert!(manager.0.lock().unwrap().prepared.is_none());
        assert!(server
            .await
            .unwrap()
            .starts_with("POST /api/v1/provisioning/create "));
    }

    #[tokio::test]
    async fn remote_create_status_reads_only_retained_operation_and_rejects_mismatched_ids() {
        for different in [false, true] {
            let body=json!({"id":if different {"e".repeat(32)} else {"d".repeat(32)},"kind":"vm_provision","status":"running","result":null}).to_string();
            let (endpoint, ca, server) = super::super::tests::fixture(
                "200 OK",
                &body,
                "x-vmanyware-authenticated: bearer\r\n",
            )
            .await;
            let manager = RemoteConnection::default();
            let id = manager.begin().unwrap();
            manager
                .complete(id, Profile::new(&endpoint, TOKEN, Some(ca)).unwrap())
                .unwrap();
            manager.0.lock().unwrap().prepared = Some(Prepared {
                key: "retained-key".into(),
                plan: plan(),
                submitted: true,
                inflight: false,
                operation: Some("d".repeat(32)),
            });
            let result = manager.create_status(id, "retained-key").await;
            assert_eq!(result.is_ok(), !different);
            let request = server.await.unwrap();
            assert!(request.starts_with(&format!("GET /api/v1/operations/{} ", "d".repeat(32))));
            assert!(request.contains(TOKEN));
        }
    }

    #[test]
    fn remote_create_plan_and_ready_reports_reject_wrong_shape_scope_and_false_success() {
        let valid = plan();
        assert!(valid.valid(202));
        assert!(!valid.valid(203));
        let mut invalid = valid.clone();
        invalid.definition_name = "..".into();
        assert!(!invalid.valid(202));
        let mut operation:Operation=serde_json::from_value(json!({"id":"d".repeat(32),"kind":"vm_provision","status":"succeeded","result":{"instanceId":"e".repeat(32),"providerScopeId":"c".repeat(64),"planHash":"b".repeat(64),"guestReady":true,"guest":{"ipv4":"192.0.2.42","interface":"ens18","cloudInitExitCode":0,"cloudInitClean":true}}})).unwrap();
        assert!(validate_operation(&operation, &valid).is_ok());
        operation
            .result
            .as_mut()
            .unwrap()
            .guest
            .cloud_init_exit_code = 2;
        assert!(validate_operation(&operation, &valid).is_err());
        operation
            .result
            .as_mut()
            .unwrap()
            .guest
            .cloud_init_exit_code = 0;
        operation.result.as_mut().unwrap().provider_scope_id = "f".repeat(64);
        assert!(validate_operation(&operation, &valid).is_err());
    }
}

#[cfg(test)]
mod prepare_tests {
    use super::*;
    use std::sync::Arc;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    use tokio_rustls::{rustls, TlsAcceptor};
    pub(super) async fn sequence(
        responses: Vec<(&'static str, String)>,
    ) -> (
        String,
        reqwest::Certificate,
        tokio::task::JoinHandle<Vec<String>>,
    ) {
        let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let ca = reqwest::Certificate::from_pem(certificate.cert.pem().as_bytes()).unwrap();
        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![certificate.cert.der().clone()],
                rustls::pki_types::PrivatePkcs8KeyDer::from(
                    certificate.signing_key.serialize_der(),
                )
                .into(),
            )
            .unwrap();
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!(
            "https://localhost:{}",
            listener.local_addr().unwrap().port()
        );
        let task = tokio::spawn(async move {
            let mut requests = Vec::new();
            for (status, body) in responses {
                let (socket, _) = listener.accept().await.unwrap();
                let mut stream = acceptor.accept(socket).await.unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0; 2048];
                let end = loop {
                    let n = stream.read(&mut buffer).await.unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buffer[..n]);
                    if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        break end;
                    }
                };
                let length = String::from_utf8_lossy(&bytes[..end])
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
                while bytes.len() < end + 4 + length {
                    let n = stream.read(&mut buffer).await.unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buffer[..n]);
                }
                requests.push(String::from_utf8(bytes).unwrap());
                let response=format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nx-vmanyware-authenticated: bearer\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
                stream.write_all(response.as_bytes()).await.unwrap();
                let _ = stream.shutdown().await;
            }
            requests
        });
        (endpoint, ca, task)
    }
    #[tokio::test]
    async fn remote_create_prepare_saves_or_matches_definition_before_allowing_confirmation() {
        for variant in ["new", "same", "different"] {
            let plan = super::tests::plan();
            let record=json!({"name":"mobile-lab","hash":if variant=="different"{"f".repeat(64)}else{"a".repeat(64)}}).to_string();
            let mut responses = vec![("200 OK", serde_json::to_string(&plan).unwrap())];
            if variant == "new" {
                responses.push(("201 Created", record));
            } else {
                responses.push(("409 Conflict", "{}".into()));
                responses.push(("200 OK", record));
            }
            let (endpoint, ca, task) = sequence(responses).await;
            let manager = RemoteConnection::default();
            let id = manager.begin().unwrap();
            manager
                .complete(
                    id,
                    Profile::new(
                        &endpoint,
                        "fixture-only-access-token-0123456789abcdef",
                        Some(ca),
                    )
                    .unwrap(),
                )
                .unwrap();
            let result = manager
                .prepare_create(id, "prepared-key".into(), "fixture-yaml".into(), 202)
                .await;
            assert_eq!(result.is_ok(), variant != "different");
            let requests = task.await.unwrap();
            assert!(requests[0].starts_with("POST /api/v1/provisioning/plan "));
            assert!(requests[1].starts_with("POST /api/v1/definitions "));
            assert!(requests[1].contains("application/yaml"));
            assert!(requests[1].ends_with("fixture-yaml"));
            if variant != "new" {
                assert!(requests[2].starts_with("GET /api/v1/definitions/mobile-lab "));
            }
            assert!(requests
                .iter()
                .all(|request| !request.contains("/provisioning/create ")));
            assert_eq!(
                manager.0.lock().unwrap().prepared.is_some(),
                variant != "different"
            );
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreationSummary {
    id: String,
    kind: String,
    status: Status,
    instance_id: Option<String>,
    created_at: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryEnvironment {
    id: String,
    definition_name: String,
    definition_hash: String,
    provider_reference: RecoveryProvider,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecoveryProvider {
    provider: String,
    scope_id: String,
    node: String,
    vmid: u32,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveredCreation {
    operation: Operation,
    environment: RecoveryEnvironment,
}
impl RemoteConnection {
    async fn recent_creations(&self, id: u64) -> Result<Vec<CreationSummary>, String> {
        let profile = self.snapshot(id)?;
        let result: Result<Vec<CreationSummary>, String> = profile
            .create_json(
                profile.client.get(
                    profile
                        .endpoint
                        .join("api/v1/operations")
                        .map_err(|_| "Invalid server address.")?,
                ),
            )
            .await;
        self.current(id)?;
        let operations = result?;
        if operations.len() > 100
            || operations.iter().any(|o| {
                !hex(&o.id, 32)
                    || o.instance_id.as_ref().is_some_and(|i| !hex(i, 32))
                    || o.created_at.is_empty()
                    || o.created_at.len() > 48
            })
        {
            return Err("Invalid operation history.".into());
        }
        Ok(operations
            .into_iter()
            .filter(|o| o.kind == "vm_provision")
            .collect())
    }
    async fn recover_creation(
        &self,
        id: u64,
        operation_id: &str,
    ) -> Result<RecoveredCreation, String> {
        if !hex(operation_id, 32) {
            return Err("Enter a 32-character operation ID.".into());
        }
        let profile = self.snapshot(id)?;
        let result: Result<Operation, String> = profile
            .create_json(
                profile.client.get(
                    profile
                        .endpoint
                        .join(&format!("api/v1/operations/{operation_id}"))
                        .map_err(|_| "Invalid operation ID.")?,
                ),
            )
            .await;
        self.current(id)?;
        let operation = result?;
        validate_read_operation(&operation)?;
        if operation.id != operation_id {
            return Err("The server returned another operation.".into());
        }
        let instance = operation
            .instance_id
            .as_ref()
            .filter(|i| hex(i, 32))
            .ok_or("The creation has no valid environment association.")?;
        let result: Result<RecoveryEnvironment, String> = profile
            .create_json(
                profile.client.get(
                    profile
                        .endpoint
                        .join(&format!("api/v1/environments/{instance}"))
                        .map_err(|_| "Invalid environment ID.")?,
                ),
            )
            .await;
        self.current(id)?;
        let environment = result?;
        if &environment.id != instance
            || !hex(&environment.definition_hash, 64)
            || !identifier(&environment.definition_name, 63)
            || environment.provider_reference.provider != "proxmox"
            || !hex(&environment.provider_reference.scope_id, 64)
            || !identifier(&environment.provider_reference.node, 63)
            || !(100..=999999999).contains(&environment.provider_reference.vmid)
        {
            return Err("Invalid retained environment association.".into());
        }
        if let Some(ready) = operation
            .result
            .as_ref()
            .filter(|_| matches!(operation.status, Status::Succeeded))
        {
            if ready.instance_id != environment.id
                || ready.provider_scope_id != environment.provider_reference.scope_id
            {
                return Err("Guest proof differs from the retained environment scope.".into());
            }
        }
        Ok(RecoveredCreation {
            operation,
            environment,
        })
    }
}
#[tauri::command]
pub async fn read_remote_creations(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
) -> Result<Vec<CreationSummary>, String> {
    state.recent_creations(connection_id).await
}
#[tauri::command]
pub async fn recover_remote_creation(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    operation_id: String,
) -> Result<RecoveredCreation, String> {
    state.recover_creation(connection_id, &operation_id).await
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    const TOKEN: &str = "fixture-only-access-token-0123456789abcdef";
    fn operation() -> serde_json::Value {
        json!({"id":"d".repeat(32),"instanceId":"e".repeat(32),"kind":"vm_provision","status":"succeeded","result":{"instanceId":"e".repeat(32),"providerScopeId":"c".repeat(64),"planHash":"b".repeat(64),"guestReady":true,"guest":{"ipv4":"192.0.2.42","interface":"ens18","cloudInitExitCode":0,"cloudInitClean":true}}})
    }
    fn environment() -> serde_json::Value {
        json!({"id":"e".repeat(32),"definitionName":"recovered-lab","definitionHash":"a".repeat(64),"providerReference":{"provider":"proxmox","scopeId":"c".repeat(64),"node":"pve-test","vmid":202}})
    }
    fn manager(endpoint: &str, ca: reqwest::Certificate) -> (RemoteConnection, u64) {
        let manager = RemoteConnection::default();
        let id = manager.begin().unwrap();
        manager
            .complete(id, Profile::new(endpoint, TOKEN, Some(ca)).unwrap())
            .unwrap();
        (manager, id)
    }
    #[tokio::test]
    async fn remote_recovery_works_without_prepared_intent_and_only_gets_exact_operation_and_environment(
    ) {
        for variant in [
            "valid",
            "wrong_operation",
            "wrong_kind",
            "missing_instance",
            "false_ready",
            "wrong_environment",
            "wrong_scope",
        ] {
            let mut op = operation();
            let mut env = environment();
            match variant {
                "wrong_operation" => op["id"] = json!("f".repeat(32)),
                "wrong_kind" => op["kind"] = json!("inventory_refresh"),
                "missing_instance" => {
                    op.as_object_mut().unwrap().remove("instanceId");
                }
                "false_ready" => op["result"]["guestReady"] = json!(false),
                "wrong_environment" => env["id"] = json!("f".repeat(32)),
                "wrong_scope" => env["providerReference"]["scopeId"] = json!("f".repeat(64)),
                _ => {}
            }
            let mut responses = vec![("200 OK", op.to_string())];
            if ["valid", "wrong_environment", "wrong_scope"].contains(&variant) {
                responses.push(("200 OK", env.to_string()));
            }
            let (endpoint, ca, task) = super::prepare_tests::sequence(responses).await;
            let (manager, id) = manager(&endpoint, ca);
            assert!(manager.0.lock().unwrap().prepared.is_none());
            let result = manager.recover_creation(id, &"d".repeat(32)).await;
            assert_eq!(result.is_ok(), variant == "valid", "{variant}");
            let requests = task.await.unwrap();
            assert!(requests[0].starts_with(&format!("GET /api/v1/operations/{} ", "d".repeat(32))));
            if requests.len() == 2 {
                assert!(requests[1]
                    .starts_with(&format!("GET /api/v1/environments/{} ", "e".repeat(32))));
            }
            assert!(requests.iter().all(|r| r.starts_with("GET ")));
            assert!(manager.0.lock().unwrap().prepared.is_none());
        }
    }
    #[tokio::test]
    async fn remote_recovery_history_filters_creation_jobs_and_rejects_invalid_rows() {
        for invalid in [false, true] {
            let body=json!([{"id":if invalid {"../bad".into()}else{"d".repeat(32)},"kind":"vm_provision","instanceId":"e".repeat(32),"status":"running","createdAt":"2026-10-18T10:00:00Z"},{"id":"f".repeat(32),"kind":"inventory_refresh","instanceId":null,"status":"succeeded","createdAt":"2026-10-18T10:00:00Z"}]).to_string();
            let (endpoint, ca, task) = super::super::tests::fixture(
                "200 OK",
                &body,
                "x-vmanyware-authenticated: bearer\r\n",
            )
            .await;
            let (manager, id) = manager(&endpoint, ca);
            let result = manager.recent_creations(id).await;
            assert_eq!(result.is_err(), invalid);
            if !invalid {
                assert_eq!(result.unwrap().len(), 1);
            }
            assert!(task.await.unwrap().starts_with("GET /api/v1/operations "));
        }
        let manager = RemoteConnection::default();
        assert!(manager.recover_creation(1, "../escape").await.is_err());
    }
    #[tokio::test]
    async fn remote_recovery_disconnect_discards_reply_before_fetching_another_resource() {
        let (seen, observed) = tokio::sync::oneshot::channel();
        let (release, wait) = tokio::sync::oneshot::channel();
        let (endpoint, ca, server) = super::super::tests::fixture_with_gate(
            "200 OK",
            &operation().to_string(),
            "x-vmanyware-authenticated: bearer\r\n",
            Some((seen, wait)),
        )
        .await;
        let (manager, id) = manager(&endpoint, ca);
        let manager = std::sync::Arc::new(manager);
        let cloned = manager.clone();
        let task = tokio::spawn(async move { cloned.recover_creation(id, &"d".repeat(32)).await });
        tokio::time::timeout(std::time::Duration::from_secs(3), observed)
            .await
            .unwrap()
            .unwrap();
        manager.begin().unwrap();
        release.send(()).unwrap();
        assert!(task.await.unwrap().is_err());
        assert!(server.await.unwrap().starts_with("GET "));
    }
}
