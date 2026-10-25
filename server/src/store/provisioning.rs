use super::{operations, Store};
use crate::{
    definition::Definition,
    operation::{Operation, Status},
    provisioning::Catalog,
    provisioning_intent::{Approval, Job, Stage},
    proxmox::{TaskReference, VmReference},
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvisioningError {
    Unavailable,
    InvalidApproval,
    DefinitionNotFound,
    TargetChanged,
    Conflict,
    InvalidOperation,
    ApprovalExpired,
}
fn unavailable<T>(_: T) -> ProvisioningError {
    ProvisioningError::Unavailable
}
fn constrained(error: rusqlite::Error) -> ProvisioningError {
    match error {
        rusqlite::Error::SqliteFailure(code, _)
            if matches!(
                code.extended_code,
                rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE | rusqlite::ffi::SQLITE_CONSTRAINT_TRIGGER
            ) =>
        {
            ProvisioningError::Conflict
        }
        _ => ProvisioningError::Unavailable,
    }
}
pub(super) fn read(connection: &Connection, id: &str) -> Result<Option<Job>, ProvisioningError> {
    let Some(operation) = operations::read(connection, id).map_err(unavailable)? else {
        return Ok(None);
    };
    if operation.kind != "vm_provision" {
        return Err(ProvisioningError::InvalidOperation);
    }
    let (json,hash,confirmed_at,catalog_yaml,plan_json): (String,String,i64,String,String) = connection.query_row(
        "SELECT o.approval_json,o.input_hash,o.confirmed_at,p.catalog_yaml,p.plan_json FROM operations o JOIN provisioning_intents p ON p.operation_id=o.id WHERE o.id=?1",[id],
        |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).map_err(unavailable)?;
    let approval: Approval = serde_json::from_str(&json).map_err(unavailable)?;
    if !approval.valid() || approval.input_hash().map_err(unavailable)? != hash {
        return Err(ProvisioningError::Unavailable);
    }
    let instance = super::instances::read(
        connection,
        operation
            .instance_id
            .as_deref()
            .ok_or(ProvisioningError::Unavailable)?,
    )
    .map_err(unavailable)?
    .ok_or(ProvisioningError::Unavailable)?;
    if instance.summary.definition_name != approval.definition_name
        || instance.summary.definition_hash != approval.definition_hash
        || instance.summary.provider_reference.scope_id != approval.provider_scope_id
        || instance.summary.provider_reference.node != approval.target_node
        || instance.summary.provider_reference.vmid != approval.target_vmid
    {
        return Err(ProvisioningError::Unavailable);
    }
    let recreated_from: Option<String> = connection
        .query_row(
            "SELECT source_instance_id FROM recreation_sources WHERE operation_id=?1",
            [id],
            |r| r.get(0),
        )
        .optional()
        .map_err(unavailable)?;
    if let Some(source_id) = &recreated_from {
        let source = super::instances::read(connection, source_id)
            .map_err(unavailable)?
            .ok_or(ProvisioningError::Unavailable)?;
        if source.summary.status != crate::instance::Status::Deleted
            || source.summary.id == instance.summary.id
            || source.summary.definition_hash != instance.summary.definition_hash
            || source.summary.definition_revision != instance.summary.definition_revision
            || source.summary.definition_name != instance.summary.definition_name
            || source.definition_yaml != instance.definition_yaml
        {
            return Err(ProvisioningError::Unavailable);
        }
    }
    let definition = Definition::parse(&instance.definition_yaml).map_err(unavailable)?;
    let catalog = Catalog::parse(&catalog_yaml).map_err(unavailable)?;
    let plan = catalog
        .plan(&definition, approval.target_vmid)
        .map_err(unavailable)?;
    if !approval.matches(&plan)
        || serde_json::from_str::<serde_json::Value>(&plan_json).map_err(unavailable)?
            != serde_json::to_value(&plan).map_err(unavailable)?
    {
        return Err(ProvisioningError::Unavailable);
    }
    let source =
        VmReference::new(&approval.target_node, approval.source_vmid).map_err(unavailable)?;
    let mut statement=connection.prepare("SELECT position,kind,status,dispatch_started,provider_upid,result_json,config_digest FROM provisioning_stages WHERE operation_id=?1 ORDER BY position").map_err(unavailable)?;
    let rows = statement
        .query_map([id], |r| {
            Ok((
                r.get::<_, u32>(0)? as usize,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, bool>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<String>>(6)?,
            ))
        })
        .map_err(unavailable)?;
    let mut stages = Vec::new();
    for row in rows {
        let (position, kind, status, dispatch_started, upid, result, config_digest) =
            row.map_err(unavailable)?;
        if position != stages.len() || plan.steps().get(position) != Some(&kind.as_str()) {
            return Err(ProvisioningError::Unavailable);
        }
        if (kind == "configure_resources" && dispatch_started) != config_digest.is_some()
            || config_digest.as_deref().is_some_and(|d| !valid_digest(d))
        {
            return Err(ProvisioningError::Unavailable);
        }
        let task = upid
            .map(|upid| {
                let task = TaskReference::new(&approval.target_node, &upid).map_err(unavailable)?;
                let bound = match kind.as_str() {
                    "full_clone" => task.matches_clone(&source),
                    "expand_disk" => task.matches_resize(
                        &VmReference::new(&approval.target_node, approval.target_vmid)
                            .map_err(unavailable)?,
                    ),
                    "snapshot_after_provisioning" => task.matches_snapshot(
                        &VmReference::new(&approval.target_node, approval.target_vmid)
                            .map_err(unavailable)?,
                    ),
                    "start_vm" => task.matches_action(
                        &VmReference::new(&approval.target_node, approval.target_vmid)
                            .map_err(unavailable)?,
                        crate::lifecycle::Action::Start,
                    ),
                    _ => false,
                };
                if !bound {
                    return Err(ProvisioningError::Unavailable);
                }
                Ok(task)
            })
            .transpose()?;
        stages.push(Stage {
            position,
            kind,
            status: serde_json::from_value(serde_json::Value::String(status))
                .map_err(unavailable)?,
            dispatch_started,
            task,
            config_digest,
            result: result
                .map(|json| serde_json::from_str(&json))
                .transpose()
                .map_err(unavailable)?,
        });
    }
    if stages.len() != plan.steps().len() {
        return Err(ProvisioningError::Unavailable);
    }
    let disk_dispatch = connection
        .query_row(
            "SELECT config_digest,volume FROM provisioning_disk_dispatch WHERE operation_id=?1",
            [id],
            |r| {
                Ok(crate::provisioning_intent::DiskDispatch {
                    config_digest: r.get(0)?,
                    volume: r.get(1)?,
                })
            },
        )
        .optional()
        .map_err(unavailable)?;
    if stages
        .iter()
        .any(|s| s.kind == "expand_disk" && s.dispatch_started)
        != disk_dispatch.is_some()
    {
        return Err(ProvisioningError::Unavailable);
    }
    if let Some(dispatch) = &disk_dispatch {
        if plan
            .disk_request(
                operation
                    .instance_id
                    .as_deref()
                    .ok_or(ProvisioningError::Unavailable)?,
                &dispatch.config_digest,
                &dispatch.volume,
            )
            .is_err()
        {
            return Err(ProvisioningError::Unavailable);
        }
    }
    let cloud_init_dispatch = connection.query_row("SELECT config_digest,volume,network FROM provisioning_cloud_init_dispatch WHERE operation_id=?1",[id],|r|Ok(crate::provisioning_intent::CloudInitDispatch {config_digest:r.get(0)?,volume:r.get(1)?,network:r.get(2)?})).optional().map_err(unavailable)?;
    if stages
        .iter()
        .any(|s| s.kind == "configure_cloud_init" && s.dispatch_started)
        != cloud_init_dispatch.is_some()
    {
        return Err(ProvisioningError::Unavailable);
    }
    let journal: Option<String> = connection
        .query_row(
            "SELECT journal_json FROM provisioning_seed_regeneration WHERE operation_id=?1",
            [id],
            |r| r.get(0),
        )
        .optional()
        .map_err(unavailable)?;
    let seed_journal = journal
        .map(|j| serde_json::from_str::<crate::provisioning_intent::SeedJournal>(&j))
        .transpose()
        .map_err(unavailable)?;
    if stages
        .iter()
        .any(|s| s.kind == "start_vm" && s.dispatch_started)
        != seed_journal.is_some()
    {
        return Err(ProvisioningError::Unavailable);
    }
    let job = Job {
        recreated_from,
        operation,
        approval,
        confirmed_at,
        definition,
        catalog,
        plan,
        stages,
        disk_dispatch,
        cloud_init_dispatch,
        seed_journal,
    };
    if let Some(journal) = &job.seed_journal {
        let settings = job
            .cloud_init_settings()
            .ok_or(ProvisioningError::Unavailable)?;
        if !job.valid_seed(&journal.expected)
            || journal.expected.config_digest != settings.settings.resources.target.config_digest
            || (journal.completed.is_some() && job.regenerated_seed().is_none())
        {
            return Err(ProvisioningError::Unavailable);
        }
    }
    if let Some(power) = job.seed_journal.as_ref().and_then(|j| j.power.as_ref()) {
        let seed = job
            .regenerated_seed()
            .ok_or(ProvisioningError::Unavailable)?;
        if power
            .config_digest
            .as_ref()
            .is_some_and(|d| d != &seed.config_digest)
        {
            return Err(ProvisioningError::Unavailable);
        }
    }
    if job
        .stages
        .iter()
        .any(|s| s.kind == "start_vm" && s.task.is_some())
        && !job.power_dispatch_started()
    {
        return Err(ProvisioningError::Unavailable);
    }
    if let Some(probe) = job.seed_journal.as_ref().and_then(|j| j.readiness.as_ref()) {
        let stage = job
            .stages
            .iter()
            .find(|s| s.kind == "wait_for_guest_readiness")
            .ok_or(ProvisioningError::Unavailable)?;
        if job.started().is_none()
            || stage.dispatch_started != (probe.attempts > 0)
            || probe.started_at <= 0
            || probe.started_at.checked_add(300) != Some(probe.deadline)
            || probe.attempts > 60
            || (probe.probe_started && probe.attempts == 0)
            || (probe.pid.is_some() && !probe.probe_started)
            || probe.pid.is_some_and(|p| p == 0 || p > i32::MAX as u32)
        {
            return Err(ProvisioningError::Unavailable);
        }
    }
    if let Some(packages) = job.seed_journal.as_ref().and_then(|j| j.packages.as_ref()) {
        let stage = job
            .stages
            .iter()
            .find(|s| s.kind == "install_declared_packages")
            .ok_or(ProvisioningError::Unavailable)?;
        if job.guest_readiness().is_none()
            || job.definition.spec.provisioning.packages.is_empty()
            || packages.started_at <= 0
            || packages.started_at.checked_add(960) != Some(packages.deadline)
            || packages.pid.is_some_and(|p| p == 0 || p > i32::MAX as u32)
            || (packages.pid.is_some() && !stage.dispatch_started)
            || (stage.status == Status::Succeeded && job.installed_packages().is_none())
        {
            return Err(ProvisioningError::Unavailable);
        }
    } else if job
        .stages
        .iter()
        .any(|s| s.kind == "install_declared_packages" && s.dispatch_started)
    {
        return Err(ProvisioningError::Unavailable);
    }
    if let Some(stage) = job
        .stages
        .iter()
        .find(|s| s.kind == "snapshot_after_provisioning")
    {
        let digest = job
            .seed_journal
            .as_ref()
            .and_then(|j| j.snapshot_digest.as_ref());
        if stage.dispatch_started != digest.is_some()
            || digest.is_some_and(|d| {
                !valid_digest(d)
                    || job
                        .regenerated_seed()
                        .is_none_or(|seed| seed.config_digest != *d)
            })
            || (stage.dispatch_started && !job.final_snapshot_prerequisites())
            || (stage.status == Status::Succeeded
                && (!stage.dispatch_started
                    || stage.task.is_none()
                    || stage
                        .result
                        .as_ref()
                        .and_then(|v| {
                            serde_json::from_value::<crate::snapshot::ResultEvidence>(v.clone())
                                .ok()
                        })
                        .is_none_or(|p| !job.valid_final_snapshot(&p))))
        {
            return Err(ProvisioningError::Unavailable);
        }
    }
    if job
        .seed_journal
        .as_ref()
        .is_some_and(|j| j.snapshot_digest.is_some())
        && !job
            .stages
            .iter()
            .any(|s| s.kind == "snapshot_after_provisioning")
    {
        return Err(ProvisioningError::Unavailable);
    }
    if job
        .disk_dispatch
        .as_ref()
        .is_some_and(|d| job.resource_config_digest().as_deref() != Some(d.config_digest.as_str()))
    {
        return Err(ProvisioningError::Unavailable);
    }
    if let Some(dispatch) = &job.cloud_init_dispatch {
        let (digest, volume) = job
            .cloud_init_source()
            .ok_or(ProvisioningError::Unavailable)?;
        if digest != dispatch.config_digest
            || volume.as_ref().is_some_and(|v| v != &dispatch.volume)
            || job
                .plan
                .cloud_init_request(
                    &job.definition,
                    job.operation
                        .instance_id
                        .as_deref()
                        .ok_or(ProvisioningError::Unavailable)?,
                    &dispatch.config_digest,
                    &dispatch.volume,
                    &dispatch.network,
                )
                .is_err()
        {
            return Err(ProvisioningError::Unavailable);
        }
    }
    Ok(Some(job))
}
impl Store {
    /// Commits the complete plan, canonical definition association and journal before returning.
    pub fn enqueue_provisioning(
        &self,
        key: &str,
        approval: &Approval,
        catalog: &Catalog,
    ) -> Result<Operation, ProvisioningError> {
        self.enqueue_provisioning_source(key, approval, catalog, None)
    }

    /// Fresh confirmation of the archived definition; never resumes the old identity.
    pub fn enqueue_recreation(
        &self,
        key: &str,
        source_id: &str,
        approval: &Approval,
        catalog: &Catalog,
    ) -> Result<Operation, ProvisioningError> {
        if source_id.len() != 32
            || !source_id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(ProvisioningError::InvalidApproval);
        }
        self.enqueue_provisioning_source(key, approval, catalog, Some(source_id))
    }

    fn enqueue_provisioning_source(
        &self,
        key: &str,
        approval: &Approval,
        catalog: &Catalog,
        source_id: Option<&str>,
    ) -> Result<Operation, ProvisioningError> {
        if !crate::operation::valid_request_key(key) || !approval.valid() {
            return Err(ProvisioningError::InvalidApproval);
        }
        let hash = approval.input_hash().map_err(unavailable)?;
        let mut connection = self.0.lock().map_err(unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let existing: Option<(String, Option<String>, String)> = transaction
            .query_row(
                "SELECT id,input_hash,kind FROM operations WHERE request_key=?1",
                [key],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(unavailable)?;
        if let Some((id, old_hash, kind)) = existing {
            if kind != "vm_provision" || old_hash.as_deref() != Some(hash.as_str()) {
                return Err(ProvisioningError::Conflict);
            }
            let saved_source: Option<String> = transaction
                .query_row(
                    "SELECT source_instance_id FROM recreation_sources WHERE operation_id=?1",
                    [&id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(unavailable)?;
            if saved_source.as_deref() != source_id {
                return Err(ProvisioningError::Conflict);
            }
            return Ok(read(&transaction, &id)?
                .ok_or(ProvisioningError::Unavailable)?
                .operation);
        }
        let record = if let Some(source_id) = source_id {
            let source = super::instances::read(&transaction, source_id)
                .map_err(unavailable)?
                .ok_or(ProvisioningError::DefinitionNotFound)?;
            if source.summary.status != crate::instance::Status::Deleted {
                return Err(ProvisioningError::Conflict);
            }
            super::Record {
                name: source.summary.definition_name,
                yaml: source.definition_yaml,
                hash: source.summary.definition_hash,
                revision: source.summary.definition_revision,
                created_at: source.summary.created_at.clone(),
                updated_at: source.summary.created_at,
            }
        } else {
            super::read(&transaction, &approval.definition_name)
                .map_err(unavailable)?
                .ok_or(ProvisioningError::DefinitionNotFound)?
        };
        let definition = Definition::parse(&record.yaml).map_err(unavailable)?;
        if definition.hash().map_err(unavailable)? != record.hash {
            return Err(ProvisioningError::Unavailable);
        }
        let plan = catalog
            .plan(&definition, approval.target_vmid)
            .map_err(|_| ProvisioningError::TargetChanged)?;
        if !approval.matches(&plan) {
            return Err(ProvisioningError::TargetChanged);
        }
        let instance: String=transaction.query_row(
            "INSERT INTO instances (definition_name,definition_hash,definition_revision,definition_yaml,provider_scope,node,vmid) VALUES (?1,?2,?3,?4,?5,?6,?7) RETURNING id",
            params![record.name,record.hash,record.revision,record.yaml,approval.provider_scope_id,approval.target_node,approval.target_vmid],|r|r.get(0)).map_err(constrained)?;
        let id: String=transaction.query_row("INSERT INTO operations (request_key,kind,instance_id,status,approval_json,input_hash,confirmed_at) VALUES (?1,'vm_provision',?2,'queued',?3,?4,unixepoch()) RETURNING id",
            params![key,instance,serde_json::to_string(approval).map_err(unavailable)?,hash],|r|r.get(0)).map_err(constrained)?;
        transaction
            .execute(
                "INSERT INTO provisioning_intents VALUES (?1,?2,?3)",
                params![
                    id,
                    catalog.canonical_yaml().map_err(unavailable)?,
                    serde_json::to_string(&plan).map_err(unavailable)?
                ],
            )
            .map_err(unavailable)?;
        for (position, step) in plan.steps().iter().enumerate() {
            transaction
                .execute(
                    "INSERT INTO provisioning_stages(operation_id,position,kind) VALUES (?1,?2,?3)",
                    params![id, position as u32, step],
                )
                .map_err(unavailable)?;
        }
        if let Some(source_id) = source_id {
            transaction.execute("INSERT INTO recreation_sources(operation_id,source_instance_id) VALUES (?1,?2)", params![id,source_id]).map_err(constrained)?;
        }
        let job = read(&transaction, &id)?.ok_or(ProvisioningError::Unavailable)?;
        transaction.commit().map_err(unavailable)?;
        Ok(job.operation)
    }
    pub fn provisioning_job(&self, id: &str) -> Result<Option<Job>, ProvisioningError> {
        let connection = self.0.lock().map_err(unavailable)?;
        read(&connection, id)
    }
    /// Initially only clone work can be claimed. Unknown dispatched clones are never reclaimed.
    pub fn claim_provisioning_clone(&self) -> Result<Option<Job>, ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let id: Option<String>=transaction.query_row("UPDATE operations SET status='running',failure_code=NULL WHERE id=(SELECT o.id FROM operations o JOIN provisioning_stages s ON s.operation_id=o.id AND s.position=0 WHERE o.kind='vm_provision' AND (o.status='queued' OR (o.status='uncertain' AND s.provider_upid IS NOT NULL AND o.next_reconcile_at<=unixepoch())) AND s.kind='full_clone' AND s.status IN ('queued','uncertain') ORDER BY o.created_at,o.id LIMIT 1) RETURNING id",[],|r|r.get(0)).optional().map_err(unavailable)?;
        let job = if let Some(id) = id {
            transaction.execute("UPDATE provisioning_stages SET status='running' WHERE operation_id=?1 AND position=0",[&id]).map_err(unavailable)?;
            Some(read(&transaction, &id)?.ok_or(ProvisioningError::Unavailable)?)
        } else {
            None
        };
        transaction.commit().map_err(unavailable)?;
        Ok(job)
    }
    pub fn begin_provisioning_clone(
        &self,
        id: &str,
        expected: &Approval,
    ) -> Result<(), ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let job = read(&transaction, id)?.ok_or(ProvisioningError::InvalidOperation)?;
        let stage = &job.stages[0];
        if job.operation.status != Status::Running
            || stage.status != Status::Running
            || stage.dispatch_started
        {
            return Err(ProvisioningError::InvalidOperation);
        }
        if !expected.valid()
            || expected.input_hash().map_err(unavailable)?
                != job.approval.input_hash().map_err(unavailable)?
        {
            return Err(ProvisioningError::TargetChanged);
        }
        let now: i64 = transaction
            .query_row("SELECT unixepoch()", [], |r| r.get(0))
            .map_err(unavailable)?;
        if !now
            .checked_sub(job.confirmed_at)
            .is_some_and(|age| (0..=120).contains(&age))
        {
            return Err(ProvisioningError::ApprovalExpired);
        }
        transaction.execute("UPDATE provisioning_stages SET dispatch_started=1 WHERE operation_id=?1 AND position=0",[id]).map_err(unavailable)?;
        transaction
            .execute("UPDATE operations SET dispatch_started=1 WHERE id=?1", [id])
            .map_err(unavailable)?;
        transaction.commit().map_err(unavailable)
    }
    pub fn link_provisioning_clone(
        &self,
        id: &str,
        task: &TaskReference,
    ) -> Result<(), ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let job = read(&transaction, id)?.ok_or(ProvisioningError::InvalidOperation)?;
        let stage = &job.stages[0];
        if job.operation.status != Status::Running
            || stage.status != Status::Running
            || !stage.dispatch_started
        {
            return Err(ProvisioningError::InvalidOperation);
        }
        if !task.matches_clone(
            &VmReference::new(&job.approval.target_node, job.approval.source_vmid)
                .map_err(unavailable)?,
        ) {
            return Err(ProvisioningError::TargetChanged);
        }
        if stage.task.as_ref().is_some_and(|old| old != task) {
            return Err(ProvisioningError::Conflict);
        }
        transaction.execute("UPDATE provisioning_stages SET provider_upid=?2 WHERE operation_id=?1 AND position=0",params![id,task.upid()]).map_err(unavailable)?;
        transaction
            .execute(
                "UPDATE operations SET provider_upid=?2 WHERE id=?1",
                params![id, task.upid()],
            )
            .map_err(unavailable)?;
        transaction.commit().map_err(unavailable)
    }
}

impl Store {
    /// Completing clone advances its journal only; provisioning remains running and incomplete.
    pub fn finish_provisioning_clone(
        &self,
        id: &str,
        outcome: crate::provisioning_intent::CloneOutcome,
    ) -> Result<Operation, ProvisioningError> {
        use crate::{
            operation::Failure,
            provisioning_intent::CloneOutcome,
            proxmox::{PowerState, QmpState, TaskStatus},
        };
        let mut connection = self.0.lock().map_err(unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let job = read(&transaction, id)?.ok_or(ProvisioningError::InvalidOperation)?;
        let stage = &job.stages[0];
        if job.operation.status != Status::Running || stage.status != Status::Running {
            return Err(ProvisioningError::InvalidOperation);
        }
        let (stage_status, operation_status, result, failure) = match outcome {
            CloneOutcome::Completed(evidence) => {
                if !stage.dispatch_started
                    || stage.task.is_none()
                    || evidence.instance_id
                        != job.operation.instance_id.as_deref().unwrap_or_default()
                    || evidence.provider_scope_id != job.approval.provider_scope_id
                    || evidence.plan_hash != job.approval.plan_hash
                    || evidence.task_status != TaskStatus::Succeeded
                    || evidence.target.observation.node != job.approval.target_node
                    || evidence.target.observation.vmid != job.approval.target_vmid
                    || evidence.target.observation.status != PowerState::Stopped
                    || !matches!(
                        evidence.target.observation.qmp_status,
                        None | Some(QmpState::Stopped)
                    )
                    || evidence.target.config_digest.len() != 40
                    || !evidence
                        .target
                        .config_digest
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit())
                {
                    return Err(ProvisioningError::InvalidOperation);
                }
                (
                    "succeeded",
                    "running",
                    Some(serde_json::to_string(&evidence).map_err(unavailable)?),
                    None,
                )
            }
            CloneOutcome::Failed(code) => {
                if matches!(
                    code,
                    Failure::ProviderSubmissionUnconfirmed
                        | Failure::ProviderTaskUnconfirmed
                        | Failure::PowerStateUnconfirmed
                ) || (stage.dispatch_started
                    && !matches!(
                        (&stage.task, code),
                        (
                            None,
                            Failure::ProviderUnauthorized | Failure::ProviderForbidden
                        ) | (Some(_), Failure::ProviderTaskFailed)
                    ))
                {
                    return Err(ProvisioningError::InvalidOperation);
                }
                // Refused before dispatch, or rejected by authorization at dispatch with no
                // task: no provider VM can exist, so release the VMID association.
                if !stage.dispatch_started
                    || (stage.task.is_none()
                        && matches!(
                            code,
                            Failure::ProviderUnauthorized | Failure::ProviderForbidden
                        ))
                {
                    if let Some(instance) = job.operation.instance_id.as_deref() {
                        transaction
                            .execute(
                                "UPDATE instances SET status='released' WHERE id=?1 AND status='registered'",
                                [instance],
                            )
                            .map_err(unavailable)?;
                    }
                }
                ("failed", "failed", None, Some(code))
            }
            CloneOutcome::Uncertain(code) => {
                if !stage.dispatch_started
                    || !matches!(
                        code,
                        Failure::ProviderSubmissionUnconfirmed
                            | Failure::ProviderTaskUnconfirmed
                            | Failure::PowerStateUnconfirmed
                            | Failure::ProviderScopeChanged
                            | Failure::ProviderNotConfigured
                            | Failure::ProcessInterrupted
                    )
                {
                    return Err(ProvisioningError::InvalidOperation);
                }
                ("uncertain", "uncertain", None, Some(code))
            }
        };
        transaction.execute("UPDATE provisioning_stages SET status=?2,result_json=?3 WHERE operation_id=?1 AND position=0",params![id,stage_status,result]).map_err(unavailable)?;
        let failure = failure
            .map(serde_json::to_value)
            .transpose()
            .map_err(unavailable)?
            .map(|v| v.as_str().unwrap_or_default().to_owned());
        transaction.execute("UPDATE operations SET status=?2,failure_code=?3,next_reconcile_at=unixepoch()+5,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",params![id,operation_status,failure]).map_err(unavailable)?;
        let operation = operations::read(&transaction, id)
            .map_err(unavailable)?
            .ok_or(ProvisioningError::Unavailable)?;
        transaction.commit().map_err(unavailable)?;
        Ok(operation)
    }
}

pub(super) fn valid_digest(digest: &str) -> bool {
    digest.len() == 40 && digest.bytes().all(|b| b.is_ascii_hexdigit())
}

impl Store {
    /// Only advance a completed clone; a dispatched resource stage reconciles by GET.
    pub fn claim_provisioning_resources(&self) -> Result<Option<Job>, ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let id: Option<String> = transaction.query_row(
            "UPDATE provisioning_stages SET status='running' WHERE operation_id=(SELECT o.id FROM operations o JOIN provisioning_stages c ON c.operation_id=o.id AND c.position=0 JOIN provisioning_stages s ON s.operation_id=o.id AND s.position=1 WHERE o.kind='vm_provision' AND o.status IN ('running','uncertain') AND c.kind='full_clone' AND c.status='succeeded' AND s.kind='configure_resources' AND ((s.status IN ('queued','interrupted') AND s.dispatch_started=0) OR (s.status='uncertain' AND s.dispatch_started=1 AND o.next_reconcile_at<=unixepoch())) ORDER BY o.created_at,o.id LIMIT 1) AND position=1 RETURNING operation_id",
            [], |r| r.get(0)).optional().map_err(unavailable)?;
        let job = if let Some(id) = id {
            transaction
                .execute(
                    "UPDATE operations SET status='running',failure_code=NULL WHERE id=?1",
                    [&id],
                )
                .map_err(unavailable)?;
            let job = read(&transaction, &id)?.ok_or(ProvisioningError::Unavailable)?;
            let digest = job
                .cloned_config_digest()
                .ok_or(ProvisioningError::Unavailable)?;
            if job.stages[1]
                .config_digest
                .as_deref()
                .is_some_and(|retained| retained != digest)
            {
                return Err(ProvisioningError::Unavailable);
            }
            Some(job)
        } else {
            None
        };
        transaction.commit().map_err(unavailable)?;
        Ok(job)
    }

    /// Retain the exact clone revision before one resource PUT. Full-plan approval
    /// was checked before clone; later stages do not renew or restart that intent.
    pub fn begin_provisioning_resources(
        &self,
        id: &str,
        expected: &Approval,
        digest: &str,
    ) -> Result<(), ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let job = read(&transaction, id)?.ok_or(ProvisioningError::InvalidOperation)?;
        let stage = &job.stages[1];
        if job.operation.status != Status::Running
            || stage.status != Status::Running
            || stage.dispatch_started
        {
            return Err(ProvisioningError::InvalidOperation);
        }
        if !expected.valid()
            || expected.input_hash().map_err(unavailable)?
                != job.approval.input_hash().map_err(unavailable)?
            || !valid_digest(digest)
            || job.cloned_config_digest().as_deref() != Some(digest)
        {
            return Err(ProvisioningError::TargetChanged);
        }
        transaction.execute("UPDATE provisioning_stages SET dispatch_started=1,config_digest=?2 WHERE operation_id=?1 AND position=1",params![id,digest]).map_err(unavailable)?;
        transaction.commit().map_err(unavailable)
    }

    pub fn finish_provisioning_resources(
        &self,
        id: &str,
        outcome: crate::provisioning_intent::ResourceOutcome,
    ) -> Result<Operation, ProvisioningError> {
        use crate::{
            operation::Failure,
            provisioning_intent::ResourceOutcome,
            proxmox::{PowerState, QmpState},
        };
        let mut connection = self.0.lock().map_err(unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let job = read(&transaction, id)?.ok_or(ProvisioningError::InvalidOperation)?;
        let stage = &job.stages[1];
        if job.operation.status != Status::Running
            || stage.status != Status::Running
            || job.cloned_config_digest().is_none()
        {
            return Err(ProvisioningError::InvalidOperation);
        }
        let (stage_status, operation_status, result, failure) = match outcome {
            ResourceOutcome::Completed(evidence) => {
                let target = &evidence.resources.target;
                if !stage.dispatch_started
                    || stage.config_digest.is_none()
                    || Some(evidence.instance_id.as_str()) != job.operation.instance_id.as_deref()
                    || evidence.provider_scope_id != job.approval.provider_scope_id
                    || evidence.plan_hash != job.approval.plan_hash
                    || evidence.resources.cpu != job.plan.target.resources.cpu
                    || evidence.resources.memory_mib != job.plan.target.resources.memory_mib
                    || target.observation.node != job.approval.target_node
                    || target.observation.vmid != job.approval.target_vmid
                    || target.observation.status != PowerState::Stopped
                    || !matches!(
                        target.observation.qmp_status,
                        None | Some(QmpState::Stopped)
                    )
                    || !valid_digest(&target.config_digest)
                {
                    return Err(ProvisioningError::InvalidOperation);
                }
                (
                    "succeeded",
                    "running",
                    Some(serde_json::to_string(&evidence).map_err(unavailable)?),
                    None,
                )
            }
            ResourceOutcome::Failed(code) => {
                if stage.dispatch_started
                    || !matches!(
                        code,
                        Failure::ProvisioningDisabled
                            | Failure::ProvisioningPlanChanged
                            | Failure::ProvisioningPreconditionsFailed
                            | Failure::ProviderNotConfigured
                            | Failure::ProviderScopeChanged
                    )
                {
                    return Err(ProvisioningError::InvalidOperation);
                }
                ("failed", "failed", None, Some(code))
            }
            ResourceOutcome::Uncertain(code) => {
                if !stage.dispatch_started
                    || !matches!(
                        code,
                        Failure::ProviderSubmissionUnconfirmed
                            | Failure::ResourceStateUnconfirmed
                            | Failure::ProviderNotConfigured
                            | Failure::ProviderScopeChanged
                            | Failure::ProcessInterrupted
                    )
                {
                    return Err(ProvisioningError::InvalidOperation);
                }
                ("uncertain", "uncertain", None, Some(code))
            }
        };
        transaction.execute("UPDATE provisioning_stages SET status=?2,result_json=?3 WHERE operation_id=?1 AND position=1",params![id,stage_status,result]).map_err(unavailable)?;
        let failure = failure
            .map(serde_json::to_value)
            .transpose()
            .map_err(unavailable)?
            .map(|v| v.as_str().unwrap_or_default().to_owned());
        transaction.execute("UPDATE operations SET status=?2,failure_code=?3,next_reconcile_at=unixepoch()+5,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",params![id,operation_status,failure]).map_err(unavailable)?;
        let operation = operations::read(&transaction, id)
            .map_err(unavailable)?
            .ok_or(ProvisioningError::Unavailable)?;
        transaction.commit().map_err(unavailable)?;
        Ok(operation)
    }
}
