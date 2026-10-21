import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type Environment = { id: string; definitionName: string; providerReference: { scopeId: string; node: string; vmid: number } };
type Status = { instanceId: string; providerScopeId: string; observation: { node: string; vmid: number; status: "running" | "stopped"; qmpStatus: string | null } };
const qmpStates = ["running", "stopped", "paused", "prelaunch", "shutdown", "suspended", "guest-panicked", "unknown"];
export function RemoteInstanceStatus({ connectionId, environment, disabled, invalidated }: { connectionId: number; environment: Environment; disabled: boolean; invalidated: boolean }) {
  const [observed, setObserved] = useState<{ value: Status; readAt: string } | null>(null);
  const [busy, setBusy] = useState(false), [error, setError] = useState("");
  const generation = useRef(0);
  const { scopeId, node, vmid } = environment.providerReference;
  useEffect(() => {
    generation.current++; setObserved(null); setError(""); setBusy(false);
    return () => { generation.current++; };
  }, [connectionId, environment.id, scopeId, node, vmid, invalidated]);
  async function read() {
    if (disabled || invalidated || busy) return;
    const current = ++generation.current; setBusy(true); setError(""); setObserved(null);
    try {
      const value = await invoke<Status>("read_remote_instance_status", { connectionId, target: { instanceId: environment.id, providerScopeId: scopeId, node, vmid } });
      if (generation.current !== current) return;
      const observed = value.observation;
      if (value.instanceId !== environment.id || value.providerScopeId !== scopeId || observed.node !== node || observed.vmid !== vmid || !["running", "stopped"].includes(observed.status) || (observed.qmpStatus !== null && !qmpStates.includes(observed.qmpStatus))) throw "Power observation does not match the selected instance or target.";
      setObserved({ value, readAt: new Date().toISOString() });
    } catch (failure) { if (generation.current === current) setError(typeof failure === "string" ? failure : "Cannot read this VM's power state. It remains unknown."); }
    finally { if (generation.current === current) setBusy(false); }
  }
  return <section className="remote-instance-status" aria-label={`Power observation for VM ${vmid}`} aria-busy={busy}>
    <button type="button" disabled={disabled || invalidated || busy} onClick={read}>{observed ? `Refresh power for VM ${vmid}` : `Read power for VM ${vmid}`}</button>
    {busy && <p role="status">Reading the registered target's power state…</p>}
    {!observed && !busy && <p role="status">Power state not observed.</p>}
    {observed && <>
      <p role="status">Observed power: {observed.value.observation.status} · QMP: {observed.value.observation.qmpStatus ?? "unknown"}</p>
      <p>Read completed at <time dateTime={observed.readAt}>{observed.readAt}</time> (this device's clock).</p>
      <p>This point-in-time observation may already be stale. Read again before relying on it.</p>
    </>}
    <p>Power does not verify ownership, guest readiness, network reachability or SSH host identity.</p>
    {error && <p role="alert" className="error">{error}</p>}
  </section>;
}
