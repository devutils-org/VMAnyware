import { useEffect, useRef, useState } from "react";
import { LifecycleActions } from "./LifecycleActions";
import { invoke } from "@tauri-apps/api/core";
import { observeInstance, validateLifecyclePreflight, type InstanceSummary, type ObservationOperation, type LifecyclePreflight, type LifecycleCheckCode } from "./instanceObservation";

const failures: Record<string, string> = {
  provider_not_configured: "Configure Proxmox on the control plane before observing this VM.",
  provider_scope_changed: "The configured Proxmox endpoint differs from the registered instance.",
  instance_reference_invalid: "The registered target cannot be used. Check the instance reference.",
  provider_unauthorized: "Proxmox rejected the configured API token.",
  provider_forbidden: "The Proxmox token lacks permission to read this VM.",
  provider_timeout: "The Proxmox read timed out.",
  provider_unavailable: "Cannot read the VM. Check its reference, endpoint and TLS trust.",
  process_interrupted: "The service stopped before confirming this read. Observe again to request a new read.",
};

const checkLabels: Record<LifecycleCheckCode, string> = {
  instance_marker: "Instance tag matches",
  not_template: "Target is a VM",
  unlocked: "No configuration lock",
  no_pending_changes: "No pending configuration changes",
  configuration_unchanged: "Configuration remained unchanged during the check",
};

export function Instances({ port, native }: { port: string; native: boolean }) {
  const [instances, setInstances] = useState<InstanceSummary[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [mounted, setMounted] = useState(false);
  const [operation, setOperation] = useState<ObservationOperation | null>(null);
  const [preflight, setPreflight] = useState<LifecyclePreflight | null>(null);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const generation = useRef(0);
  useEffect(() => {
    setMounted(true);
    return () => { generation.current++; };
  }, []);

  function servicePort(): number | null {
    const value = Number(port);
    if (!Number.isInteger(value) || value < 1 || value > 65535) {
      setError("Enter a local service port between 1 and 65535.");
      return null;
    }
    return value;
  }
  async function load() {
    const value = servicePort(); if (value === null) return;
    const current = ++generation.current;
    setBusy(true); setError(""); setMessage(""); setOperation(null); setPreflight(null);
    try {
      const result = await invoke<InstanceSummary[]>("list_instances", { port: value });
      if (generation.current === current) setInstances(result);
    } catch (cause) {
      if (generation.current === current) setError(typeof cause === "string" ? cause : "Cannot load registered instances. Please retry.");
    } finally { if (generation.current === current) setBusy(false); }
  }
  async function observe(instance: InstanceSummary) {
    const value = servicePort(); if (value === null) return;
    const current = ++generation.current;
    const active = () => generation.current === current;
    setBusy(true); setError(""); setMessage(""); setOperation(null); setPreflight(null);
    try {
      const outcome = await observeInstance(value, instance, invoke, setOperation, active);
      if (active() && outcome === "paused") setMessage("Monitoring paused. Observe the VM again to resume the same operation.");
    } catch (cause) {
      if (active()) setError(typeof cause === "string" ? cause : "Cannot monitor this instance. Observe again to resume.");
    } finally { if (active()) setBusy(false); }
  }
  async function checkLifecycle(instance: InstanceSummary) {
    const value = servicePort(); if (value === null) return;
    const current = ++generation.current;
    const active = () => generation.current === current;
    setBusy(true); setError(""); setMessage(""); setOperation(null); setPreflight(null);
    try {
      const result = await invoke<LifecyclePreflight>("check_instance_lifecycle", { port: value, instanceId: instance.id });
      if (!active()) return;
      validateLifecyclePreflight(result, instance);
      setPreflight(result);
    } catch (cause) {
      if (active()) setError(typeof cause === "string" ? cause : "Cannot check lifecycle preconditions. Please retry.");
    } finally { if (active()) setBusy(false); }
  }
  const observation = operation?.result?.observation;
  return <section aria-labelledby="instances-heading" aria-busy={busy}>
    <h2 id="instances-heading">Registered instances</h2>
    <p>Load local definition-to-VM associations, then explicitly read their current state. Registration does not verify VM ownership or guest readiness.</p>
    <button disabled={!native || !mounted || busy} onClick={load}>{busy ? "Working…" : "Load registered instances"}</button>
    {instances !== null && <p role="status">{instances.length === 0 ? "No instances have been registered." : `${instances.length} registered instances loaded.`}</p>}
    {instances !== null && instances.length > 0 && <ul>{instances.map(instance => <li key={instance.id}>
      <strong>{instance.definitionName}</strong> · {instance.providerReference.node} / VM {instance.providerReference.vmid}
      <p>Definition revision {instance.definitionRevision} · {instance.status}</p>
      <p className="operation-id">{instance.id}</p>
      <button disabled={!native || !mounted || busy} onClick={() => observe(instance)}>Observe VM {instance.providerReference.vmid}</button>
      <button disabled={!native || !mounted || busy} onClick={() => checkLifecycle(instance)}>Check lifecycle for VM {instance.providerReference.vmid}</button>
      <LifecycleActions key={`${instance.id}.${instance.definitionHash}`} port={Number(port)} instance={instance}
        report={preflight?.instanceId === instance.id ? preflight : null} disabled={!native || !mounted || busy} invalidate={() => setPreflight(null)} />
    </li>)}</ul>}
    {operation && <p role="status">Instance observation: {operation.status} · <span className="operation-id">{operation.id}</span></p>}
    {observation && <div role="status">
      <p>VM {observation.vmid}: {observation.status} · QEMU: {observation.qmpStatus ?? "unknown"}</p>
      <p>Observed at <time dateTime={operation!.updatedAt}>{operation!.updatedAt}</time>. Guest readiness and ownership remain unverified.</p>
    </div>}
    {preflight && <div role="status">
      <h3>Lifecycle preconditions for VM {preflight.observation.vmid}</h3>
      <p>Service lifecycle execution: {preflight.executionEnabled ? "enabled; each action requires explicit confirmation" : "disabled"}.</p>
      <p>Start: {preflight.startPreconditions} · Shutdown: {preflight.shutdownPreconditions}</p>
      <ul>{preflight.checks.map(check => <li key={check.code}>{checkLabels[check.code]}: {check.status}</li>)}</ul>
      <p>A matching tag is an administrative declaration. Ownership and guest readiness remain unverified. Every action requires explicit target confirmation.</p>
    </div>}
    {operation?.failure && <p role="alert" className="error">{failures[operation.failure] ?? "The observation failed. Check the control plane before requesting a new read."}</p>}
    {message && <p role="status">{message}</p>}
    {error && <p role="alert" className="error">{error}</p>}
  </section>;
}
