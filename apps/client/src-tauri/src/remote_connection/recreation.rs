//! Retained source-bound recreation review. Preparation never submits creation.
use super::{
    create::{validate_operation, Operation, Plan},
    hex, RemoteConnection,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{Duration, Instant};
const CHECKS: [&str; 10] = [
    "template",
    "source_disk",
    "cloud_init_drive",
    "storage",
    "storage_capacity",
    "bridge",
    "target_vmid",
    "template_revision",
    "execution_permissions",
    "guest_readiness",
];
#[derive(Default)]
pub(super) struct RecreationState {
    epoch: u64,
    key: Option<String>,
    reading: bool,
    review: Option<Review>,
    prepared_at: Option<Instant>,
    submitted: bool,
    operation: Option<(String, String)>,
    terminal: bool,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    source_instance_id: String,
    source_definition_hash: String,
    source_definition_revision: u32,
    definition_yaml: String,
    plan: Plan,
    restores_previous_disk_contents: bool,
    requires_new_instance_identity: bool,
    recreation_enabled: bool,
    preflight: Preflight,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Preflight {
    plan_hash: String,
    creation_enabled: bool,
    vmid_reserved: bool,
    checks: Vec<Check>,
}
#[derive(Clone, Deserialize, Serialize)]
struct Check {
    code: String,
    status: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Source {
    pub(super) id: String,
    pub(super) definition_name: String,
    pub(super) definition_hash: String,
    pub(super) definition_revision: u32,
    pub(super) definition_yaml: String,
    pub(super) status: String,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Saved {
    pub(super) source_instance_id: String,
    pub(super) operation: Operation,
    pub(super) plan: Plan,
    pub(super) restores_previous_disk_contents: bool,
    pub(super) requires_new_instance_identity: bool,
}
impl Source {
    pub(super) fn valid(&self, id: &str) -> bool {
        self.id == id
            && self.status == "deleted"
            && hex(&self.definition_hash, 64)
            && self.definition_revision > 0
            && !self.definition_yaml.is_empty()
            && self.definition_yaml.len() <= 65536
    }
}
impl Saved {
    pub(super) fn valid(&self, source: &Source, operation_id: &str) -> Result<(), String> {
        if self.source_instance_id != source.id
            || self.operation.id() != operation_id
            || self.restores_previous_disk_contents
            || !self.requires_new_instance_identity
            || !self.plan.valid(self.plan.target_vmid())
            || !self
                .plan
                .matches_definition(&source.definition_name, &source.definition_hash)
            || !self
                .operation
                .instance_id()
                .is_some_and(|target| hex(target, 32) && target != source.id)
        {
            return Err(
                "Saved recreation does not match the selected source, plan or fresh instance."
                    .into(),
            );
        }
        validate_operation(&self.operation, &self.plan)
    }
}
impl Review {
    fn eligible(&self) -> bool {
        self.recreation_enabled
            && self
                .preflight
                .checks
                .iter()
                .all(|check| check.status != "failed")
            && CHECKS[..7].iter().all(|code| {
                self.preflight
                    .checks
                    .iter()
                    .any(|check| check.code == *code && check.status == "passed")
            })
    }
    fn operation_matches(
        &self,
        operation: &Operation,
        known: Option<&(String, String)>,
    ) -> Result<(), String> {
        validate_operation(operation, &self.plan)?;
        let target = operation
            .instance_id()
            .filter(|target| hex(target, 32) && *target != self.source_instance_id)
            .ok_or("Recreation did not return a distinct new instance identity.")?;
        if known.is_some_and(|(id, instance)| id != operation.id() || instance != target) {
            return Err("The saved recreation operation or instance changed.".into());
        }
        Ok(())
    }

    fn valid(&self, source: &Source, vmid: u32) -> bool {
        self.source_instance_id == source.id
            && self.source_definition_hash == source.definition_hash
            && self.source_definition_revision == source.definition_revision
            && self.definition_yaml == source.definition_yaml
            && self.plan.valid(vmid)
            && self
                .plan
                .matches_definition(&source.definition_name, &source.definition_hash)
            && !self.restores_previous_disk_contents
            && self.requires_new_instance_identity
            && self.recreation_enabled == self.plan.enabled()
            && self.preflight.creation_enabled == self.recreation_enabled
            && !self.preflight.vmid_reserved
            && hex(&self.preflight.plan_hash, 64)
            && serde_json::to_value(&self.plan)
                .is_ok_and(|plan| plan["planHash"] == self.preflight.plan_hash)
            && self.preflight.checks.len() == CHECKS.len()
            && CHECKS.iter().all(|code| {
                self.preflight
                    .checks
                    .iter()
                    .filter(|check| check.code == *code)
                    .count()
                    == 1
            })
            && self
                .preflight
                .checks
                .iter()
                .all(|check| matches!(check.status.as_str(), "passed" | "failed" | "unknown"))
    }
}
impl RemoteConnection {
    #[cfg(test)]
    pub(super) fn recreation_state_is_empty(&self) -> bool {
        let state = self.0.lock().unwrap();
        state.recreation.review.is_none()
            && state.recreation.key.is_none()
            && !state.recreation.submitted
            && state.recreation.operation.is_none()
    }

    async fn prepare_recreation(
        &self,
        id: u64,
        key: String,
        source_id: String,
        vmid: u32,
    ) -> Result<Review, String> {
        if !hex(&source_id, 32)
            || !(100..=999999999).contains(&vmid)
            || key.is_empty()
            || key.len() > 64
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        {
            return Err("Choose a valid retired source, target VMID and request key.".into());
        }
        let profile = self.snapshot(id)?;
        let epoch = {
            let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
            if state.generation != id || state.recreation.reading || state.recreation.submitted {
                return Err("The connection or recreation review changed.".into());
            }
            let review = &mut state.recreation;
            review.epoch = review
                .epoch
                .checked_add(1)
                .ok_or("Recreation state unavailable.")?;
            review.key = Some(key.clone());
            review.reading = true;
            review.review = None;
            review.prepared_at = None;
            review.operation = None;
            review.terminal = false;
            review.epoch
        };
        let result = async {
            let source: Source = profile
                .create_json(
                    profile.client.get(
                        profile
                            .endpoint
                            .join(&format!("api/v1/environments/{source_id}"))
                            .map_err(|_| "Invalid source path.")?,
                    ),
                )
                .await?;
            self.current(id)?;
            if !source.valid(&source_id) {
                return Err("The server did not return the selected retired definition.".into());
            }
            let review: Review = profile
                .create_json(
                    profile
                        .client
                        .post(
                            profile
                                .endpoint
                                .join(&format!(
                                    "api/v1/environments/{source_id}/recreate-preflight"
                                ))
                                .map_err(|_| "Invalid recreation path.")?,
                        )
                        .json(&json!({"targetVmid":vmid})),
                )
                .await?;
            if !review.valid(&source, vmid) {
                return Err(
                    "The recreation review does not match the retained source, plan or checks."
                        .into(),
                );
            }
            Ok(review)
        }
        .await;
        self.current(id)?;
        let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != id
            || state.recreation.epoch != epoch
            || state.recreation.key.as_ref() != Some(&key)
        {
            return Err("The recreation review changed. Prepare it again.".into());
        }
        state.recreation.reading = false;
        match result {
            Ok(review) => {
                state.recreation.review = Some(review.clone());
                state.recreation.prepared_at = Some(Instant::now());
                Ok(review)
            }
            Err(error) => Err(error),
        }
    }
    async fn confirm_recreation(
        &self,
        id: u64,
        key: &str,
        confirmed: bool,
    ) -> Result<Operation, String> {
        if !confirmed {
            return Err("Explicitly confirm the displayed source and fresh VM target.".into());
        }
        let profile = self.snapshot(id)?;
        let review = {
            let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
            if state.generation != id {
                return Err("The connection changed.".into());
            }
            let retained = &mut state.recreation;
            if retained.key.as_deref() != Some(key)
                || retained.reading
                || retained.operation.is_some()
            {
                return Err(
                    "Read the known operation or wait for this request before submitting.".into(),
                );
            }
            let review = retained.review.clone().ok_or("Prepare recreation first.")?;
            if !review.eligible() {
                return Err("Recreation prerequisites or execution capability do not pass.".into());
            }
            if !retained.submitted
                && !retained
                    .prepared_at
                    .is_some_and(|at| at.elapsed() <= Duration::from_secs(120))
            {
                return Err("Recreation review expired. Review the target again.".into());
            }
            retained.submitted = true;
            retained.reading = true;
            review
        };
        let result:Result<Operation,String>=profile.create_json(profile.client.post(profile.endpoint.join(&format!("api/v1/environments/{}/recreate",review.source_instance_id)).map_err(|_|"Invalid recreation source.")?).header("idempotency-key",key).json(&json!({"sourceInstanceId":review.source_instance_id,"approval":review.plan.approval()}))).await;
        let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != id {
            return Err("The connection changed. Submitted server work may continue.".into());
        }
        let retained = &mut state.recreation;
        retained.reading = false;
        let operation = result?;
        review.operation_matches(&operation, None)?;
        retained.operation = Some((
            operation.id().into(),
            operation.instance_id().unwrap().into(),
        ));
        retained.terminal = operation.terminal();
        Ok(operation)
    }
    async fn recreation_operation(&self, id: u64, key: &str) -> Result<Operation, String> {
        let profile = self.snapshot(id)?;
        let (review, known) = {
            let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
            if state.generation != id {
                return Err("The connection changed.".into());
            }
            let retained = &mut state.recreation;
            if retained.key.as_deref() != Some(key) || retained.reading {
                return Err("The recreation request changed or is busy.".into());
            }
            let known = retained
                .operation
                .clone()
                .ok_or("No operation ID was received. Retry only the same confirmed request.")?;
            let review = retained
                .review
                .clone()
                .ok_or("Recreation review unavailable.")?;
            retained.reading = true;
            (review, known)
        };
        let result: Result<Saved, String> = profile
            .create_json(
                profile.client.get(
                    profile
                        .endpoint
                        .join(&format!(
                            "api/v1/environments/{}/recreation-operations/{}",
                            review.source_instance_id, known.0
                        ))
                        .map_err(|_| "Invalid saved recreation path.")?,
                ),
            )
            .await;
        let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != id {
            return Err("The connection changed.".into());
        }
        let retained = &mut state.recreation;
        retained.reading = false;
        let saved = result?;
        if saved.source_instance_id != review.source_instance_id
            || saved.restores_previous_disk_contents
            || !saved.requires_new_instance_identity
            || !saved.plan.same_configuration(&review.plan)
        {
            return Err("Saved recreation does not match the retained source and plan.".into());
        }
        review.operation_matches(&saved.operation, Some(&known))?;
        retained.terminal = saved.operation.terminal();
        Ok(saved.operation)
    }
    fn clear_recreation(&self, id: u64, key: &str) -> Result<(), String> {
        self.current(id)?;
        let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != id || state.recreation.key.as_deref() != Some(key) {
            return Err("The recreation review changed.".into());
        }
        if state.recreation.submitted && (state.recreation.reading || !state.recreation.terminal) {
            return Err("Retain the submitted request and read its operation. Disconnecting does not cancel server work.".into());
        }
        let epoch = state
            .recreation
            .epoch
            .checked_add(1)
            .ok_or("Recreation state unavailable.")?;
        state.recreation = RecreationState {
            epoch,
            ..Default::default()
        };
        Ok(())
    }
}
#[tauri::command]
pub async fn prepare_remote_recreation(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
    source_instance_id: String,
    target_vmid: u32,
) -> Result<Review, String> {
    state
        .prepare_recreation(connection_id, request_key, source_instance_id, target_vmid)
        .await
}
#[tauri::command]
pub fn clear_remote_recreation(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
) -> Result<(), String> {
    state.clear_recreation(connection_id, &request_key)
}

#[tauri::command]
pub async fn confirm_remote_recreation(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
    confirmed: bool,
) -> Result<Operation, String> {
    state
        .confirm_recreation(connection_id, &request_key, confirmed)
        .await
}
#[tauri::command]
pub async fn read_remote_recreation_operation(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
) -> Result<Operation, String> {
    state
        .recreation_operation(connection_id, &request_key)
        .await
}

#[cfg(test)]
#[path = "recreation_tests.rs"]
mod tests;
