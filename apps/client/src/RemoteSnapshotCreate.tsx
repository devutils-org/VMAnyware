import { useEffect, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { CheckSummary, useOperationPolling, useAutoRun } from "./actionFlow";

type Target = { id: string; definitionName: string; providerReference: { scopeId: string; node: string; vmid: number } };
type CheckStatus = "passed" | "failed" | "unknown";
type Review = { instanceId: string; providerScopeId: string; definitionHash: string; node: string; vmid: number; configDigest: string; request: { name: string; description: string }; includesRam: boolean; checks: { code: string; status: CheckStatus }[]; creationPreconditions: CheckStatus; executionEnabled: boolean };
type Operation = { id: string; kind: string; instanceId: string; providerTaskKnown: boolean; status: "queued" | "running" | "succeeded" | "failed" | "interrupted" | "uncertain"; failure: string | null; result: null | { instanceId: string; providerScopeId: string; node: string; vmid: number; taskStatus: string; snapshot: { name: string; description: string; includesRam: boolean | null; providerState: string | null } } };
const labels: Record<string, string> = { instance_marker: "Instance marker matches", not_template: "Target is a VM", unlocked: "Configuration is unlocked", no_pending_changes: "No pending changes", configuration_unchanged: "Configuration stayed unchanged", snapshot_supported: "Snapshot support", snapshot_name_available: "Name is available", snapshot_target_unchanged: "Target stayed unchanged", snapshot_tree_complete: "No incomplete snapshot work" };
const failures: Record<string, string> = { snapshot_disabled: "Snapshot creation is disabled on the server.", snapshot_target_changed: "The reviewed target changed. Inspect the VM before preparing another snapshot.", snapshot_preconditions_failed: "Snapshot prerequisites no longer pass. Inspect the VM before another request.", approval_expired: "Confirmation expired before dispatch. Review the target again.", provider_submission_unconfirmed: "Submission outcome is unknown. The server will not repeat the write.", provider_task_unconfirmed: "The provider task is not yet confirmed.", snapshot_state_unconfirmed: "The exact completed disk-only snapshot is not yet verified.", provider_task_failed: "The provider task failed. Partial snapshot state may remain." };

export function RemoteSnapshotCreate({ connectionId, environment, disabled, onLock, onDone, onSettled }: { connectionId: number; environment: Target; disabled: boolean; onLock: (locked: boolean) => void; onDone?: () => void; onSettled?: (succeeded: boolean) => void }) {
  const [name, setName] = useState("before-work"), [description, setDescription] = useState("");
  const [review, setReview] = useState<Review | null>(null), [requestKey, setRequestKey] = useState("");
  const [confirmed, setConfirmed] = useState(false), [submitted, setSubmitted] = useState(false), [busy, setBusy] = useState(false);
  const [operation, setOperation] = useState<Operation | null>(null), [error, setError] = useState("");
  const generation = useRef(0);
  // Set once a definitive result has released the retained native request.
  const [released, setReleased] = useState(false);
  useEffect(() => { onLock(!released && (busy || review !== null || submitted)); }, [busy, review, submitted, released, onLock]);
  useEffect(() => () => { generation.current++; onLock(false); }, [connectionId, onLock]);
  const eligible = review?.executionEnabled && review.creationPreconditions === "passed" && review.checks.length === 9 && review.checks.every(check => labels[check.code] && check.status === "passed") && new Set(review.checks.map(check => check.code)).size === 9;
  const locked = disabled || busy || review !== null || submitted;
  const terminal = operation && ["succeeded", "failed", "interrupted"].includes(operation.status);
  const report = (failure: unknown) => typeof failure === "string" ? failure : "Cannot complete the snapshot request. Retain its request key.";
  async function prepare(event: FormEvent) {
    event.preventDefault(); if (locked) return;
    const current = ++generation.current, key = crypto.randomUUID();
    setRequestKey(key); setBusy(true); setError(""); setConfirmed(false); setOperation(null);
    try {
      const result = await invoke<Review>("prepare_remote_snapshot", { connectionId, requestKey: key, instanceId: environment.id, name, description });
      if (generation.current !== current) return;
      const target = environment.providerReference;
      if (result.instanceId !== environment.id || result.providerScopeId !== target.scopeId || result.node !== target.node || result.vmid !== target.vmid || result.request.name !== name || result.request.description !== description || result.includesRam !== false) throw "Snapshot review changed the selected target or request. Reload the workspace.";
      setReview(result);
    } catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  function validateOperation(result: Operation) {
    if (!review || !/^[0-9a-f]{32}$/.test(result.id) || result.kind !== "vm_snapshot_create" || result.instanceId !== environment.id || (operation && result.id !== operation.id)) throw "The snapshot operation belongs to a different request. Retain the original request key.";
    if (result.status === "succeeded") {
      const evidence = result.result;
      if (!result.providerTaskKnown || result.failure !== null || !evidence || evidence.instanceId !== review.instanceId || evidence.providerScopeId !== review.providerScopeId || evidence.node !== review.node || evidence.vmid !== review.vmid || evidence.taskStatus !== "succeeded" || evidence.snapshot.name !== review.request.name || evidence.snapshot.description !== review.request.description || evidence.snapshot.includesRam !== false || evidence.snapshot.providerState !== null) throw "Snapshot success is missing the reviewed task and disk-only evidence.";
    }
  }
  async function submit() {
    if (!review || !eligible || busy || disabled || operation) return;
    const current = ++generation.current; setSubmitted(true); setBusy(true); setError("");
    try { const result = await invoke<Operation>("confirm_remote_snapshot", { connectionId, requestKey, confirmed: true }); if (generation.current === current) { validateOperation(result); setOperation(result); } }
    catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  async function refresh() {
    if (!operation || busy || disabled) return;
    const current = ++generation.current; setBusy(true); setError("");
    try { const result = await invoke<Operation>("read_remote_snapshot_operation", { connectionId, requestKey }); if (generation.current === current) { validateOperation(result); setOperation(result); } }
    catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  async function clear() {
    if (disabled || busy || (submitted && !terminal)) return;
    const current = ++generation.current; setBusy(true); setError("");
    try { await invoke("clear_remote_snapshot", { connectionId, requestKey }); if (generation.current === current) { setReview(null); setRequestKey(""); setConfirmed(false); setSubmitted(false); setOperation(null); onDone?.(); } }
    catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  // A definitive result needs no acknowledgement: release the retained request automatically.
  async function release() {
    try { await invoke("clear_remote_snapshot", { connectionId, requestKey }); setReleased(true); onSettled?.(operation?.status === "succeeded"); }
    catch { /* Keep the explicit Done fallback below. */ }
  }
  useAutoRun(!!terminal && !released, () => void release(), [operation?.status]);
  const pending = !!operation && !terminal;
  useOperationPolling(pending && !disabled, operation, () => void refresh());
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
        <button type="button" className="primary" disabled={disabled || busy || !eligible} onClick={submit}>Create snapshot {review.request.name}</button>
        <button type="button" disabled={disabled || busy} onClick={clear}>Cancel</button>
      </div>}
      <p className="hint">Disk only, RAM excluded · request key <code>{requestKey}</code></p>
    </>}
    {submitted && !operation && <>
      <p role="status">Submission outcome unknown. Keep this request key and retry only the same confirmed snapshot.</p>
      <button type="button" disabled={disabled || busy} onClick={submit}>Retry same snapshot request</button>
    </>}
    {operation && <div className={`result ${operation.status}`}>
      <p role="status"><span className="status-dot" />{pending ? "In progress" : operation.status === "succeeded" ? "Done" : "Not confirmed"} · {operation.status} · <code>{operation.id}</code></p>
      {pending && <p className="hint">Updating automatically. You can close this panel; the server keeps working and the action stays in History.</p>}
      {operation.status === "succeeded" && <p>Snapshot creation verified: {review?.request.name}. Disk only; restoration and guest consistency remain unverified.</p>}
      {operation.failure && <p role="alert">{failures[operation.failure] || "The operation did not confirm success. Inspect this VM before another action."}</p>}
      {operation.status === "uncertain" && <p>This request remains retained and is re-read without submitting another creation.</p>}
      {terminal && !released && <button type="button" className="primary" disabled={disabled || busy} onClick={clear}>Done</button>}
    </div>}
    {error && <p role="alert" className="error">{error}</p>}
  </section>;
}
