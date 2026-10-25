import { ProvisioningProgress } from "./ProvisioningProgress";
import { ProvisioningFailure } from "./ProvisioningFailure";
import { useEffect, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { CheckSummary, useOperationPolling, useAutoRun } from "./actionFlow";

type Source = { id: string; definitionName: string; providerReference: { node: string; vmid: number } };
type Plan = { definitionName: string; definitionHash: string; planHash: string; providerScopeId: string; creationEnabled: boolean; sshKeyCount: number; packages: string[]; snapshotAfterProvisioning: boolean; source: { templateVmid: number; templateVersion: string }; target: { node: string; vmid: number; storage: string; bridge: string; guestUser: string; resources: { cpu: number; memoryMiB: number; diskGiB: number } } };
type Review = { sourceInstanceId: string; sourceDefinitionHash: string; sourceDefinitionRevision: number; definitionYaml: string; plan: Plan; restoresPreviousDiskContents: boolean; requiresNewInstanceIdentity: boolean; recreationEnabled: boolean; preflight: { planHash: string; creationEnabled: boolean; vmidReserved: boolean; checks: { code: string; status: string }[] } };
type Operation = { id: string; kind: string; instanceId: string; status: "queued" | "running" | "uncertain" | "interrupted" | "failed" | "succeeded"; failure?: string | null; result: null | { instanceId: string; providerScopeId: string; planHash: string; guestReady: boolean; guest: { ipv4: string; cloudInitExitCode: number; cloudInitClean: boolean } } };
const labels: Record<string, string> = { template: "Source is a template", source_disk: "Template disk matches catalog", cloud_init_drive: "Cloud-Init drive exists", storage: "Storage supports VM images", storage_capacity: "Reported storage capacity", bridge: "Network bridge is active", target_vmid: "Target VMID was observed free", template_revision: "Template image identity", execution_permissions: "Provider write permissions", guest_readiness: "Actual guest readiness" };
const required = Object.keys(labels).slice(0, 7);
function usableIpv4(value: string) { const parts = value.split("."); return parts.length === 4 && parts.every(part => /^(0|[1-9][0-9]{0,2})$/.test(part) && Number(part) <= 255) && Number(parts[0]) > 0 && Number(parts[0]) < 224 && parts[0] !== "127" && !(parts[0] === "169" && parts[1] === "254"); }
export function RemoteRecreation({ connectionId, source, disabled, onLock, onSshTarget, onEnvironmentsChanged, onDone, onSettled }: { connectionId: number; source: Source; disabled: boolean; onLock: (locked: boolean) => void; onSshTarget: (host: string, username: string) => void; onEnvironmentsChanged?: () => void; onDone?: () => void; onSettled?: (succeeded: boolean) => void }) {
  const [vmid, setVmid] = useState(""), [review, setReview] = useState<Review | null>(null), [key, setKey] = useState("");
  const [confirmed, setConfirmed] = useState(false), [submitted, setSubmitted] = useState(false), [busy, setBusy] = useState(false);
  const [operation, setOperation] = useState<Operation | null>(null), [error, setError] = useState("");
  const generation = useRef(0);
  // Set once a definitive result has released the retained native request.
  const [released, setReleased] = useState(false);
  useEffect(() => { onLock(!released && (busy || review !== null || submitted)); }, [busy, review, submitted, released, onLock]);
  useEffect(() => () => { generation.current++; onLock(false); }, [connectionId, source.id, onLock]);
  // Registration happens on submission; reload the environment list when the status changes.
  useEffect(() => { if (operation?.status) onEnvironmentsChanged?.(); }, [operation?.status, onEnvironmentsChanged]);
  const eligible = review?.recreationEnabled && review.plan.creationEnabled && review.preflight.creationEnabled && !review.preflight.vmidReserved && review.preflight.checks.length === 10 && new Set(review.preflight.checks.map(check => check.code)).size === 10 && review.preflight.checks.every(check => labels[check.code] && ["passed", "unknown"].includes(check.status)) && required.every(code => review.preflight.checks.some(check => check.code === code && check.status === "passed"));
  const terminal = operation && ["succeeded", "failed", "interrupted"].includes(operation.status);
  const report = (failure: unknown) => typeof failure === "string" ? failure : "Cannot complete this recreation request. Retain its request key if submitted.";
  async function prepare(event: FormEvent) {
    event.preventDefault(); if (disabled || busy || review || submitted) return;
    const target = Number(vmid); if (!/^[1-9][0-9]*$/.test(vmid) || target < 100 || target > 999999999) { setError("Choose a target VMID between 100 and 999999999."); return; }
    const current = ++generation.current, requestKey = crypto.randomUUID(); setKey(requestKey); setBusy(true); setError(""); setConfirmed(false); setOperation(null);
    try {
      const result = await invoke<Review>("prepare_remote_recreation", { connectionId, requestKey, sourceInstanceId: source.id, targetVmid: target });
      if (generation.current !== current) return;
      if (result.sourceInstanceId !== source.id || result.plan.definitionName !== source.definitionName || result.sourceDefinitionHash !== result.plan.definitionHash || !/^[0-9a-f]{64}$/.test(result.sourceDefinitionHash) || result.sourceDefinitionRevision < 1 || result.plan.target.vmid !== target || result.restoresPreviousDiskContents || !result.requiresNewInstanceIdentity || result.preflight.planHash !== result.plan.planHash) throw "Recreation review changed the selected source, definition or target. Review again.";
      setReview(result);
    } catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  function accept(result: Operation) {
    if (!review || !/^[0-9a-f]{32}$/.test(result.id) || result.kind !== "vm_provision" || !/^[0-9a-f]{32}$/.test(result.instanceId) || result.instanceId === source.id || (operation && (result.id !== operation.id || result.instanceId !== operation.instanceId)) || !["queued", "running", "uncertain", "interrupted", "failed", "succeeded"].includes(result.status)) throw "Recreation operation does not match a distinct new instance. Retain the original key.";
    if (result.status === "succeeded") {
      const e = result.result;
      if (!e || e.instanceId !== result.instanceId || e.providerScopeId !== review.plan.providerScopeId || e.planHash !== review.plan.planHash || !e.guestReady || !e.guest.cloudInitClean || e.guest.cloudInitExitCode !== 0 || !usableIpv4(e.guest.ipv4)) throw "Recreation success lacks matching new-instance and guest readiness evidence.";
    }
    setOperation(result);
  }
  async function submit() {
    if (!review || !eligible || disabled || busy || operation) return;
    const current = ++generation.current; setSubmitted(true); setBusy(true); setError("");
    try { const result = await invoke<Operation>("confirm_remote_recreation", { connectionId, requestKey: key, confirmed: true }); if (generation.current === current) accept(result); }
    catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  async function refresh() {
    if (!operation || disabled || busy) return;
    const current = ++generation.current; setBusy(true); setError("");
    try { const result = await invoke<Operation>("read_remote_recreation_operation", { connectionId, requestKey: key }); if (generation.current === current) accept(result); }
    catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  async function clear() {
    if (disabled || busy || (submitted && !terminal)) return;
    const current = ++generation.current; setBusy(true); setError("");
    try { await invoke("clear_remote_recreation", { connectionId, requestKey: key }); if (generation.current === current) { setReview(null); setKey(""); setConfirmed(false); setSubmitted(false); setOperation(null); onDone?.(); } }
    catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  // Readiness may report uncertain while its retained probe continues; keep reading.
  // A definitive result needs no acknowledgement: release the retained request automatically.
  async function release() {
    try { await invoke("clear_remote_recreation", { connectionId, requestKey: key }); setReleased(true); onSettled?.(operation?.status === "succeeded"); }
    catch { /* Keep the explicit Done fallback below. */ }
  }
  useAutoRun(!!terminal && !released, () => void release(), [operation?.status]);
  const pending = !!operation && !terminal;
  useOperationPolling(pending && !disabled, operation, () => void refresh());
  return <section className="remote-recreation" aria-labelledby="recreation-heading" aria-busy={busy}>
    <h3 id="recreation-heading">Recreate from a retained definition</h3>
    <p>Deleted source: {source.definitionName} · {source.providerReference.node} · VM {source.providerReference.vmid}</p>
    <p>Creates a fresh VM from the saved definition. Previous disk contents and snapshots are not restored. The deleted source and its history remain.</p>
    <form onSubmit={prepare}><label htmlFor="recreation-vmid">New target VMID</label><input id="recreation-vmid" inputMode="numeric" pattern="[1-9][0-9]*" required value={vmid} disabled={disabled || busy || review !== null || submitted} onChange={event => setVmid(event.target.value)} />{!review && !submitted && <button type="submit" disabled={disabled || busy}>Continue</button>}</form>
    {review && <>
      <p>Reviewed recreation: {review.plan.definitionName} · revision {review.sourceDefinitionRevision} · {review.plan.target.node} · VM {review.plan.target.vmid}</p>
      <p>Definition hash: <code>{review.sourceDefinitionHash}</code></p>
      <p>Template {review.plan.source.templateVmid} · {review.plan.source.templateVersion} · {review.plan.target.resources.cpu} CPUs · {review.plan.target.resources.memoryMiB} MiB · {review.plan.target.resources.diskGiB} GiB</p>
      <p>Storage {review.plan.target.storage} · bridge {review.plan.target.bridge} · guest user {review.plan.target.guestUser} · {review.plan.sshKeyCount} SSH public key(s)</p>
      <p>Declared packages: {review.plan.packages.length ? review.plan.packages.join(", ") : "None declared"}</p>
      <p>Final provisioning snapshot: {review.plan.snapshotAfterProvisioning ? "Requested · disk-only, after all steps succeed" : "Not requested"}</p>
      {review.plan.snapshotAfterProvisioning && <p>The new snapshot requires server snapshot permission. It does not recover an old snapshot or replace an independent backup.</p>}
      <details><summary>Retained definition</summary><pre>{review.definitionYaml}</pre></details>
      <p>Server recreation: {review.recreationEnabled ? "enabled" : "disabled"}. Checks are observations, not a VMID reservation or proof of guest access.</p>
      <CheckSummary checks={review.preflight.checks} labels={labels} />
      <div className="actions-row">
        {!operation && <button type="button" className="primary" disabled={!eligible || disabled || busy} onClick={submit}>{submitted ? "Retry the same recreation request" : `Create VM ${review.plan.target.vmid}`}</button>}
        {!submitted && <button type="button" disabled={disabled || busy} onClick={clear}>Cancel</button>}
        {terminal && !released && <button type="button" className="primary" disabled={disabled || busy} onClick={clear}>Done</button>}
      </div>
      <p className="hint">Request key <code>{key}</code></p>
    </>}
    {submitted && !operation && <p role="status">No verified operation ID received. Keep this request key and retry only this same confirmed recreation.</p>}
    {operation && <><ProvisioningProgress key={operation.id} refreshToken={operation} connectionId={connectionId} operationId={operation.id} instanceId={operation.instanceId} disabled={disabled} planHash={review?.plan.planHash} definitionHash={review?.plan.definitionHash} sourceInstanceId={source.id} node={review?.plan.target.node} vmid={review?.plan.target.vmid} /><p role="status">Recreation status: {operation.status} · operation <code>{operation.id}</code> · new instance <code>{operation.instanceId}</code></p>
      {operation.status === "succeeded" && operation.result && <><p>New guest initialized at {operation.result.guest.ipv4}. Previous disk contents were not recovered. SSH access and host identity still need verification.</p><button type="button" disabled={disabled || busy} onClick={() => onSshTarget(operation.result!.guest.ipv4, review!.plan.target.guestUser)}>Prepare SSH for recreated VM</button></>}
      {["failed", "interrupted", "uncertain"].includes(operation.status) && <ProvisioningFailure code={operation.failure} />}
      {["failed", "interrupted", "uncertain"].includes(operation.status) && <p role="alert">Recreation did not confirm readiness. The new target is retained for inspection; refreshing does not submit another creation.</p>}
      {pending && <p className="hint">Updating automatically. You can close this panel; the server keeps working and the action stays in History.</p>}</>}
    {busy && !operation && <p role="status">{review ? "Submitting…" : "Checking the plan and prerequisites…"}</p>}
    {error && <p role="alert" className="error">{error}</p>}
    <p>Closing or disconnecting does not cancel server work. Consent is not restored automatically.</p>
  </section>;
}
