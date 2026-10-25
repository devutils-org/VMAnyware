import { useEffect, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { CheckSummary, useOperationPolling } from "./actionFlow";

type Target = { id: string; definitionName: string; providerReference: { scopeId: string; node: string; vmid: number } };
type Effects = { version: number; removeVmConfiguration: boolean; removeSnapshotMetadata: boolean; removeVmPermissions: boolean; removeVmFirewall: boolean; cleanupIpam: boolean; cleanupRecordedFleecingImages: boolean; purgeJobs: boolean; destroyUnreferencedDisks: boolean; retainDefinitionAndHistory: boolean };
type Review = { instanceId: string; providerScopeId: string; node: string; vmid: number; observation: { node: string; vmid: number; status: string; qmpStatus: string | null }; volumes: { volume: string; references: string[]; ownerVerified: boolean; effect: string }[]; snapshots: string[]; preservedMedia: string[]; preservedOwnedVolumes: string[]; effects: Effects; haManaged: boolean; replicationJobs: string[]; destructive: boolean; definitionRetained: boolean; purge: boolean; destroyUnreferencedDisks: boolean; checks: { code: string; status: string }[]; deletionPreconditions: string; executionEnabled: boolean };
type Operation = { id: string; kind: string; instanceId: string; status: string; providerTaskKnown: boolean; failure: string | null; result: null | { instanceId: string; providerScopeId: string; node: string; vmid: number; approvalHash: string; taskStatus: string; vmAbsenceVerified: boolean; volumesAbsenceVerified: boolean; definitionRetained: boolean; associationRetired: boolean } };
const labels: Record<string, string> = { instance_marker: "Instance marker matches", not_template: "Target is a VM", unlocked: "Configuration is unlocked", no_pending_changes: "No pending changes", configuration_unchanged: "Configuration stayed unchanged", delete_vm_stopped: "VM is already stopped", delete_not_protected: "Deletion protection is off", delete_scope_complete: "All disk references are reviewed", delete_volumes_owned: "Candidate volume ownership is verified", delete_snapshots_complete: "Snapshot work is complete", delete_scope_stable: "Cleanup scope stayed unchanged", delete_target_unchanged: "Reviewed target stayed unchanged", delete_not_ha_managed: "No HA resource depends on this VM", delete_no_replication_jobs: "No replication job depends on this VM", delete_management_stable: "Dependencies stayed unchanged", delete_provider_effect_scope: "Provider cleanup effects are understood" };
const failures: Record<string, string> = { deletion_disabled: "Deletion is disabled on the server.", deletion_target_changed: "The reviewed target or cleanup scope changed. Review it again before a new request.", deletion_preconditions_failed: "Deletion prerequisites no longer pass. Inspect the target.", approval_expired: "Confirmation expired before dispatch. Review the target again.", provider_submission_unconfirmed: "Submission outcome is unknown. The server will not repeat the DELETE.", provider_task_unconfirmed: "The provider task outcome is not confirmed. Refresh its saved operation.", deletion_state_unconfirmed: "The VM or reviewed volumes still need absence verification. Cleanup may be partial; no DELETE will be repeated.", provider_task_failed: "The provider task failed and cleanup may be partial. The target remains locked for inspection and read-only reconciliation." };
function fixedEffects(e: Effects) { return e.version === 1 && e.removeVmConfiguration && e.removeSnapshotMetadata && e.removeVmPermissions && e.removeVmFirewall && e.cleanupIpam && e.cleanupRecordedFleecingImages && !e.purgeJobs && !e.destroyUnreferencedDisks && e.retainDefinitionAndHistory && Object.keys(e).length === 10; }

export function RemoteDeletion({ connectionId, environment, disabled, onLock, onDeleted, onDone }: { connectionId: number; environment: Target; disabled: boolean; onLock: (locked: boolean) => void; onDeleted: (instanceId: string) => void; onDone?: () => void }) {
  const [review, setReview] = useState<Review | null>(null), [requestKey, setRequestKey] = useState("");
  const [confirmed, setConfirmed] = useState(false), [loss, setLoss] = useState(false), [submitted, setSubmitted] = useState(false), [busy, setBusy] = useState(false), [retired, setRetired] = useState(false);
  const [operation, setOperation] = useState<Operation | null>(null), [error, setError] = useState("");
  const generation = useRef(0);
  useEffect(() => { onLock(busy || review !== null || submitted); }, [busy, review, submitted, onLock]);
  useEffect(() => () => { generation.current++; onLock(false); }, [connectionId, onLock]);
  // Opening the action is the request to review it; the review is read-only.
  useEffect(() => { void prepare(); }, []);
  const eligible = review?.executionEnabled && review.deletionPreconditions === "passed" && review.destructive && review.definitionRetained && !review.purge && !review.destroyUnreferencedDisks && fixedEffects(review.effects) && !review.haManaged && review.replicationJobs.length === 0 && review.observation.node === review.node && review.observation.vmid === review.vmid && review.observation.status === "stopped" && (review.observation.qmpStatus === null || review.observation.qmpStatus === "stopped") && review.checks.length === 16 && new Set(review.checks.map(check => check.code)).size === 16 && review.checks.every(check => labels[check.code] && check.status === "passed") && review.volumes.every(volume => volume.ownerVerified && ["delete_owned_volume", "fleecing_cleanup"].includes(volume.effect));
  const terminal = operation && ["succeeded", "failed", "interrupted"].includes(operation.status);
  const report = (failure: unknown) => typeof failure === "string" ? failure : "Cannot complete the deletion request. Retain its request key.";
  async function prepare(event?: FormEvent) {
    event?.preventDefault(); if (disabled || busy || review || submitted || retired) return;
    const current = ++generation.current, key = crypto.randomUUID();
    setRequestKey(key); setBusy(true); setError(""); setConfirmed(false); setLoss(false); setOperation(null);
    try {
      const result = await invoke<Review>("prepare_remote_deletion", { connectionId, requestKey: key, instanceId: environment.id });
      if (generation.current !== current) return;
      const target = environment.providerReference;
      if (result.instanceId !== environment.id || result.providerScopeId !== target.scopeId || result.node !== target.node || result.vmid !== target.vmid || !result.destructive || !result.definitionRetained || result.purge || result.destroyUnreferencedDisks || !fixedEffects(result.effects)) throw "Deletion review changed the selected target or cleanup effects. Reload the workspace.";
      setReview(result);
    } catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  function accept(result: Operation) {
    if (!review || !/^[0-9a-f]{32}$/.test(result.id) || result.kind !== "vm_delete" || result.instanceId !== environment.id || (operation && result.id !== operation.id) || !["queued", "running", "succeeded", "failed", "interrupted", "uncertain"].includes(result.status) || (result.status === "failed" && result.providerTaskKnown)) throw "Deletion status does not match the retained request. Keep its original key.";
    if (result.status === "succeeded") {
      const e = result.result;
      if (!result.providerTaskKnown || result.failure !== null || !e || e.instanceId !== review.instanceId || e.providerScopeId !== review.providerScopeId || e.node !== review.node || e.vmid !== review.vmid || e.taskStatus !== "succeeded" || !/^[0-9a-f]{64}$/.test(e.approvalHash) || !e.vmAbsenceVerified || !e.volumesAbsenceVerified || !e.definitionRetained || !e.associationRetired) throw "Deletion success lacks verified task, absence or retained-definition evidence.";
      setRetired(true); onDeleted(environment.id);
    }
    setOperation(result);
  }
  async function submit() {
    if (!review || !eligible || !confirmed || !loss || disabled || busy || operation) return;
    const current = ++generation.current; setSubmitted(true); setBusy(true); setError("");
    try { const result = await invoke<Operation>("confirm_remote_deletion", { connectionId, requestKey, confirmed: true, acknowledgeDataLoss: true }); if (generation.current === current) accept(result); }
    catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  async function refresh() {
    if (!operation || disabled || busy) return;
    const current = ++generation.current; setBusy(true); setError("");
    try { const result = await invoke<Operation>("read_remote_deletion_operation", { connectionId, requestKey }); if (generation.current === current) accept(result); }
    catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  async function clear() {
    if (disabled || busy || (submitted && !terminal)) return;
    const current = ++generation.current; setBusy(true); setError("");
    try { await invoke("clear_remote_deletion", { connectionId, requestKey }); if (generation.current === current) { setReview(null); setRequestKey(""); setConfirmed(false); setLoss(false); setSubmitted(false); setOperation(null); onDone?.(); } }
    catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  const pending = !!operation && !terminal;
  useOperationPolling(pending && !disabled, operation, () => void refresh());
  return <section className="remote-deletion" aria-labelledby="deletion-heading" aria-busy={busy}>
    <h3 id="deletion-heading">Delete a VM</h3>
    <p>Target: {environment.definitionName} · {environment.providerReference.node} · VM {environment.providerReference.vmid}</p>
    <p>Permanently removes this VM and its reviewed volumes. Keep any data you need before confirming. The reusable definition and operation history remain.</p>
    {!review && !submitted && !busy && !retired && <button type="button" disabled={disabled} onClick={() => void prepare()}>Review again</button>}
    {busy && !operation && <p role="status">{review ? "Submitting…" : "Checking prerequisites and reviewing volumes…"}</p>}
    {review && <>
      <p>Reviewed deletion: {review.node} · VM {review.vmid} · {review.observation.status}</p>
      <h4>Volumes to remove</h4>
      {review.volumes.length ? <ul>{review.volumes.map(volume => <li key={volume.volume}><code>{volume.volume}</code> · {volume.effect === "fleecing_cleanup" ? "recorded fleecing cleanup" : volume.effect === "snapshot_reference" ? "unsupported snapshot-only cleanup" : "owned volume removal"} · ownership {volume.ownerVerified ? "verified" : "unconfirmed"}<ul>{volume.references.map(reference => <li key={reference}>{reference}</li>)}</ul></li>)}</ul> : <p>No candidate volumes in this reviewed scope.</p>}
      <p>Snapshots removed: {review.snapshots.join(", ") || "none"}</p>
      <h4>Preserved media</h4><p>{review.preservedMedia.join(", ") || "No media in this reviewed scope."}</p>
      <h4>Other owned volumes kept</h4><p>{review.preservedOwnedVolumes.join(", ") || "None observed on the inspected storages."}</p>
      <p>Cleanup removes VM configuration, snapshot metadata, VM permissions and firewall rules, with IPAM and recorded fleecing cleanup. It does not purge related jobs or bulk-delete unreferenced volumes. No automatic shutdown or start is requested.</p>
      <p>Reusable definition and operation history are retained. Preservation review covers the inspected storages.</p>
      {!review.executionEnabled && <p role="alert">Deletion is disabled on this server.</p>}
      <CheckSummary checks={review.checks} labels={labels} />
      {/* One explicit acknowledgement carries both the confirmation and the data-loss consent. */}
      <label className="consent"><input id="deletion-confirm" type="checkbox" checked={confirmed && loss} onChange={event => { setConfirmed(event.target.checked); setLoss(event.target.checked); }} disabled={!eligible || submitted || busy || disabled} /> Delete VM {review.vmid} on {review.node} with the reviewed volumes and snapshots. Its data is permanently lost.</label>
      <div className="actions-row">
        {!operation && <button type="button" className="danger solid" disabled={!eligible || !confirmed || !loss || busy || disabled || operation !== null} onClick={submit}>{submitted ? "Retry the same deletion request" : `Delete VM ${review.vmid}`}</button>}
        {!submitted && <button type="button" disabled={disabled || busy} onClick={clear}>Cancel</button>}
      </div>
      <p className="hint">Request key <code>{requestKey}</code></p>
    </>}
    {operation && <div className={`result ${operation.status}`}>
      <p role="status"><span className="status-dot" />{pending ? "In progress" : operation.status === "succeeded" ? "Done" : "Not confirmed"} · {operation.status} · <code>{operation.id}</code></p>
      {pending && <p className="hint">Updating automatically. You can close this panel; the server keeps working and the action stays in History.</p>}
      {operation.failure && <p role="alert">{failures[operation.failure] || "The server could not verify this deletion. Inspect the saved operation."}</p>}
      {operation.status === "succeeded" && <p>Deletion verified: the VM and reviewed volumes are absent. Definition and history retained.</p>}
      {terminal && <button type="button" className="primary" disabled={disabled || busy} onClick={clear}>Done</button>}
    </div>}
    {retired && !review && <p>This instance was deleted. Its definition and history are retained.</p>}
    {submitted && !operation && <p role="status">No verified operation ID received. Keep this request key and retry only this same confirmed request.</p>}
    <p>Disconnecting or closing the app does not cancel server work. Consent is not restored automatically after reconnecting.</p>
    {error && <p role="alert" className="error">{error}</p>}
  </section>;
}
