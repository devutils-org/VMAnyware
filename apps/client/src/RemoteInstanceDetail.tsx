import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAutoRun } from "./actionFlow";

type Environment = { id: string; definitionName: string; status: "registered" | "deleted"; providerReference: { scopeId: string; node: string; vmid: number } };
type Detail = { id: string; definitionName: string; definitionHash: string; definitionRevision: number; definitionYaml: string; status: "registered" | "deleted"; providerReference: { provider: string; scopeId: string; node: string; vmid: number } };
export function RemoteInstanceDetail({ connectionId, environment, disabled, autoRead = false }: { connectionId: number; environment: Environment; disabled: boolean; autoRead?: boolean }) {
  const [detail, setDetail] = useState<Detail | null>(null), [busy, setBusy] = useState(false), [error, setError] = useState("");
  const generation = useRef(0), { scopeId, node, vmid } = environment.providerReference;
  useEffect(() => { generation.current++; setDetail(null); setBusy(false); setError(""); return () => { generation.current++; }; }, [connectionId, environment.id, environment.definitionName, environment.status, scopeId, node, vmid]);
  // Opening the overview is the explicit read request.
  useAutoRun(autoRead, () => void read(), [connectionId, environment.id, environment.status]);
  async function read() {
    if (disabled || busy) return;
    const current = ++generation.current; setBusy(true); setError(""); setDetail(null);
    try {
      const value = await invoke<Detail>("read_remote_instance_detail", { connectionId, target: { instanceId: environment.id, providerScopeId: scopeId, node, vmid }, definitionName: environment.definitionName });
      if (generation.current !== current) return;
      const target = value.providerReference;
      if (value.id !== environment.id || value.definitionName !== environment.definitionName || target.provider !== "proxmox" || target.scopeId !== scopeId || target.node !== node || target.vmid !== vmid || !["registered", "deleted"].includes(value.status) || !Number.isInteger(value.definitionRevision) || value.definitionRevision < 1 || value.definitionRevision > 4294967295 || !/^[0-9a-f]{64}$/.test(value.definitionHash) || !value.definitionYaml || new TextEncoder().encode(value.definitionYaml).length > 65536) throw "Saved definition does not match this instance or target. Reload the workspace.";
      setDetail(value);
    } catch (failure) { if (generation.current === current) setError(typeof failure === "string" ? failure : "Cannot read the saved definition. Retry the read."); }
    finally { if (generation.current === current) setBusy(false); }
  }
  function hide() { generation.current++; setDetail(null); setError(""); }
  return <section className="remote-instance-detail" data-instance-id={environment.id} aria-label={`Retained definition for VM ${vmid}`} aria-busy={busy}>
    {!detail && <button type="button" disabled={disabled || busy} onClick={read}>Read definition for VM {vmid}</button>}
    {busy && <p role="status">Reading the retained definition…</p>}
    {detail && <>
      <dl className="facts">
        <div><dt>Definition</dt><dd>{detail.definitionName} · revision {detail.definitionRevision}</dd></div>
        <div><dt>SHA-256</dt><dd className="mono">{detail.definitionHash.slice(0, 16)}…</dd></div>
      </dl>
      <details className="yaml">
        <summary>Show YAML</summary>
        <pre aria-label={`Saved YAML for ${detail.definitionName}`}><code>{detail.definitionYaml}</code></pre>
        <p className="hint">The saved declaration for this VM. It does not verify current VM resources, existence, guest readiness or SSH identity. Full hash <code>{detail.definitionHash}</code></p>
      </details>
      <div className="actions-row"><button type="button" className="ghost" disabled={disabled || busy} onClick={read}>Refresh</button><button type="button" className="ghost" disabled={disabled || busy} onClick={hide}>Hide</button></div>
    </>}
    {error && <p role="alert" className="error">{error}</p>}
  </section>;
}
