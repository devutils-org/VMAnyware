import { useEffect, useId, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";

type Target = { id: string; definitionName: string; providerReference: { scopeId: string; node: string; vmid: number } };
type Status = "queued" | "running" | "succeeded" | "failed" | "interrupted" | "uncertain";
type History = { instanceId: string; operations: { id: string; status: Status; createdAt: string; approvalHash: string }[] };
type Recovered = { target: { instanceId: string; providerScopeId: string; definitionHash: string; node: string; vmid: number }; configDigest: string; approvalHash: string; submissionStarted: boolean; plan: { scopeHash: string; volumes: { volume: string; references: string[]; ownerVerified: boolean; effect: string }[]; snapshots: string[]; preservedMedia: string[]; preservedOwnedVolumes: string[] }; operation: { id: string; kind: string; instanceId: string; status: Status; providerTaskKnown: boolean; failure: string | null; result: null | { instanceId: string; providerScopeId: string; node: string; vmid: number; approvalHash: string; taskStatus: string; vmAbsenceVerified: boolean; volumesAbsenceVerified: boolean; definitionRetained: boolean; associationRetired: boolean } } };
const failures: Record<string, string> = { provider_task_failed: "The recorded destroy task failed; cleanup may be partial and remains uncertain.", provider_submission_unconfirmed: "The recorded submission is unknown. No DELETE is repeated by these controls.", deletion_state_unconfirmed: "Recorded VM or reviewed-volume absence remains unconfirmed.", provider_task_unconfirmed: "The recorded task outcome remains unconfirmed.", deletion_preconditions_failed: "The recorded attempt failed its deletion prerequisites.", deletion_target_changed: "The reviewed target or cleanup scope changed before the recorded attempt.", deletion_disabled: "Deletion was disabled for this recorded attempt.", approval_expired: "The recorded consent expired before dispatch." };

export function RemoteDeletionRecovery({ connectionId, environment, disabled }: { connectionId: number; environment: Target; disabled: boolean }) {
  const [history, setHistory] = useState<History | null>(null), [operationId, setOperationId] = useState("");
  const [saved, setSaved] = useState<Recovered | null>(null), [busy, setBusy] = useState(false), [error, setError] = useState("");
  const generation = useRef(0), inputId = useId();
  useEffect(() => () => { generation.current++; }, [connectionId, environment.id]);
  const report = (failure: unknown) => typeof failure === "string" ? failure : "Cannot read saved deletion operations. Please retry the read.";
  async function load() {
    if (disabled || busy) return;
    const current = ++generation.current; setBusy(true); setError(""); setSaved(null); setHistory(null);
    try {
      const result = await invoke<History>("read_remote_deletion_history", { connectionId, instanceId: environment.id });
      if (generation.current !== current) return;
      if (result.instanceId !== environment.id) throw "Saved deletion history belongs to a different instance.";
      setHistory(result);
    } catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  function validate(result: Recovered, id: string) {
    const expected = environment.providerReference, target = result.target, op = result.operation;
    if (target.instanceId !== environment.id || target.providerScopeId !== expected.scopeId || target.node !== expected.node || target.vmid !== expected.vmid || op.instanceId !== environment.id || op.id !== id || op.kind !== "vm_delete" || !/^[0-9a-f]{64}$/.test(result.approvalHash) || !/^[0-9a-f]{64}$/.test(result.plan.scopeHash) || !/^[0-9a-f]{40}$/.test(result.configDigest)) throw "Saved deletion belongs to a different target or request.";
    if (op.status === "succeeded") {
      const e = op.result;
      if (!result.submissionStarted || !op.providerTaskKnown || op.failure !== null || !e || e.instanceId !== target.instanceId || e.providerScopeId !== target.providerScopeId || e.node !== target.node || e.vmid !== target.vmid || e.approvalHash !== result.approvalHash || e.taskStatus !== "succeeded" || !e.vmAbsenceVerified || !e.volumesAbsenceVerified || !e.definitionRetained || !e.associationRetired) throw "Saved deletion success lacks matching task, scope, absence or retirement evidence.";
    }
  }
  async function inspect(id: string) {
    if (disabled || busy) return;
    if (!/^[0-9a-f]{32}$/.test(id)) { setError("Enter a saved deletion operation ID of 32 lowercase hexadecimal characters."); return; }
    const current = ++generation.current; setOperationId(id); setSaved(null); setBusy(true); setError("");
    try {
      const result = await invoke<Recovered>("recover_remote_deletion", { connectionId, instanceId: environment.id, operationId: id });
      if (generation.current === current) { validate(result, id); setSaved(result); }
    } catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  function submit(event: FormEvent) { event.preventDefault(); void inspect(operationId); }
  return <section className="deletion-recovery" aria-label={`Saved deletion operations for VM ${environment.providerReference.vmid}`} aria-busy={busy}>
    <h4>Saved deletion operations</h4>
    <p>Read requests for {environment.definitionName} · {environment.providerReference.node} · VM {environment.providerReference.vmid}, including deleted associations and after reconnecting. These controls never submit deletion or restore consent.</p>
    <button type="button" disabled={disabled || busy} onClick={load}>Find deletion operations for VM {environment.providerReference.vmid}</button>
    {history && (history.operations.length === 0 ? <p role="status">No deletion operations in this instance's recent history.</p> : <ul>{history.operations.map(operation => <li key={operation.id}>
      <button type="button" disabled={disabled || busy} onClick={() => inspect(operation.id)}>Inspect deletion operation {operation.id}</button> · {operation.status} · {operation.createdAt}
    </li>)}</ul>)}
    <form onSubmit={submit}>
      <label htmlFor={inputId}>Saved deletion operation ID</label>
      <input id={inputId} value={operationId} onChange={event => { generation.current++; setOperationId(event.target.value); setSaved(null); setError(""); setBusy(false); }} required minLength={32} maxLength={32} pattern="[0-9a-f]{32}" disabled={disabled} autoCapitalize="none" autoCorrect="off" spellCheck={false} />
      <button type="submit" disabled={disabled || busy}>Read saved deletion operation</button>
    </form>
    {busy && <p role="status">Reading saved deletion records…</p>}
    {saved && <>
      <p>Saved deletion target: {saved.target.node} · VM {saved.target.vmid}</p>
      <p role="status">Saved deletion status: {saved.operation.status} · <code>{saved.operation.id}</code></p>
      <p>This is historical evidence. Current VM/storage state has not been checked; a reused VMID can belong to a different instance.</p>
      <p>Submission recorded: {saved.submissionStarted ? "started" : "not started"} · provider task: {saved.operation.providerTaskKnown ? "known" : "not known"}</p>
      <h5>Saved reviewed cleanup scope</h5>
      {saved.plan.volumes.length ? <ul>{saved.plan.volumes.map(volume => <li key={volume.volume}><code>{volume.volume}</code> · {volume.effect === "fleecing_cleanup" ? "recorded fleecing cleanup" : "owned-volume removal"} · ownership recorded {volume.ownerVerified ? "verified" : "unconfirmed"}<ul>{volume.references.map(reference => <li key={reference}>{reference}</li>)}</ul></li>)}</ul> : <p>No candidate volumes in the saved scope.</p>}
      <p>Saved planned effects: VM configuration and snapshot metadata, VM permissions and firewall rules, IPAM and recorded fleecing cleanup. No job purge or bulk unreferenced-disk deletion. The definition and operation history are retained.</p>
      <p>Saved snapshots: {saved.plan.snapshots.join(", ") || "none"}</p>
      <p>Media excluded from deletion: {saved.plan.preservedMedia.join(", ") || "none recorded"}</p>
      <p>Other owned volumes excluded: {saved.plan.preservedOwnedVolumes.join(", ") || "none observed on inspected storages"}</p>
      {saved.operation.status === "succeeded" && <p>Saved deletion verified: the VM and reviewed volumes were absent at completion. Definition and history were retained.</p>}
      {saved.operation.status === "uncertain" && <p>Cleanup may be partial. Refresh only reads the saved operation; it never retries DELETE.</p>}
      {saved.operation.failure && <p role="alert">{failures[saved.operation.failure] || "The saved operation did not confirm deletion. Inspect its retained target."}</p>}
      <button type="button" disabled={disabled || busy} onClick={() => inspect(saved.operation.id)}>Refresh saved deletion operation</button>
    </>}
    {error && <p role="alert" className="error">{error}</p>}
  </section>;
}
