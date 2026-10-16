import { useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";

type Plan = {
  definitionName: string;
  definitionHash: string;
  planHash: string;
  source: { image: string; node: string; templateVmid: number; templateVersion: string; sourceDiskGiB: number; diskDevice: string };
  target: { node: string; vmid: number; name: string; storage: string; bridge: string; guestUser: string; ipv4: string;
    resources: { cpu: number; memoryMiB: number; diskGiB: number } };
  sshKeyCount: number;
  packages: string[];
  snapshotAfterProvisioning: boolean;
  liveVerified: boolean;
  vmidReserved: boolean;
  creationEnabled: boolean;
};

type Preflight = {
  planHash: string;
  checks: { code: string; status: "passed" | "failed" | "unknown"; message: string }[];
  creationEnabled: false;
  vmidReserved: false;
};

export function ProvisioningPreview({ port, yaml, native }: { port: string; yaml: string; native: boolean }) {
  const [vmid, setVmid] = useState("");
  const [busy, setBusy] = useState<"preview" | "preflight" | null>(null);
  const [checked, setChecked] = useState<{ port: string; yaml: string; vmid: string; result: Preflight } | null>(null);
  const [error, setError] = useState<{ port: string; yaml: string; vmid: string; message: string } | null>(null);
  const [response, setResponse] = useState<{ port: string; yaml: string; vmid: string; plan: Plan } | null>(null);
  const plan = response?.port === port && response.yaml === yaml && response.vmid === vmid ? response.plan : null;
  const preflight = checked?.port === port && checked.yaml === yaml && checked.vmid === vmid && checked.result.planHash === plan?.planHash ? checked.result : null;
  const errorMessage = error?.port === port && error.yaml === yaml && error.vmid === vmid ? error.message : "";
  function reportError(message: string) { setError({ port, yaml, vmid, message }); }

  async function preview(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!Number.isInteger(Number(port)) || Number(port) < 1 || Number(port) > 65535) {
      reportError("Enter a local service port between 1 and 65535.");
      return;
    }
    if (!Number.isInteger(Number(vmid)) || Number(vmid) < 100 || Number(vmid) > 999999999) {
      reportError("Choose a target VMID between 100 and 999999999.");
      return;
    }
    setBusy("preview");
    setChecked(null);
    setError(null);
    setResponse(null);
    try {
      const result = await invoke<Plan>("preview_provisioning", { port: Number(port), yaml, targetVmid: Number(vmid) });
      setResponse({ port, yaml, vmid, plan: result });
    } catch (error) {
      reportError(typeof error === "string" ? error : "Cannot preview provisioning. Please retry.");
    } finally { setBusy(null); }
  }

  async function checkAgainstProxmox() {
    if (!plan) return;
    setBusy("preflight");
    setError(null);
    setChecked(null);
    try {
      const result = await invoke<Preflight>("preflight_provisioning", { port: Number(port), yaml, targetVmid: Number(vmid) });
      if (result.planHash !== plan.planHash) {
        reportError("The template mapping changed. Preview provisioning again before checking it.");
        return;
      }
      setChecked({ port, yaml, vmid, result });
    } catch (error) {
      reportError(typeof error === "string" ? error : "Cannot check provisioning. Please retry.");
    } finally { setBusy(null); }
  }

  return <section aria-labelledby="plan-heading" aria-busy={busy !== null}>
    <h2 id="plan-heading">Provisioning preview</h2>
    <p>Preview the current YAML using the template mapping configured on the control plane.</p>
    <form onSubmit={preview}>
      <label htmlFor="target-vmid">Requested VMID</label>
      <div className="connection-fields">
        <input id="target-vmid" type="number" min="100" max="999999999" step="1" required value={vmid}
          disabled={busy !== null} onChange={event => { setVmid(event.target.value); setError(null); }} />
        <button disabled={!native || busy !== null} type="submit">{busy === "preview" ? "Preparing preview…" : "Preview provisioning"}</button>
      </div>
    </form>
    {errorMessage && <p role="alert" className="error">{errorMessage}</p>}
    {plan && <div role="status">
      <h3>{plan.definitionName}</h3>
      <dl className="plan-details">
        <dt>Source image</dt><dd>{plan.source.image} · {plan.source.templateVersion}</dd>
        <dt>Template</dt><dd>{plan.source.node} · VMID {plan.source.templateVmid}</dd>
        <dt>Target</dt><dd>{plan.target.node} · VMID {plan.target.vmid} · {plan.target.name}</dd>
        <dt>Resources</dt><dd>{plan.target.resources.cpu} CPUs · {plan.target.resources.memoryMiB} MiB RAM · {plan.target.resources.diskGiB} GiB disk</dd>
        <dt>Disk</dt><dd>{plan.source.diskDevice} · {plan.source.sourceDiskGiB} → {plan.target.resources.diskGiB} GiB · {plan.target.storage}</dd>
        <dt>Network</dt><dd>{plan.target.bridge} · IPv4 {plan.target.ipv4.toUpperCase()}</dd>
        <dt>Guest access</dt><dd>{plan.target.guestUser} · {plan.sshKeyCount} public SSH key(s)</dd>
        <dt>Packages</dt><dd>{plan.packages.length ? plan.packages.join(", ") : "None declared"}</dd>
        <dt>Snapshot after provisioning</dt><dd>{plan.snapshotAfterProvisioning ? "Requested" : "Not requested"}</dd>
      </dl>
      <button type="button" disabled={!native || busy !== null} onClick={checkAgainstProxmox}>
        {busy === "preflight" ? "Checking Proxmox…" : "Check against Proxmox"}
      </button>
      {preflight && <div aria-labelledby="preflight-heading">
        <h3 id="preflight-heading">Read-only Proxmox checks</h3>
        <ul>{preflight.checks.map(check => <li key={check.code}>
          <strong>{check.status === "passed" ? "Passed" : check.status === "failed" ? "Failed" : "Unknown"}:</strong> {check.message}
        </li>)}</ul>
        <p>These results describe the host at the time of the check. Unknown checks need further verification. The VMID is not reserved and creation remains unavailable.</p>
      </div>}
      {!preflight && <p>Offline preview only. The template, available capacity, network and VMID have not been checked against Proxmox. The VMID is not reserved.</p>}
      <p>Creation is not available yet. Package installation and snapshots are declarations in this plan.</p>
      <details><summary>Configuration references</summary>
        <p className="operation-id">Definition: {plan.definitionHash}</p>
        <p className="operation-id">Plan: {plan.planHash}</p>
      </details>
    </div>}
  </section>;
}
