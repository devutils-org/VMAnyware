import { useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { CheckSummary, useReviewedOperation } from "./actionFlow";

type Target = { id: string; definitionName: string; providerReference: { scopeId: string; node: string; vmid: number } };
type CheckStatus = "passed" | "failed" | "unknown";
type Review = { instanceId: string; providerScopeId: string; definitionHash: string; node: string; vmid: number; configDigest: string; request: { name: string; description: string }; includesRam: boolean; checks: { code: string; status: CheckStatus }[]; creationPreconditions: CheckStatus; executionEnabled: boolean };
type Operation = { id: string; kind: string; instanceId: string; providerTaskKnown: boolean; status: "queued" | "running" | "succeeded" | "failed" | "interrupted" | "uncertain"; failure: string | null; result: null | { instanceId: string; providerScopeId: string; node: string; vmid: number; taskStatus: string; snapshot: { name: string; description: string; includesRam: boolean | null; providerState: string | null } } };
const labels: Record<string, string> = { instance_marker: "Instance marker matches", not_template: "Target is a VM", unlocked: "Configuration is unlocked", no_pending_changes: "No pending changes", configuration_unchanged: "Configuration stayed unchanged", snapshot_supported: "Snapshot support", snapshot_name_available: "Name is available", snapshot_target_unchanged: "Target stayed unchanged", snapshot_tree_complete: "No incomplete snapshot work" };
const failures: Record<string, string> = { snapshot_disabled: "Snapshot creation is disabled on the server.", snapshot_target_changed: "The reviewed target changed. Inspect the VM before preparing another snapshot.", snapshot_preconditions_failed: "Snapshot prerequisites no longer pass. Inspect the VM before another request.", approval_expired: "Confirmation expired before dispatch. Review the target again.", provider_submission_unconfirmed: "Submission outcome is unknown. The server will not repeat the write.", provider_task_unconfirmed: "The provider task is not yet confirmed.", snapshot_state_unconfirmed: "The exact completed disk-only snapshot is not yet verified.", provider_task_failed: "The provider task failed. Partial snapshot state may remain." };

export function RemoteSnapshotCreate({ connectionId, environment, disabled, onLock, onDone, onSettled }: { connectionId: number; environment: Target; disabled: boolean; onLock: (locked: boolean) => void; onDone?: () => void; onSettled?: (succeeded: boolean) => void }) {
  const [name, setName] = useState("before-work"), [description, setDescription] = useState("");
  const flow = useReviewedOperation<{ name: string; description: string }, Review, Operation>({
    async prepare(requestKey, input) {
      const result = await invoke<Review>("prepare_remote_snapshot", { connectionId, requestKey, instanceId: environment.id, name: input.name, description: input.description });
      const target = environment.providerReference;
      if (result.instanceId !== environment.id || result.providerScopeId !== target.scopeId || result.node !== target.node || result.vmid !== target.vmid || result.request.name !== input.name || result.request.description !== input.description || result.includesRam !== false) throw "Snapshot review changed the selected target or request. Reload the workspace.";
      return result;
    },
    submit: requestKey => invoke<Operation>("confirm_remote_snapshot", { connectionId, requestKey, confirmed: true }),
    read: requestKey => invoke<Operation>("read_remote_snapshot_operation", { connectionId, requestKey }),
    clear: async requestKey => { await invoke("clear_remote_snapshot", { connectionId, requestKey }); },
    validate(result, review, previous) {
      if (!/^[0-9a-f]{32}$/.test(result.id) || result.kind !== "vm_snapshot_create" || result.instanceId !== environment.id || (previous && result.id !== previous.id)) throw "The snapshot operation belongs to a different request. Retain the original request key.";
      if (result.status === "succeeded") {
        const evidence = result.result;
        if (!result.providerTaskKnown || result.failure !== null || !evidence || evidence.instanceId !== review.instanceId || evidence.providerScopeId !== review.providerScopeId || evidence.node !== review.node || evidence.vmid !== review.vmid || evidence.taskStatus !== "succeeded" || evidence.snapshot.name !== review.request.name || evidence.snapshot.description !== review.request.description || evidence.snapshot.includesRam !== false || evidence.snapshot.providerState !== null) throw "Snapshot success is missing the reviewed task and disk-only evidence.";
      }
    },
    isTerminal: operation => ["succeeded", "failed", "interrupted"].includes(operation.status),
  }, { fallbackError: "Cannot complete the snapshot request. Retain its request key.", disabled, onLock, onSettled, onDone });
  const { review, operation, submitted, released, busy, error, requestKey } = flow.state;
  const eligible = review?.executionEnabled && review.creationPreconditions === "passed" && review.checks.length === 9 && review.checks.every(check => labels[check.code] && check.status === "passed") && new Set(review.checks.map(check => check.code)).size === 9;
  const locked = disabled || busy || review !== null || submitted;
  function prepare(event: FormEvent) { event.preventDefault(); if (!locked) flow.review({ name, description }); }
  return <section className="remote-snapshot-create" aria-labelledby="snapshot-create-heading" aria-busy={busy}>
    <h3 id="snapshot-create-heading">Create a disk-only snapshot</h3>
    <p>Target: {environment.definitionName} · {environment.providerReference.node} · VM {environment.providerReference.vmid}</p>
    <p>Snapshots depend on the original VM and storage. They are not independent backups. Guest consistency and restoration are not verified by this action.</p>
    <form onSubmit={prepare}>
      <label htmlFor="snapshot-name">Snapshot name</label>
      <input id="snapshot-name" value={name} onChange={event => setName(event.target.value)} minLength={2} maxLength={40} pattern="[A-Za-z][A-Za-z0-9_-]{1,39}" required disabled={locked} autoCapitalize="none" autoCorrect="off" spellCheck={false} />
      <label htmlFor="snapshot-description">Description (optional, maximum 4 KiB)</label>
      <textarea id="snapshot-description" value={description} onChange={event => setDescription(event.target.value)} maxLength={4096} disabled={locked} />
      {!review && !submitted && <button type="submit" disabled={locked || name === "current" || new TextEncoder().encode(description).length > 4096}>Continue</button>}
    </form>
    {busy && !operation && <p role="status">{review ? "Submitting…" : "Checking prerequisites…"}</p>}
    {review && <>
      <CheckSummary checks={review.checks} labels={labels} />
      {!review.executionEnabled && <p role="alert">Snapshot creation is disabled on this server.</p>}
      {!eligible && <p role="alert">Snapshot prerequisites or server capability do not permit creation.</p>}
      {!submitted && <div className="actions-row">
        <button type="button" className="primary" disabled={disabled || busy || !eligible} onClick={flow.submit}>Create snapshot {review.request.name}</button>
        <button type="button" disabled={disabled || busy} onClick={flow.close}>Cancel</button>
      </div>}
      <p className="hint">Disk only, RAM excluded · request key <code>{requestKey}</code></p>
    </>}
    {submitted && !operation && <>
      <p role="status">Submission outcome unknown. Keep this request key and retry only the same confirmed snapshot.</p>
      <button type="button" disabled={disabled || busy} onClick={flow.submit}>Retry same snapshot request</button>
    </>}
    {operation && <div className={`result ${operation.status}`}>
      <p role="status"><span className="status-dot" />{flow.pending ? "In progress" : operation.status === "succeeded" ? "Done" : "Not confirmed"} · {operation.status} · <code>{operation.id}</code></p>
      {flow.pending && <p className="hint">Updating automatically. You can close this panel; the server keeps working and the action stays in History.</p>}
      {operation.status === "succeeded" && <p>Snapshot creation verified: {review?.request.name}. Disk only; restoration and guest consistency remain unverified.</p>}
      {operation.failure && <p role="alert">{failures[operation.failure] || "The operation did not confirm success. Inspect this VM before another action."}</p>}
      {operation.status === "uncertain" && <p>This request remains retained and is re-read without submitting another creation.</p>}
      {flow.terminal && !released && <button type="button" className="primary" disabled={disabled || busy} onClick={flow.close}>Done</button>}
    </div>}
    {error && <p role="alert" className="error">{error}</p>}
  </section>;
}
