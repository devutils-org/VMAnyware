use crate::model::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
pub const YAML: &str = "apiVersion: vmanyware.dev/v1\nkind: Environment\n";
pub fn environment_value(id: &str) -> Value {
    json!({"id":id,"definitionName":format!("lab-{}", &id[..4]),"definitionHash":format!("{:x}", Sha256::digest(YAML.as_bytes())),"definitionRevision":1,
        "providerReference":{"provider":"proxmox","scopeId":"c".repeat(64),"node":"pve-test","vmid":202},
        "status":"registered","lastGuest":{"address":"10.10.10.205","user":"debian","observedAt":"2026-10-25T12:00:00Z"},"createdAt":"2026-10-25T12:00:00Z"})
}
pub fn environment(id: &str) -> Environment {
    serde_json::from_value(environment_value(id)).unwrap()
}
pub fn operation_value(id: &str, instance: &str) -> Value {
    json!({"id":id,"instanceId":instance,"kind":"vm_provision","status":"running","failure":null,"createdAt":"2026-10-25T12:00:00Z","updatedAt":"2026-10-25T12:00:01Z"})
}
pub fn operation(id: &str, instance: &str) -> Operation {
    serde_json::from_value(operation_value(id, instance)).unwrap()
}
pub fn progress_value(op: &str, instance: &str) -> Value {
    json!({"operationId":op,"instanceId":instance,"sourceInstanceId":null,"providerScopeId":"c".repeat(64),"definitionHash":format!("{:x}", Sha256::digest(YAML.as_bytes())),"planHash":"f".repeat(64),"node":"pve-test","vmid":202,"status":"running","failure":null,
        "stages":[{"position":0,"kind":"full_clone","status":"succeeded","submissionStarted":true},
        {"position":1,"kind":"configure_resources","status":"running","submissionStarted":true},
        {"position":2,"kind":"configure_cloud_init","status":"queued","submissionStarted":false},
        {"position":3,"kind":"start_vm","status":"queued","submissionStarted":false},
        {"position":4,"kind":"wait_for_guest_readiness","status":"queued","submissionStarted":false}]})
}
