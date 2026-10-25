import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { CheckSummary, useOperationPolling, useAutoRun } from "./actionFlow";

type Action = "start" | "shutdown";
type Target = { id: string; definitionName: string; providerReference: { scopeId: string; node: string; vmid: number } };
type CheckStatus = "passed" | "failed" | "unknown";
type Observation = { node: string; vmid: number; status: "running" | "stopped"; qmpStatus: string | null };
type Review = { target: { instanceId: string; providerScopeId: string; node: string; vmid: number }; action: Action; preflight: { instanceId: string; providerScopeId: string; definitionHash: string; expectedMarker: string; configDigest: string; observation: Observation; checks: { code: string; status: CheckStatus }[]; startPreconditions: CheckStatus; shutdownPreconditions: CheckStatus; executionEnabled: boolean } };
type Operation = { id: string; kind: string; instanceId: string; status: "queued" | "running" | "succeeded" | "failed" | "interrupted" | "uncertain"; providerTaskKnown: boolean; failure: string | null; result: null | { action: Action; instanceId: string; providerScopeId: string; taskStatus: string; observation: Observation } };
const labels: Record<string, string> = { instance_marker: "Instance marker matches", not_template: "Target is a VM", unlocked: "Configuration is unlocked", no_pending_changes: "No pending changes", configuration_unchanged: "Configuration stayed unchanged" };
const failures: Record<string, string> = { lifecycle_disabled: "Lifecycle actions are disabled on the server.", approval_expired: "The confirmation expired before dispatch.", lifecycle_target_changed: "The reviewed target changed.", lifecycle_preconditions_failed: "Lifecycle prerequisites no longer pass.", provider_submission_unconfirmed: "Submission outcome is unknown. The server will not repeat the write.", provider_task_unconfirmed: "The provider task is not confirmed.", power_state_unconfirmed: "The requested power state is not confirmed.", process_interrupted: "The server was interrupted. Read the recorded operation before another action.", provider_task_failed: "The provider task failed." };

export function RemoteLifecycle({ connectionId, environment, action, disabled, onLock, onDone, onSettled }: { connectionId: number; environment: Target; action: Action; disabled: boolean; onLock: (locked: boolean) => void; onDone?: () => void; onSettled?: (succeeded: boolean) => void }) {
  const [review, setReview] = useState<Review | null>(null), [requestKey, setRequestKey] = useState("");
  const [confirmed, setConfirmed] = useState(false), [submitted, setSubmitted] = useState(false), [busy, setBusy] = useState(false);
  const [operation, setOperation] = useState<Operation | null>(null), [error, setError] = useState("");
  const generation = useRef(0);
  // Set once a definitive result has released the retained native request.
  const [released, setReleased] = useState(false);
  useEffect(() => { onLock(!released && (busy || review !== null || submitted)); }, [busy, review, submitted, released, onLock]);
  useEffect(() => () => { generation.current++; onLock(false); }, [connectionId, onLock]);
  // Opening the action is the request to review it; the review is read-only.
  useAutoRun(true, () => void prepare(), []);
  const verb = action === "start" ? "Start" : "Gracefully shut down";
  const eligible = !!review && review.preflight.executionEnabled && (action === "start" ? review.preflight.startPreconditions : review.preflight.shutdownPreconditions) === "passed" && review.preflight.checks.length === 5 && review.preflight.checks.every(check => labels[check.code] && check.status === "passed") && new Set(review.preflight.checks.map(check => check.code)).size === 5;
  const terminal = operation && (["succeeded", "failed"].includes(operation.status) || (operation.status === "interrupted" && !operation.providerTaskKnown));
  const report = (failure: unknown) => typeof failure === "string" ? failure : "Cannot complete the lifecycle request. Retain its request key.";
  async function prepare() {
    if (disabled || busy || review || submitted) return;
    const current = ++generation.current, key = crypto.randomUUID(); setBusy(true); setError(""); setConfirmed(false); setRequestKey(key); setOperation(null);
    const target = { instanceId: environment.id, providerScopeId: environment.providerReference.scopeId, node: environment.providerReference.node, vmid: environment.providerReference.vmid };
    try {
      const result = await invoke<Review>("prepare_remote_lifecycle", { connectionId, requestKey: key, target, action });
      if (generation.current !== current) return;
      const p = result.preflight;
      if (result.action !== action || result.target.instanceId !== target.instanceId || result.target.providerScopeId !== target.providerScopeId || result.target.node !== target.node || result.target.vmid !== target.vmid || p.instanceId !== target.instanceId || p.providerScopeId !== target.providerScopeId || p.observation.node !== target.node || p.observation.vmid !== target.vmid || p.expectedMarker !== `vmanyware-${target.instanceId}` || !/^[0-9a-f]{64}$/.test(p.definitionHash) || !/^[0-9a-f]{40}$/.test(p.configDigest)) throw "Lifecycle review changed the selected target or action. Reload the workspace.";
      setReview(result);
    } catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  function validateOperation(result: Operation) {
    if (!review || !/^[0-9a-f]{32}$/.test(result.id) || result.kind !== (action === "start" ? "vm_start" : "vm_shutdown") || result.instanceId !== environment.id || (operation && result.id !== operation.id) || !["queued", "running", "succeeded", "failed", "interrupted", "uncertain"].includes(result.status) || (result.status === "queued" && result.providerTaskKnown)) throw "Lifecycle operation does not match this request. Retain its original key.";
    const success = result.status === "succeeded", failed = ["failed", "interrupted", "uncertain"].includes(result.status);
    if (success !== (result.result !== null) || failed !== (result.failure !== null)) throw "Lifecycle operation lacks consistent outcome evidence.";
    if (success) {
      const e = result.result;
      if (!result.providerTaskKnown || !e || e.action !== action || e.instanceId !== environment.id || e.providerScopeId !== review.target.providerScopeId || e.taskStatus !== "succeeded" || e.observation.node !== review.target.node || e.observation.vmid !== review.target.vmid || e.observation.status !== (action === "start" ? "running" : "stopped")) throw "Lifecycle success lacks matching task and power-state evidence.";
    }
  }
  async function submit() {
    if (disabled || busy || !review || !eligible || operation) return;
    const current = ++generation.current; setSubmitted(true); setBusy(true); setError("");
    try { const result = await invoke<Operation>("confirm_remote_lifecycle", { connectionId, requestKey, confirmed: true }); if (generation.current === current) { validateOperation(result); setOperation(result); } }
    catch (failure) {
      if (generation.current === current) {
        // These fixed native refusals happen before submitted is set or any POST.
        if (failure === "The lifecycle review expired. Prepare and review again." || failure === "Lifecycle execution is disabled or its preconditions have not passed.") { setSubmitted(false); setConfirmed(false); }
        setError(report(failure));
      }
    }
    finally { if (generation.current === current) setBusy(false); }
  }
  async function refresh() {
    if (disabled || busy || !operation) return;
    const current = ++generation.current; setBusy(true); setError("");
    try { const result = await invoke<Operation>("read_remote_lifecycle_operation", { connectionId, requestKey }); if (generation.current === current) { validateOperation(result); setOperation(result); } }
    catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  async function clear() {
    if (disabled || busy || !requestKey || (submitted && !terminal)) return;
    const current = ++generation.current; setBusy(true); setError("");
    try { await invoke("clear_remote_lifecycle", { connectionId, requestKey }); if (generation.current === current) { setReview(null); setRequestKey(""); setConfirmed(false); setSubmitted(false); setOperation(null); onDone?.(); } }
    catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  // A definitive result needs no acknowledgement: release the retained request automatically.
  async function release() {
    try { await invoke("clear_remote_lifecycle", { connectionId, requestKey }); setReleased(true); onSettled?.(operation?.status === "succeeded"); }
    catch { /* Keep the explicit Done fallback below. */ }
  }
  useAutoRun(!!terminal && !released, () => void release(), [operation?.status]);
  const pending = !!operation && !terminal;
  useOperationPolling(pending && !disabled, operation, () => void refresh());
  return <section className="remote-lifecycle" aria-labelledby="remote-lifecycle-heading" aria-busy={busy}>
    <h3 id="remote-lifecycle-heading">{verb} a remote VM</h3>
    <p>Target: {environment.definitionName} · {environment.providerReference.node} · VM {environment.providerReference.vmid}</p>
    <p>{action === "shutdown" ? "Graceful shutdown asks the guest to stop; unsaved work may be lost. This does not force power off." : "Starting changes the VM power state. It does not verify guest readiness or SSH access."}</p>
    {!review && !submitted && !busy && <button type="button" disabled={disabled} onClick={prepare}>Review again</button>}
    {busy && !operation && <p role="status">{review ? "Submitting…" : "Checking prerequisites…"}</p>}
    {review && <>
      <p>Observed power: <strong>{review.preflight.observation.status}</strong> · QMP: {review.preflight.observation.qmpStatus ?? "unknown"}. This observation may become stale.</p>
      <CheckSummary checks={review.preflight.checks} labels={labels} />
      {!eligible && (review.preflight.observation.status === (action === "start" ? "running" : "stopped")
        ? <p className="notice">VM {review.target.vmid} is already {action === "start" ? "running" : "stopped"}. Nothing to do.</p>
        : <p role="alert">Lifecycle prerequisites or server capability do not permit this action.</p>)}
      {!submitted && <div className="actions-row">{eligible && <button type="button" className="primary" disabled={disabled || busy} onClick={submit}>{action === "start" ? "Start" : "Shut down"} VM {review.target.vmid}</button>}<button type="button" disabled={disabled || busy} onClick={clear}>{eligible ? "Cancel" : "Close"}</button></div>}
      <p className="hint">Request key <code>{requestKey}</code></p>
    </>}
    {submitted && !operation && <><p role="status">No verified lifecycle operation ID received. Retain this request key and retry only the same confirmed action.</p><button type="button" disabled={disabled || busy} onClick={submit}>Retry same lifecycle request</button></>}
    {operation && <div className={`result ${operation.status}`}>
      <p role="status"><span className="status-dot" />{pending ? "In progress" : operation.status === "succeeded" ? "Done" : "Not confirmed"} · {operation.status} · <code>{operation.id}</code></p>
      {pending && <p className="hint">Updating automatically. You can close this panel; the server keeps working and the action stays in History.</p>}
      {operation.status === "succeeded" && <p>Provider task and requested power state verified: {operation.result?.observation.status}. Guest readiness and SSH access are not verified.</p>}
      {operation.failure && <p role="alert">{failures[operation.failure] || "The action did not confirm success. Inspect its operation and VM before another action."}</p>}
      {(operation.status === "uncertain" || (operation.status === "interrupted" && operation.providerTaskKnown)) && <p>The request remains retained and is re-read without repeating the action.</p>}
      {terminal && !released && <button type="button" className="primary" disabled={disabled || busy} onClick={clear}>Done</button>}
    </div>}
    {error && <p role="alert" className="error">{error}</p>}
  </section>;
}
