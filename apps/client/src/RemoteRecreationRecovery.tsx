import { ProvisioningProgress } from "./ProvisioningProgress";
import { ProvisioningFailure } from "./ProvisioningFailure";
import { useEffect, useId, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";

type Source = { id: string; definitionName: string; providerReference: { node: string; vmid: number } };
type Status = "queued" | "running" | "uncertain" | "interrupted" | "failed" | "succeeded";
type History = { sourceInstanceId: string; operations: { id: string; instanceId: string; status: Status; createdAt: string }[] };
type Saved = { sourceInstanceId: string; restoresPreviousDiskContents: boolean; requiresNewInstanceIdentity: boolean; plan: { definitionName: string; definitionHash: string; providerScopeId: string; planHash: string; source: { templateVmid: number; templateVersion: string }; target: { node: string; vmid: number; storage: string; bridge: string; guestUser: string; resources: { cpu: number; memoryMiB: number; diskGiB: number } } }; operation: { id: string; kind: string; instanceId: string; status: Status; failure?: string | null; result: null | { instanceId: string; providerScopeId: string; planHash: string; guestReady: boolean; guest: { ipv4: string; cloudInitClean: boolean; cloudInitExitCode: number } } } };
const statuses = ["queued", "running", "uncertain", "interrupted", "failed", "succeeded"];
export function RemoteRecreationRecovery({ connectionId, source, disabled }: { connectionId: number; source: Source; disabled: boolean }) {
  const [history, setHistory] = useState<History | null>(null), [operationId, setOperationId] = useState("");
  const [saved, setSaved] = useState<Saved | null>(null), [busy, setBusy] = useState(false), [error, setError] = useState("");
  const generation = useRef(0), inputId = useId();
  useEffect(() => () => { generation.current++; }, [connectionId, source.id]);
  const report = (failure: unknown) => typeof failure === "string" ? failure : "Cannot read saved recreations. Please retry the read.";
  async function load() {
    if (disabled || busy) return;
    const current = ++generation.current; setBusy(true); setError(""); setHistory(null); setSaved(null);
    try {
      const result = await invoke<History>("read_remote_recreation_history", { connectionId, sourceInstanceId: source.id });
      if (generation.current !== current) return;
      if (result.sourceInstanceId !== source.id || result.operations.length > 100 || new Set(result.operations.map(operation => operation.id)).size !== result.operations.length || result.operations.some(operation => !/^[0-9a-f]{32}$/.test(operation.id) || !/^[0-9a-f]{32}$/.test(operation.instanceId) || operation.instanceId === source.id || !statuses.includes(operation.status))) throw "Saved recreation history does not match this source or new instances.";
      setHistory(result);
    } catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  async function inspect(id: string) {
    if (disabled || busy) return;
    if (!/^[0-9a-f]{32}$/.test(id)) { setError("Enter a saved operation ID of 32 lowercase hexadecimal characters."); return; }
    const current = ++generation.current; setOperationId(id); setSaved(null); setBusy(true); setError("");
    try {
      const result = await invoke<Saved>("recover_remote_recreation", { connectionId, sourceInstanceId: source.id, operationId: id });
      if (generation.current !== current) return;
      const plan = result.plan, op = result.operation;
      if (result.sourceInstanceId !== source.id || result.restoresPreviousDiskContents || !result.requiresNewInstanceIdentity || plan.definitionName !== source.definitionName || !/^[0-9a-f]{64}$/.test(plan.definitionHash) || !/^[0-9a-f]{64}$/.test(plan.planHash) || !/^[0-9a-f]{64}$/.test(plan.providerScopeId) || op.id !== id || op.kind !== "vm_provision" || !/^[0-9a-f]{32}$/.test(op.instanceId) || op.instanceId === source.id || !statuses.includes(op.status)) throw "Saved recreation belongs to a different source, request or instance.";
      if (op.status === "succeeded") {
        const e = op.result;
        if (!e || e.instanceId !== op.instanceId || e.planHash !== plan.planHash || e.providerScopeId !== plan.providerScopeId || !e.guestReady || !e.guest.cloudInitClean || e.guest.cloudInitExitCode !== 0) throw "Saved recreation success lacks matching new-instance and guest evidence.";
      }
      setSaved(result);
    } catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  function submit(event: FormEvent) { event.preventDefault(); void inspect(operationId); }
  return <section className="recreation-recovery" aria-label={`Saved recreations from VM ${source.providerReference.vmid}`} aria-busy={busy}>
    <h4>Saved recreations</h4>
    <p>Read recorded recreations from {source.definitionName} · {source.providerReference.node} · source VM {source.providerReference.vmid}. These controls never submit a new creation or restore consent.</p>
    <button type="button" disabled={disabled || busy} onClick={load}>Find recreations from VM {source.providerReference.vmid}</button>
    {history && (history.operations.length === 0 ? <p role="status">No recreations in this source's recent history.</p> : <ul>{history.operations.map(operation => <li key={operation.id}><button type="button" disabled={disabled || busy} onClick={() => inspect(operation.id)}>Inspect recreation operation {operation.id}</button> · {operation.status} · {operation.createdAt} · new instance <code>{operation.instanceId}</code></li>)}</ul>)}
    <form onSubmit={submit}><label htmlFor={inputId}>Saved recreation operation ID</label><input id={inputId} value={operationId} onChange={event => { generation.current++; setOperationId(event.target.value); setSaved(null); setError(""); setBusy(false); }} required minLength={32} maxLength={32} pattern="[0-9a-f]{32}" disabled={disabled} autoCapitalize="none" autoCorrect="off" spellCheck={false} /><button type="submit" disabled={disabled || busy}>Read saved recreation operation</button></form>
    {busy && <p role="status">Reading saved recreation records…</p>}
    {saved && <>
      <ProvisioningProgress key={saved.operation.id + saved.operation.status} connectionId={connectionId} operationId={saved.operation.id} instanceId={saved.operation.instanceId} disabled={disabled || busy} planHash={saved.plan.planHash} definitionHash={saved.plan.definitionHash} sourceInstanceId={source.id} node={saved.plan.target.node} vmid={saved.plan.target.vmid} />
      <p>Saved recreation target: {saved.plan.target.node} · VM {saved.plan.target.vmid} · new instance <code>{saved.operation.instanceId}</code></p>
      <p role="status">Saved recreation status: {saved.operation.status} · <code>{saved.operation.id}</code></p>
      <p>This is historical evidence. Current VM existence, guest readiness, address and SSH host identity have not been checked.</p>
      <p>Retained definition: {saved.plan.definitionName} · <code>{saved.plan.definitionHash}</code></p>
      <p>Recorded template {saved.plan.source.templateVmid} · {saved.plan.source.templateVersion} · {saved.plan.target.resources.cpu} CPUs · {saved.plan.target.resources.memoryMiB} MiB · {saved.plan.target.resources.diskGiB} GiB · storage {saved.plan.target.storage} · bridge {saved.plan.target.bridge}</p>
      <p>Recreation provisions a fresh VM; previous disk contents and snapshots are not restored. The source and its history remain.</p>
      {saved.operation.status === "succeeded" && saved.operation.result && <p>Recorded guest initialization succeeded at {saved.operation.result.guest.ipv4}. Inspect the current instance before preparing SSH.</p>}
      {["failed", "interrupted", "uncertain"].includes(saved.operation.status) && <ProvisioningFailure code={saved.operation.failure} />}
      {["failed", "interrupted", "uncertain"].includes(saved.operation.status) && <p>Recorded readiness is not confirmed. Refresh reads only the saved operation and never repeats creation.</p>}
      <button type="button" disabled={disabled || busy} onClick={() => inspect(saved.operation.id)}>Refresh saved recreation operation</button>
    </>}
    {error && <p role="alert" className="error">{error}</p>}
  </section>;
}
