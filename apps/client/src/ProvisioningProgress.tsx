import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAutoRun } from "./actionFlow";

const labels = {
  full_clone: "Clone template", configure_resources: "Configure CPU and memory",
  expand_disk: "Expand disk", configure_cloud_init: "Configure Cloud-Init and SSH public keys",
  start_vm: "Start VM", wait_for_guest_readiness: "Verify guest initialization",
  install_declared_packages: "Install declared packages", snapshot_after_provisioning: "Create final disk-only snapshot",
};
type Status = "queued" | "running" | "succeeded" | "failed" | "interrupted" | "uncertain";
type Progress = { operationId: string; instanceId: string; sourceInstanceId: string | null; definitionHash: string; planHash: string; node: string; vmid: number; status: Status; stages: { position: number; kind: keyof typeof labels; status: Status; submissionStarted: boolean }[] };
type Props = { connectionId: number; operationId: string; disabled: boolean; instanceId?: string; planHash?: string; definitionHash?: string; sourceInstanceId?: string | null; node?: string; vmid?: number };
const statuses = ["queued", "running", "succeeded", "failed", "interrupted", "uncertain"];
export function ProvisioningProgress({ connectionId, operationId, disabled, instanceId, planHash, definitionHash, sourceInstanceId, node, vmid, refreshToken }: Props & { refreshToken?: unknown }) {
  const [progress, setProgress] = useState<Progress | null>(null), [busy, setBusy] = useState(false), [error, setError] = useState("");
  const generation = useRef(0);
  useEffect(() => {
    generation.current++; setProgress(null); setError(""); setBusy(false);
    return () => { generation.current++; };
  }, [connectionId, operationId, instanceId, planHash, definitionHash, sourceInstanceId, node, vmid]);
  // Re-read in place whenever the parent re-reads its operation; keep the last steps shown.
  useAutoRun(true, () => void read(), [connectionId, operationId, refreshToken]);
  async function read() {
    if (disabled || busy) return;
    const current = ++generation.current; setBusy(true); setError("");
    try {
      const result = await invoke<Progress>("read_remote_provisioning_progress", { connectionId, operationId });
      if (generation.current !== current) return;
      if (result.operationId !== operationId || (instanceId !== undefined && result.instanceId !== instanceId) || (planHash !== undefined && result.planHash !== planHash) || (definitionHash !== undefined && result.definitionHash !== definitionHash) || (sourceInstanceId !== undefined && result.sourceInstanceId !== sourceInstanceId) || (node !== undefined && result.node !== node) || (vmid !== undefined && result.vmid !== vmid) || !statuses.includes(result.status) || result.stages.length < 5 || result.stages.length > 8 || result.stages.some((stage, i) => stage.position !== i || !Object.hasOwn(labels, stage.kind) || !statuses.includes(stage.status))) throw Error();
      setProgress(result);
    } catch { if (generation.current === current) setError("Recorded steps are unavailable. Refresh to retry the read; older servers may not support this view."); }
    finally { if (generation.current === current) setBusy(false); }
  }
  return <section className="provisioning-progress" aria-label="Recorded provisioning steps">
    {busy && !progress && <p role="status">Reading recorded provisioning steps…</p>}
    {progress && <>
      <p className="steps-head"><span className={`chip state ${progress.status}`}>{progress.status}</span> {progress.node} · VM {progress.vmid}
        <button type="button" className="ghost" disabled={disabled || busy} onClick={read}>Refresh</button></p>
      <ol className="steps">{progress.stages.map(stage => <li key={stage.position} className={`step ${stage.status}`}>
        <span className="step-icon" aria-hidden="true" />
        <span className="step-label">{labels[stage.kind]}</span>
        <span className={`chip state ${stage.status}`}>{stage.status}</span>
      </li>)}</ol>
      <p className="hint">Recorded stage evidence. It does not re-check current power, guest readiness or SSH access.</p>
    </>}
    {!progress && !busy && <button type="button" disabled={disabled} onClick={read}>Read recorded steps</button>}
    {error && <p role="alert">{error}</p>}
  </section>;
}
