import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { confirmedLifecycle, monitorLifecycle, validatePendingLifecycle, type LifecycleAction, type LifecycleOperation, type PendingLifecycle } from "./lifecycleAction";
import type { InstanceSummary, LifecyclePreflight } from "./instanceObservation";

type Saved = { pending: PendingLifecycle | null; blocked: boolean };
export function LifecycleActions({ port, instance, report, disabled, invalidate }: {
  port: number; instance: InstanceSummary; report: LifecyclePreflight | null; disabled: boolean; invalidate: () => void;
}) {
  const cache = `vmanyware.instance.lifecycle.${port}.${instance.id}`;
  function read(): PendingLifecycle | null {
    const text = localStorage.getItem(cache);
    if (text === null) return null;
    const pending: PendingLifecycle = JSON.parse(text);
    validatePendingLifecycle(pending, instance);
    if (pending.port !== port) throw Error("Unrelated request");
    return pending;
  }
  const [saved, setSaved] = useState<Saved>(() => {
    try { return { pending: read(), blocked: false }; }
    catch { return { pending: null, blocked: true }; }
  });
  const [review, setReview] = useState<LifecycleAction | null>(null);
  const [accepted, setAccepted] = useState(false);
  const [busy, setBusy] = useState(false);
  const [mounted, setMounted] = useState(false);
  const [operation, setOperation] = useState<LifecycleOperation | null>(null);
  const [message, setMessage] = useState("");
  const generation = useRef(0);
  const running = useRef(false);
  useEffect(() => { setMounted(true); return () => { generation.current++; }; }, []);
  useEffect(() => { setReview(null); setAccepted(false); }, [report]);
  // Another window may retain intent. Never silently overwrite it with a fresh key.
  function persist(pending: PendingLifecycle) {
    const existing = read();
    if (existing && (existing.requestKey !== pending.requestKey
      || JSON.stringify(existing.approval) !== JSON.stringify(pending.approval))) {
      throw "Another lifecycle request is saved. Reload and review it before another action.";
    }
    localStorage.setItem(cache, JSON.stringify(pending));
  }
  async function monitor(pending: PendingLifecycle) {
    if (running.current) return;
    running.current = true;
    const current = ++generation.current;
    const active = () => generation.current === current;
    setBusy(true); setMessage(""); setOperation(null); setReview(null); setAccepted(false);
    invalidate();
    try {
      const outcome = await monitorLifecycle(pending, invoke, setOperation, active, { persist: value => {
        persist(value);
        if (active()) setSaved({ pending: value, blocked: false });
      } });
      if (active() && outcome === "paused") setMessage("Monitoring paused. Resume the saved request to continue; no new action will be created.");
    } catch {
      if (active()) setMessage("Cannot confirm the latest outcome. Keep the saved request and resume it, or review operation history. Do not submit another action.");
    } finally { running.current = false; if (active()) setBusy(false); }
  }
  async function confirm() {
    if (!review || !accepted || !report || running.current || disabled) return;
    try {
      if (read()) {
        setSaved({ pending: read(), blocked: false }); setReview(null);
        setMessage("A saved lifecycle request already exists. Resume it before another action."); return;
      }
      const pending = confirmedLifecycle(port, instance, report, review);
      // Retain intent before entering any async transport path.
      persist(pending); setSaved({ pending, blocked: false });
      await monitor(pending);
    } catch {
      setReview(null); setAccepted(false); setSaved(previous => ({ ...previous, blocked: true }));
      setMessage("Cannot safely save or confirm this request. Reload and review the instance and operation history.");
    }
  }
  function acknowledge() {
    if (!saved.pending || !operation || !["succeeded", "failed", "interrupted"].includes(operation.status)) return;
    try {
      const existing = read();
      if (!existing || existing.requestKey !== saved.pending.requestKey || existing.operationId !== operation.id) throw Error("Changed request");
      localStorage.removeItem(cache);
      setSaved({ pending: null, blocked: false }); setOperation(null); invalidate();
      setMessage("Result reviewed. Run a fresh lifecycle check before another action.");
    } catch { setMessage("The saved request changed. Reload and review operation history."); }
  }
  const unavailable = disabled || !mounted || busy || saved.blocked;
  const actionAvailable = report?.executionEnabled && report.checks.every(check => check.status === "passed");
  return <div className="lifecycle-actions" aria-busy={busy}>
    {saved.blocked && <p role="alert">Saved lifecycle state is unavailable or invalid. Reload and review operation history before another action.</p>}
    {saved.pending ? <>
      <p>Saved {saved.pending.approval.action} request for {saved.pending.approval.node} / VM {saved.pending.approval.vmid}.</p>
      {saved.pending.operationId ? <><p className="operation-id">{saved.pending.operationId}</p><p>Resuming reads the saved operation.</p></>
        : <p>The original request may not have reached the service. Resuming retries that same confirmed request and key.</p>}
      <button disabled={unavailable} onClick={() => monitor(saved.pending!)}>Resume lifecycle monitoring</button>
    </> : actionAvailable && <>
      <button disabled={unavailable || report.startPreconditions !== "passed"} onClick={() => { setReview("start"); setAccepted(false); }}>Review start VM {instance.providerReference.vmid}</button>
      <button disabled={unavailable || report.shutdownPreconditions !== "passed"} onClick={() => { setReview("shutdown"); setAccepted(false); }}>Review shutdown VM {instance.providerReference.vmid}</button>
    </>}
    {review && report && !saved.pending && <div role="group" aria-label="Confirm lifecycle action">
      <h4>Confirm {review === "start" ? "start" : "graceful shutdown"}</h4>
      <p>{instance.definitionName} · {instance.providerReference.node} / VM {instance.providerReference.vmid} · local service port {port}</p>
      <p>Instance: <span className="operation-id">{instance.id}</span></p>
      <p>Provider scope: <span className="operation-id">{instance.providerReference.scopeId}</span></p>
      <p>Definition: <span className="operation-id">{instance.definitionHash}</span></p>
      <p>Configuration: <span className="operation-id">{report.configDigest}</span></p>
      <label><input type="checkbox" checked={accepted} onChange={event => setAccepted(event.currentTarget.checked)} /> I confirm {review} for this exact target.</label>
      <button disabled={unavailable || !accepted} onClick={confirm}>Confirm {review} VM {instance.providerReference.vmid}</button>
      <button disabled={busy} onClick={() => { setReview(null); setAccepted(false); }}>Cancel lifecycle confirmation</button>
    </div>}
    {operation && <div role="status">
      <p>Lifecycle action: {operation.status} · <span className="operation-id">{operation.id}</span></p>
      {operation.status === "uncertain" && <p>The provider outcome is unconfirmed. No action will be repeated automatically. Resume monitoring to check again.</p>}
      {operation.status === "succeeded" && <p>Provider task succeeded and power is {operation.result!.observation.status}. Guest and SSH readiness are not verified.</p>}
      {["failed", "interrupted"].includes(operation.status) && <p>The action did not complete successfully. Review the target and operation history before another action.</p>}
      {["succeeded", "failed", "interrupted"].includes(operation.status) && <button disabled={unavailable} onClick={acknowledge}>Acknowledge lifecycle result</button>}
    </div>}
    {message && <p role="status">{message}</p>}
  </div>;
}
