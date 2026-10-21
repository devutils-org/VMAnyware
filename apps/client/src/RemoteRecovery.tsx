import { ProvisioningProgress } from "./ProvisioningProgress";
import { ProvisioningFailure } from "./ProvisioningFailure";
import { useEffect, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";

type Summary = { id: string; status: string; instanceId: string | null; createdAt: string };
type Recovered = { operation: { id: string; status: string; failure?: string | null; result: null | { guestReady: boolean; guest: { ipv4: string } } }; environment: { definitionName: string; providerReference: { node: string; vmid: number } } };
export function RemoteRecovery({ connectionId, endpoint, disabled, onSshTarget }: { connectionId: number; endpoint: string; disabled: boolean; onSshTarget: (host: string, username: string) => void }) {
  const [recent, setRecent] = useState<Summary[] | null>(null), [operationId, setOperationId] = useState("");
  const [result, setResult] = useState<Recovered | null>(null), [busy, setBusy] = useState(false), [error, setError] = useState("");
  const generation = useRef(0);
  useEffect(() => () => { generation.current++; }, [connectionId]);
  function report(failure: unknown) { return typeof failure === "string" ? failure : "Cannot read this server's creation history."; }
  async function loadRecent() {
    const current = ++generation.current; setBusy(true); setError(""); setResult(null);
    try { const operations = await invoke<Summary[]>("read_remote_creations", { connectionId }); if (generation.current === current) setRecent(operations); }
    catch (failure) { if (generation.current === current) { setRecent(null); setError(report(failure)); } }
    finally { if (generation.current === current) setBusy(false); }
  }
  async function inspect(id: string) {
    const current = ++generation.current; setOperationId(id); setResult(null); setBusy(true); setError("");
    try { const detail = await invoke<Recovered>("recover_remote_creation", { connectionId, operationId: id }); if (generation.current === current) setResult(detail); }
    catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  function submit(event: FormEvent) { event.preventDefault(); void inspect(operationId); }
  return <section className="remote-recovery">
    <h3>Find an existing creation</h3>
    <p>Read operations retained on {endpoint}, including after reconnecting or restarting the app. These controls never submit creation again.</p>
    <button type="button" disabled={disabled || busy} onClick={loadRecent}>Find recent creations</button>
    {recent !== null && (recent.length === 0 ? <p>No creations in this server's recent operation history.</p> : <ul>{recent.map(operation => <li key={operation.id}>
      <button type="button" disabled={disabled || busy} onClick={() => inspect(operation.id)}>Inspect {operation.id}</button> · {operation.status} · {operation.createdAt}
    </li>)}</ul>)}
    <form onSubmit={submit}>
      <label htmlFor="recover-operation-id">Saved operation ID</label>
      <input id="recover-operation-id" value={operationId} onChange={event => { generation.current++; setOperationId(event.target.value); setResult(null); setError(""); }} minLength={32} maxLength={32} pattern="[0-9a-f]{32}" required disabled={disabled || busy} autoCapitalize="none" autoCorrect="off" spellCheck={false} />
      <button type="submit" disabled={disabled || busy}>Read saved creation</button>
    </form>
    {busy && <p role="status">Reading this server's retained operation…</p>}
    {result && <>
      <ProvisioningProgress key={result.operation.id + result.operation.status} connectionId={connectionId} operationId={result.operation.id} disabled={disabled || busy} node={result.environment.providerReference.node} vmid={result.environment.providerReference.vmid} />
      <p>Retained target: {result.environment.definitionName} · {result.environment.providerReference.node} · VM {result.environment.providerReference.vmid}</p>
      <p role="status">Recovered creation: {result.operation.status} · operation <code>{result.operation.id}</code></p>
      {result.operation.status === "succeeded" && result.operation.result?.guestReady && <>
        <p>Recorded guest initialization at {result.operation.result.guest.ipv4}. This is saved evidence; current power and SSH access still need verification.</p>
        <p>Enter the guest account after inspecting its SSH host key. Recovery does not retain an SSH username.</p>
        <button type="button" disabled={disabled || busy} onClick={() => onSshTarget(result.operation.result!.guest.ipv4, "")}>Prepare SSH connection</button>
      </>}
      {["failed", "interrupted", "uncertain"].includes(result.operation.status) && <ProvisioningFailure code={result.operation.failure} />}
      {result.operation.status === "failed" && <p role="alert">Creation failed. Inspect the retained target before requesting another action.</p>}
      <button type="button" disabled={disabled || busy} onClick={() => inspect(result.operation.id)}>Refresh recovered creation</button>
    </>}
    {error && <p role="alert" className="error">{error}</p>}
  </section>;
}
