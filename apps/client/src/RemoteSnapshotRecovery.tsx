import { useEffect, useId, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAutoRun } from "./actionFlow";

type Target = { id: string; definitionName: string; providerReference: { scopeId: string; node: string; vmid: number } };
type Status = "queued" | "running" | "succeeded" | "failed" | "interrupted" | "uncertain";
type History = { instanceId: string; operations: { id: string; name: string; status: Status; createdAt: string }[] };
type Recovered = { target: { instanceId: string; providerScopeId: string; definitionHash: string; node: string; vmid: number }; request: { name: string; description: string }; submissionStarted: boolean; operation: { id: string; kind: string; instanceId: string; status: Status; providerTaskKnown: boolean; failure: string | null; result: null | { instanceId: string; providerScopeId: string; node: string; vmid: number; taskStatus: string; snapshot: { name: string; description: string; includesRam: boolean | null; providerState: string | null } } } };

export function RemoteSnapshotRecovery({ connectionId, environment, disabled, focus }: { connectionId: number; environment: Target; disabled: boolean; focus?: string }) {
  const [history, setHistory] = useState<History | null>(null), [operationId, setOperationId] = useState("");
  const [recovered, setRecovered] = useState<Recovered | null>(null), [busy, setBusy] = useState(false), [error, setError] = useState("");
  const generation = useRef(0), inputId = useId();
  useEffect(() => () => { generation.current++; }, [connectionId, environment.id]);
  const report = (failure: unknown) => typeof failure === "string" ? failure : "Cannot read saved snapshot operations. Please retry.";
  async function load() {
    if (disabled || busy) return;
    const current = ++generation.current; setBusy(true); setError(""); setRecovered(null); setHistory(null);
    try {
      const result = await invoke<History>("read_remote_snapshot_history", { connectionId, instanceId: environment.id });
      if (generation.current !== current) return;
      if (result.instanceId !== environment.id) throw "The saved snapshot history belongs to a different instance.";
      setHistory(result);
    } catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  function validate(result: Recovered, id: string) {
    const target = environment.providerReference, saved = result.target, operation = result.operation;
    if (saved.instanceId !== environment.id || saved.providerScopeId !== target.scopeId || saved.node !== target.node || saved.vmid !== target.vmid || operation.instanceId !== environment.id || operation.id !== id || operation.kind !== "vm_snapshot_create") throw "The saved snapshot operation belongs to a different target or request.";
    if (operation.status === "succeeded") {
      const evidence = operation.result;
      if (!result.submissionStarted || !operation.providerTaskKnown || operation.failure !== null || !evidence || evidence.instanceId !== saved.instanceId || evidence.providerScopeId !== saved.providerScopeId || evidence.node !== saved.node || evidence.vmid !== saved.vmid || evidence.taskStatus !== "succeeded" || evidence.snapshot.name !== result.request.name || evidence.snapshot.description !== result.request.description || evidence.snapshot.includesRam !== false || evidence.snapshot.providerState !== null) throw "The saved snapshot success is missing matching disk-only evidence.";
    }
  }
  async function inspect(id: string) {
    if (disabled || busy) return;
    if (!/^[0-9a-f]{32}$/.test(id)) { setError("Enter a saved snapshot operation ID of 32 lowercase hexadecimal characters."); return; }
    const current = ++generation.current; setOperationId(id); setRecovered(null); setBusy(true); setError("");
    try {
      const result = await invoke<Recovered>("recover_remote_snapshot", { connectionId, instanceId: environment.id, operationId: id });
      if (generation.current === current) { validate(result, id); setRecovered(result); }
    } catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  function submit(event: FormEvent) { event.preventDefault(); void inspect(operationId); }
  // Opened from the activity timeline: read that operation directly.
  useAutoRun(!!focus, () => void inspect(focus!), [focus]);
  return <section className="snapshot-recovery" aria-label={`Saved snapshot operations for VM ${environment.providerReference.vmid}`} aria-busy={busy}>
    {!focus && <>
    <h4>Saved snapshot operations</h4>
    <p>Read requests for {environment.definitionName} · {environment.providerReference.node} · VM {environment.providerReference.vmid}, including after reconnecting. These controls never submit snapshot creation.</p>
    <button type="button" disabled={disabled || busy} onClick={load}>Find snapshot operations for VM {environment.providerReference.vmid}</button>
    {history && (history.operations.length === 0 ? <p role="status">No snapshot operations in this instance's recent history.</p> : <ul>{history.operations.map(operation => <li key={operation.id}>
      <button type="button" disabled={disabled || busy} onClick={() => inspect(operation.id)}>Inspect snapshot operation {operation.id}</button> · {operation.name} · {operation.status} · {operation.createdAt}
    </li>)}</ul>)}
    <form onSubmit={submit}>
      <label htmlFor={inputId}>Saved snapshot operation ID</label>
      <input id={inputId} value={operationId} onChange={event => { generation.current++; setOperationId(event.target.value); setRecovered(null); setError(""); setBusy(false); }} required minLength={32} maxLength={32} pattern="[0-9a-f]{32}" disabled={disabled} autoCapitalize="none" autoCorrect="off" spellCheck={false} />
      <button type="submit" disabled={disabled || busy}>Read saved snapshot operation</button>
    </form>
    </>}
    {busy && <p role="status">Reading saved snapshot records…</p>}
    {recovered && <>
      <p>Saved snapshot request: <strong>{recovered.request.name}</strong> · {recovered.target.node} · VM {recovered.target.vmid}</p>
      <p>{recovered.request.description || "No description."}</p>
      <p role="status">Saved snapshot operation: {recovered.operation.status} · <code>{recovered.operation.id}</code></p>
      <p>These are saved server records. Current snapshot existence and restoration have not been checked.</p>
      {recovered.operation.status === "succeeded" && <p>The recorded task and disk-only snapshot metadata matched this request at completion.</p>}
      {recovered.operation.status === "uncertain" && <p>The saved outcome remains unconfirmed. Refresh reads the record without repeating creation.</p>}
      {["failed", "interrupted"].includes(recovered.operation.status) && <p role="alert">This saved request did not verify snapshot creation. Inspect its retained target before another action.</p>}
      {recovered.submissionStarted && !recovered.operation.providerTaskKnown && <p>A provider submission may have occurred, but no task identity was confirmed.</p>}
      <button type="button" disabled={disabled || busy} onClick={() => inspect(recovered.operation.id)}>Refresh saved snapshot operation</button>
    </>}
    {error && <p role="alert" className="error">{error}</p>}
  </section>;
}
