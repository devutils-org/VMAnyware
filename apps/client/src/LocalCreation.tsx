import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Plan, Preflight } from "./ProvisioningPreview";
import { confirmedCreation, creationAvailable, monitorCreation, validatePendingCreation, type PendingCreation, type CreationOperation } from "./creationRequest";
import { ProvisioningFailure } from "./ProvisioningFailure";

const cache = "vmanyware.local-creation.v1";
export function LocalCreation({ port, plan, preflight, native, onLockedChange, onAcknowledged }: {
  port: string; plan: Plan | null; preflight: Preflight | null; native: boolean; onLockedChange: (locked: boolean) => void; onAcknowledged: () => void;
}) {
  const [pending, setPending] = useState<PendingCreation | null>(null);
  const [mounted, setMounted] = useState(false), [blocked, setBlocked] = useState(false);
  const [confirmed, setConfirmed] = useState(false), [busy, setBusy] = useState(false);
  const [operation, setOperation] = useState<CreationOperation | null>(null), [message, setMessage] = useState("");
  const generation = useRef(0), sending = useRef(false);
  useEffect(() => {
    try { const raw = localStorage.getItem(cache); if (raw) { const value = JSON.parse(raw) as PendingCreation; validatePendingCreation(value); setPending(value); } }
    catch { setBlocked(true); }
    setMounted(true);
    return () => { generation.current++; };
  }, []);
  useEffect(() => { onLockedChange(!mounted || !!pending || blocked); }, [mounted, pending, blocked, onLockedChange]);
  useEffect(() => { setConfirmed(false); }, [port, plan, preflight]);
  function persist(value: PendingCreation) {
    localStorage.setItem(cache, JSON.stringify(value));
    setPending(value);
  }
  async function monitor(value: PendingCreation) {
    if (sending.current || blocked || !native) return;
    const current = ++generation.current;
    sending.current = true; setBusy(true); setMessage("");
    try {
      const result = await monitorCreation(value, invoke, setOperation, () => generation.current === current, { persist });
      if (generation.current === current && result === "paused") setMessage("Monitoring paused. Resume reads the same operation.");
    } catch (error) { if (generation.current === current) setMessage(typeof error === "string" ? error : "Creation response unavailable. Resume the retained request before another action."); }
    finally { sending.current = false; if (generation.current === current) setBusy(false); }
  }
  async function create() {
    if (!plan || !preflight || !confirmed || !mounted || pending || blocked || sending.current || !native) return;
    try {
      if (localStorage.getItem(cache)) { setBlocked(true); return; }
      const value = confirmedCreation(Number(port), plan, preflight, confirmed);
      persist(value); // Persist before any transport; storage failure prevents submission.
      setConfirmed(false);
      await monitor(value);
    } catch (error) { setMessage(typeof error === "string" ? error : "Cannot retain this creation request; nothing new was submitted."); }
  }
  function acknowledge() {
    if (!pending || !operation || !["succeeded", "failed", "interrupted"].includes(operation.status)) return;
    try {
      const saved = JSON.parse(localStorage.getItem(cache) ?? "null") as PendingCreation;
      validatePendingCreation(saved);
      if (saved.requestKey !== pending.requestKey || saved.operationId !== operation.id) throw Error("Changed request");
      localStorage.removeItem(cache); setPending(null); setOperation(null); setConfirmed(false);
      setMessage("Result acknowledged. Run a fresh preview and preflight before another creation.");
      onAcknowledged();
    } catch { setBlocked(true); }
  }
  const ready = creationAvailable(plan, preflight);
  return <div className="local-creation" aria-busy={busy}>
    {blocked && <p role="alert">Saved creation state is unavailable or invalid. Review operation history before another request.</p>}
    {pending ? <>
      <h3>Retained creation request</h3>
      <p>{pending.plan.definitionName} · local service {pending.port} · {pending.plan.target.node} / VM {pending.plan.target.vmid}</p>
      <p>Request key: <span className="operation-id">{pending.requestKey}</span></p>
      {pending.operationId ? <p>Operation: <span className="operation-id">{pending.operationId}</span>. Resume reads this operation.</p>
        : <p>The original response is unconfirmed. Resume retries only the same confirmed plan and request key.</p>}
      <button type="button" disabled={!native || busy || blocked} onClick={() => monitor(pending)}>Resume creation monitoring</button>
    </> : ready && plan && <>
      <h3>Confirm creation</h3>
      <p>Create {plan.definitionName} from template {plan.source.templateVmid} on {plan.target.node}, VM {plan.target.vmid}, storage {plan.target.storage}, network {plan.target.bridge}.</p>
      <p>{plan.target.resources.cpu} CPUs · {plan.target.resources.memoryMiB} MiB RAM · {plan.target.resources.diskGiB} GiB disk · {plan.sshKeyCount} public SSH key(s).</p>
      <p>Packages: {plan.packages.length ? plan.packages.join(", ") : "None"}. Final snapshot: {plan.snapshotAfterProvisioning ? "Requested" : "Not requested"}.</p>
      <p>This clones, configures and starts a real VM. Readiness is verified after boot.</p>
      <label><input type="checkbox" checked={confirmed} disabled={busy || blocked || !native} onChange={event => setConfirmed(event.target.checked)} /> I approve creation and start of this exact target with the reviewed definition.</label>
      <button type="button" disabled={!native || !mounted || !confirmed || busy || blocked} onClick={create}>Create VM {plan.target.vmid}</button>
    </>}
    {operation && <div role="status">
      <p>Creation: {operation.status} · <span className="operation-id">{operation.id}</span></p>
      <ProvisioningFailure code={operation.failure} />
      {operation.status === "succeeded" && <p>Guest initialization verified at {operation.result!.guest.ipv4}. SSH host identity and reachability require a separate check.</p>}
      {operation.status === "uncertain" && <p>The provider outcome is unconfirmed. No new request will be created automatically.</p>}
      {["succeeded", "failed", "interrupted"].includes(operation.status) && <button type="button" disabled={busy || blocked} onClick={acknowledge}>Acknowledge creation result</button>}
    </div>}
    {message && <p role="status">{message}</p>}
  </div>;
}
