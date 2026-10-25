import { useEffect, useId, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAutoRun } from "./actionFlow";

type Environment = { id: string; definitionName: string; providerReference: { node: string; vmid: number } };
type Action = "start" | "shutdown";
type Status = "queued" | "running" | "succeeded" | "failed" | "interrupted" | "uncertain";
type Entry = { id: string; action: Action; status: Status; failure: string | null; providerTaskKnown: boolean; createdAt: string };
type History = { instanceId: string; operations: Entry[] };
type Saved = { action: Action; submissionStarted: boolean; target: { instanceId: string; providerScopeId: string; definitionHash: string; node: string; vmid: number }; operation: { id: string; kind: string; instanceId: string; status: Status; providerTaskKnown: boolean; failure: string | null; result: null | { action: Action; instanceId: string; providerScopeId: string; taskStatus: string; observation: { node: string; vmid: number; status: string } } } };
const statuses = ["queued", "running", "succeeded", "failed", "interrupted", "uncertain"];
const actions = ["start", "shutdown"];
export function RemoteLifecycleRecovery({ connectionId, environment, disabled, focus }: { connectionId: number; environment: Environment; disabled: boolean; focus?: string }) {
  const [history, setHistory] = useState<History | null>(null), [operationId, setOperationId] = useState("");
  const [saved, setSaved] = useState<Saved | null>(null), [busy, setBusy] = useState(false), [error, setError] = useState("");
  const generation = useRef(0), inputId = useId();
  useEffect(() => () => { generation.current++; }, [connectionId, environment.id]);
  const report = (failure: unknown) => typeof failure === "string" ? failure : "Cannot read saved lifecycle work. Please retry the read.";
  async function load() {
    if (disabled || busy) return;
    const current = ++generation.current; setBusy(true); setError(""); setHistory(null); setSaved(null);
    try {
      const result = await invoke<History>("read_remote_lifecycle_history", { connectionId, instanceId: environment.id });
      if (generation.current !== current) return;
      if (result.instanceId !== environment.id || result.operations.length > 100 || new Set(result.operations.map(op => op.id)).size !== result.operations.length || result.operations.some(op => !/^[0-9a-f]{32}$/.test(op.id) || !actions.includes(op.action) || !statuses.includes(op.status))) throw "Saved lifecycle history does not match this instance.";
      setHistory(result);
    } catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  async function inspect(id: string) {
    if (disabled || busy) return;
    if (!/^[0-9a-f]{32}$/.test(id)) { setError("Enter a saved operation ID of 32 lowercase hexadecimal characters."); return; }
    const current = ++generation.current; setOperationId(id); setSaved(null); setBusy(true); setError("");
    try {
      const result = await invoke<Saved>("recover_remote_lifecycle", { connectionId, instanceId: environment.id, operationId: id });
      if (generation.current !== current) return;
      const target = result.target, op = result.operation;
      if (!actions.includes(result.action) || target.instanceId !== environment.id || !/^[0-9a-f]{64}$/.test(target.providerScopeId) || !/^[0-9a-f]{64}$/.test(target.definitionHash) || !/^[A-Za-z0-9][A-Za-z0-9._-]{0,62}$/.test(target.node) || !Number.isInteger(target.vmid) || target.vmid < 100 || target.vmid > 999999999 || op.id !== id || op.instanceId !== environment.id || op.kind !== (result.action === "start" ? "vm_start" : "vm_shutdown") || !statuses.includes(op.status) || (op.providerTaskKnown && !result.submissionStarted) || (op.status === "queued" && (result.submissionStarted || op.providerTaskKnown))) throw "Saved lifecycle operation belongs to a different instance or action.";
      const success = op.status === "succeeded", failed = ["failed", "interrupted", "uncertain"].includes(op.status);
      if (success !== (op.result !== null) || failed !== (op.failure !== null)) throw "Saved lifecycle outcome is inconsistent.";
      if (success) {
        const evidence = op.result;
        if (!op.providerTaskKnown || !evidence || evidence.action !== result.action || evidence.instanceId !== environment.id || evidence.providerScopeId !== target.providerScopeId || evidence.taskStatus !== "succeeded" || evidence.observation.node !== target.node || evidence.observation.vmid !== target.vmid || evidence.observation.status !== (result.action === "start" ? "running" : "stopped")) throw "Saved lifecycle success lacks matching task and power-state evidence.";
      }
      setSaved(result);
    } catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  function submit(event: FormEvent) { event.preventDefault(); void inspect(operationId); }
  // Opened from the activity timeline: read that operation directly.
  useAutoRun(!!focus, () => void inspect(focus!), [focus]);
  return <section className="lifecycle-recovery" aria-label={`Saved lifecycle work for VM ${environment.providerReference.vmid}`} aria-busy={busy}>
    {!focus && <>
    <h4>Saved start/shutdown operations</h4>
    <p>Read recorded work for {environment.definitionName}. These controls never repeat an action or restore confirmation.</p>
    <button type="button" disabled={disabled || busy} onClick={load}>Find lifecycle operations for VM {environment.providerReference.vmid}</button>
    {history && (history.operations.length === 0 ? <p role="status">No lifecycle operations in this instance's recent history.</p> : <ul>{history.operations.map(op => <li key={op.id}><button type="button" disabled={disabled || busy} onClick={() => inspect(op.id)}>Inspect lifecycle operation {op.id}</button> · {op.action === "start" ? "start" : "graceful shutdown"} · {op.status} · {op.createdAt}</li>)}</ul>)}
    <form onSubmit={submit}><label htmlFor={inputId}>Saved lifecycle operation ID</label><input id={inputId} value={operationId} onChange={event => { generation.current++; setOperationId(event.target.value); setSaved(null); setError(""); setBusy(false); }} required minLength={32} maxLength={32} pattern="[0-9a-f]{32}" disabled={disabled} autoCapitalize="none" autoCorrect="off" spellCheck={false} /><button type="submit" disabled={disabled || busy}>Read saved lifecycle operation</button></form>
    </>}
    {busy && <p role="status">Reading saved lifecycle records…</p>}
    {saved && <>
      <p>Saved lifecycle action: {saved.action === "start" ? "start" : "graceful shutdown"} · {saved.target.node} · VM {saved.target.vmid}</p>
      <p role="status">Saved lifecycle status: {saved.operation.status} · <code>{saved.operation.id}</code></p>
      <p>This is historical evidence. Current VM existence, power, guest readiness and SSH host identity have not been checked.</p>
      {saved.operation.status === "succeeded" && <p>Recorded task and power state were verified: {saved.operation.result?.observation.status}.</p>}
      {saved.operation.status !== "succeeded" && <p>No successful power change is established by this record. Read recovery never repeats the action; inspect uncertain work before a new request.</p>}
      <button type="button" disabled={disabled || busy} onClick={() => inspect(saved.operation.id)}>Refresh saved lifecycle operation</button>
    </>}
    {error && <p role="alert" className="error">{error}</p>}
  </section>;
}
