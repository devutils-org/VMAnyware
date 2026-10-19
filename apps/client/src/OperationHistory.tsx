import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type Summary = {
  id: string;
  kind: string;
  instanceId?: string;
  providerTaskKnown?: boolean;
  status: "queued" | "running" | "succeeded" | "failed" | "interrupted" | "uncertain";
  failure: string | null;
  createdAt: string;
  updatedAt: string;
};

export function OperationHistory({ port, native }: { port: string; native: boolean }) {
  const [operations, setOperations] = useState<Summary[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [mounted, setMounted] = useState(false);
  const [error, setError] = useState("");
  const generation = useRef(0);
  useEffect(() => {
    setMounted(true);
    return () => { generation.current += 1; };
  }, []);

  async function load() {
    const value = Number(port);
    if (!Number.isInteger(value) || value < 1 || value > 65535) {
      setError("Enter a port between 1 and 65535.");
      return;
    }
    const current = ++generation.current;
    setBusy(true);
    setError("");
    try {
      const result = await invoke<Summary[]>("list_operations", { port: value });
      if (generation.current === current) setOperations(result);
    } catch (cause) {
      if (generation.current === current) {
        setError(typeof cause === "string" ? cause : "Cannot load operation history. Please retry.");
      }
    } finally {
      if (generation.current === current) setBusy(false);
    }
  }

  return <section aria-labelledby="operations-heading" aria-busy={busy}>
    <h2 id="operations-heading">Recent operations</h2>
    <p>Load the latest 100 operations saved by this local service. Refresh to see updated states.</p>
    <button disabled={!native || !mounted || busy} onClick={load}>{busy ? "Loading operations…" : "Load operation history"}</button>
    {error && <p role="alert" className="error">{error}</p>}
    {operations !== null && <p role="status">{operations.length === 0 ? "No operations have been saved." : `${operations.length} recent operations loaded.`}</p>}
    {operations !== null && operations.length > 0 && <ul>{operations.map(operation => <li key={operation.id}>
      <strong>{operation.kind === "inventory_refresh" ? "Inventory refresh" : operation.kind === "instance_observe" ? "Instance observation" : operation.kind === "vm_start" ? "VM start" : operation.kind === "vm_shutdown" ? "VM shutdown" : "Operation"}</strong> · {operation.status}
      <p className="operation-id">{operation.id}</p>
      {operation.instanceId && <p>Instance: <span className="operation-id">{operation.instanceId}</span></p>}
      <p>Updated: <time dateTime={operation.updatedAt}>{operation.updatedAt}</time></p>
      {operation.status === "uncertain" && <p>The provider outcome is unconfirmed. Automatic replay is disabled.</p>}
      {operation.providerTaskKnown && <p>A provider task reference is retained.</p>}
      {operation.status === "interrupted" && <p>The service stopped before confirming the outcome.</p>}
      {operation.status === "failed" && <p>{operation.kind === "vm_start" || operation.kind === "vm_shutdown" ? "The action failed. Review its outcome before confirming another action." : "The read failed. Request a new read to retry."}</p>}
    </li>)}</ul>}
  </section>;
}
