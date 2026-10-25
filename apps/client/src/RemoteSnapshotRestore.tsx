import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { CheckSummary, useReviewedOperation } from "./actionFlow";

type Target = { id: string; definitionName: string; providerReference: { scopeId: string; node: string; vmid: number } };
type CheckStatus = "passed" | "failed" | "unknown";
type Observation = { node: string; vmid: number; status: string; qmpStatus: string | null };
type Review = { instanceId: string; providerScopeId: string; definitionHash: string; node: string; vmid: number; configDigest: string; request: { snapshotName: string }; observation: Observation; selectedSnapshot: null | { name: string; description: string; includesRam: boolean | null; providerState: string | null }; snapshotConfigHash: string | null; destructive: boolean; startsVm: boolean; checks: { code: string; status: CheckStatus }[]; restorePreconditions: CheckStatus; executionEnabled: boolean };
type Operation = { id: string; kind: string; instanceId: string; providerTaskKnown: boolean; status: "queued" | "running" | "succeeded" | "failed" | "interrupted" | "uncertain"; failure: string | null; result: null | { instanceId: string; providerScopeId: string; node: string; vmid: number; taskStatus: string; snapshotName: string; snapshotConfigHash: string; observation: Observation } };
const labels: Record<string, string> = { instance_marker: "Instance marker matches", not_template: "Target is a VM", unlocked: "Configuration is unlocked", no_pending_changes: "No pending changes", configuration_unchanged: "Configuration stayed unchanged", restore_vm_stopped: "VM is already stopped", restore_snapshot_exists: "Selected snapshot exists", restore_snapshot_complete: "No incomplete snapshot work", restore_snapshot_disk_only: "Snapshot excludes RAM", restore_snapshot_identity: "Saved configuration belongs to this instance", restore_snapshot_stable: "Selected snapshot stayed unchanged", restore_snapshot_storage: "Saved configuration supports snapshots", restore_target_unchanged: "Live target stayed unchanged" };
const failures: Record<string, string> = { restore_disabled: "Snapshot restoration is disabled on the server.", restore_target_changed: "The reviewed VM or snapshot changed. Inspect the target before preparing another restore.", restore_preconditions_failed: "Restore prerequisites no longer pass. Inspect the VM and snapshot before another request.", approval_expired: "Confirmation expired before dispatch. Review the target again.", provider_submission_unconfirmed: "Submission outcome is unknown. The server will not repeat the write.", provider_task_unconfirmed: "The provider rollback task is not yet confirmed.", restore_state_unconfirmed: "The selected snapshot and stopped target are not yet verified after rollback.", provider_task_failed: "The provider rollback task failed. Partial changes may remain; inspect this VM." };
const stopped = (observation: Observation, node: string, vmid: number) => observation.node === node && observation.vmid === vmid && observation.status === "stopped" && (observation.qmpStatus === null || observation.qmpStatus === "stopped");

export function RemoteSnapshotRestore({ connectionId, environment, snapshotName, disabled, onLock, onDone, onSettled }: { connectionId: number; environment: Target; snapshotName: string; disabled: boolean; onLock: (locked: boolean) => void; onDone?: () => void; onSettled?: (succeeded: boolean) => void }) {
  // One explicit acknowledgement carries both the confirmation and the data-loss consent.
  const [consent, setConsent] = useState(false);
  const flow = useReviewedOperation<null, Review, Operation>({
    async prepare(requestKey) {
      const result = await invoke<Review>("prepare_remote_snapshot_restore", { connectionId, requestKey, instanceId: environment.id, snapshotName });
      const target = environment.providerReference;
      if (result.instanceId !== environment.id || result.providerScopeId !== target.scopeId || result.node !== target.node || result.vmid !== target.vmid || result.request.snapshotName !== snapshotName || !result.destructive || result.startsVm || (result.selectedSnapshot !== null && result.selectedSnapshot.name !== snapshotName)) throw "Restore review changed the selected VM or snapshot. Reload the workspace.";
      return result;
    },
    submit: requestKey => invoke<Operation>("confirm_remote_snapshot_restore", { connectionId, requestKey, confirmed: true, acknowledgeDataLoss: true }),
    read: requestKey => invoke<Operation>("read_remote_snapshot_restore_operation", { connectionId, requestKey }),
    clear: async requestKey => { await invoke("clear_remote_snapshot_restore", { connectionId, requestKey }); },
    validate(result, review, previous) {
      if (!/^[0-9a-f]{32}$/.test(result.id) || result.kind !== "vm_snapshot_restore" || result.instanceId !== environment.id || (previous && result.id !== previous.id)) throw "The snapshot operation belongs to a different request. Retain the original request key.";
      if (result.status === "succeeded") {
        const evidence = result.result;
        if (!result.providerTaskKnown || result.failure !== null || !evidence || evidence.instanceId !== review.instanceId || evidence.providerScopeId !== review.providerScopeId || evidence.node !== review.node || evidence.vmid !== review.vmid || evidence.taskStatus !== "succeeded" || evidence.snapshotName !== review.request.snapshotName || evidence.snapshotConfigHash !== review.snapshotConfigHash || !stopped(evidence.observation, review.node, review.vmid)) throw "Restore success is missing the reviewed task, snapshot or stopped-target evidence.";
      }
    },
    isTerminal: operation => ["succeeded", "failed", "interrupted"].includes(operation.status),
  }, { fallbackError: "Cannot complete the snapshot request. Retain its request key.", disabled, autoReview: null, onLock, onSettled, onDone });
  const { review, operation, submitted, released, busy, error, requestKey } = flow.state;
  const eligible = review?.executionEnabled && review.restorePreconditions === "passed" && review.checks.length === 13 && review.destructive && !review.startsVm && !!review.snapshotConfigHash && /^[0-9a-f]{64}$/.test(review.snapshotConfigHash) && review.selectedSnapshot?.name === snapshotName && review.selectedSnapshot?.includesRam === false && review.selectedSnapshot?.providerState === null && stopped(review.observation, review.node, review.vmid) && review.checks.every(check => labels[check.code] && check.status === "passed") && new Set(review.checks.map(check => check.code)).size === 13;
  return <section className="remote-snapshot-restore" aria-labelledby="snapshot-restore-heading" aria-busy={busy}>
    <h3 id="snapshot-restore-heading">Restore a snapshot</h3>
    <p>Target: {environment.definitionName} · {environment.providerReference.node} · VM {environment.providerReference.vmid}</p>
    <p>Restoring discards current VM data and settings changed since the selected snapshot. Snapshots depend on the original VM/storage and are not independent backups. This path requires an already stopped VM and a disk-only snapshot; it will not shut down or start the VM.</p>
    <p>Selected snapshot: <strong>{snapshotName}</strong></p>
    {!review && !submitted && !busy && <button type="button" disabled={disabled} onClick={() => flow.review(null)}>Review again</button>}
    {busy && !operation && <p role="status">{review ? "Submitting…" : "Checking prerequisites…"}</p>}
    {review && <>
      <p>{review.selectedSnapshot?.description || "No description."}</p>
      <CheckSummary checks={review.checks} labels={labels} />
      {review.checks.some(check => check.code === "restore_vm_stopped" && check.status !== "passed") && <p className="notice">Shut down the VM first, then open Restore again. Restoring never stops or starts the VM.</p>}
      {!review.executionEnabled && <p role="alert">Snapshot restoration is disabled on this server.</p>}
      {!eligible && <p role="alert">Restore prerequisites or server capability do not permit restoration.</p>}
      <label className="consent"><input type="checkbox" checked={consent} onChange={event => setConsent(event.target.checked)} disabled={disabled || busy || submitted || !eligible} />Restore {review.request.snapshotName} on {environment.definitionName} (VM {review.vmid}) and discard current VM data and settings changed since this snapshot.</label>
      {!submitted && <div className="actions-row">
        <button type="button" className="danger solid" disabled={disabled || busy || !eligible || !consent} onClick={flow.submit}>Restore snapshot</button>
        <button type="button" disabled={disabled || busy} onClick={flow.close}>Cancel</button>
      </div>}
      <p className="hint">Disk only, RAM excluded · request key <code>{requestKey}</code></p>
    </>}
    {submitted && !operation && <>
      <p role="status">Submission outcome unknown. Keep this request key and retry only the same confirmed restore.</p>
      <button type="button" disabled={disabled || busy} onClick={flow.submit}>Retry same restore request</button>
    </>}
    {operation && <div className={`result ${operation.status}`}>
      <p role="status"><span className="status-dot" />{flow.pending ? "In progress" : operation.status === "succeeded" ? "Done" : "Not confirmed"} · {operation.status} · <code>{operation.id}</code></p>
      {flow.pending && <p className="hint">Updating automatically. You can close this panel; the server keeps working and the action stays in History.</p>}
      {operation.status === "succeeded" && <p>Provider rollback verified: {review?.request.snapshotName}. VM observed stopped. Recovered guest contents and application consistency remain unverified.</p>}
      {operation.failure && <p role="alert">{failures[operation.failure] || "The operation did not confirm success. Inspect this VM before another action."}</p>}
      {operation.status === "uncertain" && <p>This request remains retained and is re-read without submitting another rollback.</p>}
      {flow.terminal && !released && <button type="button" className="primary" disabled={disabled || busy} onClick={flow.close}>Done</button>}
    </div>}
    {error && <p role="alert" className="error">{error}</p>}
  </section>;
}
