import { ProvisioningProgress } from "./ProvisioningProgress";
import { ProvisioningFailure } from "./ProvisioningFailure";
import { useEffect, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AppIdentity } from "./AppIdentity";

type Plan = { definitionName: string; definitionHash: string; planHash: string; creationEnabled: boolean; sshKeyCount: number; packages: string[]; snapshotAfterProvisioning: boolean; source: { templateVmid: number; templateVersion: string }; target: { node: string; vmid: number; storage: string; bridge: string; guestUser: string; resources: { cpu: number; memoryMiB: number; diskGiB: number } } };
type Operation = { id: string; status: "queued" | "running" | "uncertain" | "interrupted" | "succeeded" | "failed"; failure?: string | null; result: null | { guestReady: boolean; guest: { ipv4: string } } };
export function RemoteCreate({ connectionId, endpoint, disabled, onSshTarget, onEnvironmentsChanged, images }: { connectionId: number; endpoint: string; disabled: boolean; onSshTarget: (host: string, username: string) => void; onEnvironmentsChanged?: () => void; images?: string[] }) {
  const [name, setName] = useState("mobile-lab"), [image, setImage] = useState("debian-13"), [vmid, setVmid] = useState("202");
  const [cpu, setCpu] = useState("2"), [memory, setMemory] = useState("2048"), [disk, setDisk] = useState("8"), [key, setKey] = useState("");
  const [packages, setPackages] = useState(""), [snapshot, setSnapshot] = useState(false);
  const [plan, setPlan] = useState<Plan | null>(null), [requestKey, setRequestKey] = useState("");
  const [confirmed, setConfirmed] = useState(false), [submitted, setSubmitted] = useState(false), [busy, setBusy] = useState(false);
  const [operation, setOperation] = useState<Operation | null>(null), [error, setError] = useState("");
  const generation = useRef(0);
  useEffect(() => () => { generation.current++; }, [connectionId]);
  // Registration happens on submission; reload the environment list when the status changes.
  useEffect(() => { if (operation?.status) onEnvironmentsChanged?.(); }, [operation?.status, onEnvironmentsChanged]);
  const locked = disabled || busy || plan !== null || submitted;
  const report = (failure: unknown) => typeof failure === "string" ? failure : "Creation request unavailable. Retain its request key.";
  async function prepare(event: FormEvent) {
    event.preventDefault(); if (locked) return; const current = ++generation.current;
    const nextKey = crypto.randomUUID(); setBusy(true); setError(""); setConfirmed(false); setPlan(null);
    const declaredPackages = packages.split("\n").map(value => value.trim()).filter(Boolean);
    const yaml = JSON.stringify({ apiVersion: "vmanyware.dev/v1", kind: "Environment", metadata: { name }, spec: { image, resources: { cpu: Number(cpu), memoryMiB: Number(memory), diskGiB: Number(disk) }, provisioning: { cloudInit: true, sshAuthorizedKeys: key.split("\n").map(value => value.trim()).filter(Boolean), packages: declaredPackages }, lifecycle: { snapshotAfterProvisioning: snapshot } } });
    try { const result = await invoke<Plan>("prepare_remote_create", { connectionId, requestKey: nextKey, yaml, targetVmid: Number(vmid) }); if (generation.current === current) {
      if (JSON.stringify(result.packages) !== JSON.stringify(declaredPackages) || result.snapshotAfterProvisioning !== snapshot) throw "The server plan changed the declared packages or final snapshot. Review the definition and prepare again.";
      setPlan(result); setRequestKey(nextKey);
    } }
    catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  async function submit() {
    if (!plan || !plan.creationEnabled || !confirmed || busy || disabled) return;
    const current = ++generation.current; setSubmitted(true); setBusy(true); setError("");
    try { const result = await invoke<Operation>("confirm_remote_create", { connectionId, requestKey, confirmed: true }); if (generation.current === current) setOperation(result); }
    catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  async function refresh() {
    const current = ++generation.current; setBusy(true); setError("");
    try { const result = await invoke<Operation>("read_remote_create", { connectionId, requestKey }); if (generation.current === current) setOperation(result); }
    catch (failure) { if (generation.current === current) setError(report(failure)); }
    finally { if (generation.current === current) setBusy(false); }
  }
  return <section className="remote-create">
    <h3>Create an environment</h3>
    <AppIdentity disabled={locked} onUse={publicKey => setKey(current => [...new Set([...current.split("\n").map(value => value.trim()).filter(Boolean), publicKey])].join("\n"))} />
    <form className="create-definition" onSubmit={prepare}>
      <fieldset disabled={locked}>
        <div className="form-grid">
          <div className="field span-2"><label htmlFor="create-name">Environment name</label><input id="create-name" value={name} onChange={event => setName(event.target.value)} pattern="[a-z0-9]+(-[a-z0-9]+)*" maxLength={63} required /></div>
          <div className="field span-2"><label htmlFor="create-image">Configured image</label>{images && images.length > 0
            ? <select id="create-image" value={image} onChange={event => setImage(event.target.value)} required>{!images.includes(image) && <option value={image}>{image}</option>}{images.map(value => <option key={value} value={value}>{value}</option>)}</select>
            : <input id="create-image" value={image} onChange={event => setImage(event.target.value)} required />}</div>
          <div className="field"><label htmlFor="create-vmid">New VMID</label><input id="create-vmid" type="number" min={100} max={999999999} value={vmid} onChange={event => setVmid(event.target.value)} required /></div>
          <div className="field"><label htmlFor="create-cpu">CPUs</label><input id="create-cpu" type="number" min={1} max={64} value={cpu} onChange={event => setCpu(event.target.value)} required /></div>
          <div className="field"><label htmlFor="create-memory">Memory (MiB)</label><input id="create-memory" type="number" min={256} max={262144} value={memory} onChange={event => setMemory(event.target.value)} required /></div>
          <div className="field"><label htmlFor="create-disk">Disk (GiB)</label><input id="create-disk" type="number" min={4} max={4096} value={disk} onChange={event => setDisk(event.target.value)} required /></div>
          <div className="field span-4"><label htmlFor="create-public-key">SSH public keys, one per line</label><textarea id="create-public-key" value={key} onChange={event => setKey(event.target.value)} required spellCheck={false} autoCapitalize="none" autoCorrect="off" />
            <p className="hint">Public keys only. Private keys and passphrases must never be included.</p></div>
          <div className="field span-4"><label htmlFor="create-packages">Debian packages, one per line (optional)</label>
            <textarea id="create-packages" value={packages} onChange={event => setPackages(event.target.value)} maxLength={16512} spellCheck={false} autoCapitalize="none" autoCorrect="off" />
            <p className="hint">Up to 128 unique declarations, with optional =version. Packages use the guest’s configured repositories; unpinned dependencies may change.</p></div>
          <div className="field span-4"><label><input id="create-final-snapshot" type="checkbox" checked={snapshot} onChange={event => setSnapshot(event.target.checked)} /> Create a disk-only snapshot after all provisioning steps succeed.</label>
            <p className="hint">Requires server snapshot permission. It is not an independent backup or proof of application consistency.</p></div>
        </div>
        <button type="submit">Save definition and preview</button>
      </fieldset>
    </form>
    {plan && <>
      <h4>Review creation on {endpoint}</h4>
      <p>{plan.definitionName} · {plan.target.node} · VM {plan.target.vmid}</p>
      <p>Template {plan.source.templateVmid} ({plan.source.templateVersion}) · storage {plan.target.storage} · bridge {plan.target.bridge}</p>
      <p>{plan.target.resources.cpu} CPUs · {plan.target.resources.memoryMiB} MiB · {plan.target.resources.diskGiB} GiB · user {plan.target.guestUser} · {plan.sshKeyCount} public key(s)</p>
      <p>Declared packages: {plan.packages.length ? plan.packages.join(", ") : "None declared"}</p>
      <p>Final provisioning snapshot: {plan.snapshotAfterProvisioning ? "Requested · disk-only, after all steps succeed" : "Not requested"}</p>
      <details><summary>Definition and plan identifiers</summary><p>{plan.definitionHash}</p><p>{plan.planHash}</p></details>
      {!submitted && <><label><input id="create-confirm" type="checkbox" checked={confirmed} onChange={event => setConfirmed(event.target.checked)} disabled={disabled || busy} />I confirm creating this VM on this server.</label>
        <button type="button" disabled={disabled || busy || !confirmed || !plan.creationEnabled} onClick={submit}>Create confirmed VM</button>
        <button type="button" disabled={disabled || busy} onClick={() => { generation.current++; setPlan(null); setConfirmed(false); setError(""); }}>Edit definition</button></>}
      {submitted && <>
        <p>Request key: <code>{requestKey}</code></p><p>Disconnecting or closing the app does not cancel creation. Keep this key and the operation ID for recovery.</p>
        {!operation && <button type="button" disabled={disabled || busy} onClick={submit}>Retry same confirmed request</button>}
      </>}
    </>}
    {operation && <><ProvisioningProgress key={operation.id + operation.status} connectionId={connectionId} operationId={operation.id} disabled={disabled || busy} planHash={plan?.planHash} definitionHash={plan?.definitionHash} sourceInstanceId={null} node={plan?.target.node} vmid={plan?.target.vmid} /><p role="status">Creation: {operation.status} · operation <code>{operation.id}</code></p>
      {operation.status === "succeeded" && operation.result?.guestReady && <>
        <p>Guest initialized at {operation.result.guest.ipv4}. SSH access and host identity still need verification.</p>
        {plan && <button type="button" disabled={disabled || busy} onClick={() => onSshTarget(operation.result!.guest.ipv4, plan.target.guestUser)}>Prepare SSH connection</button>}
      </>}
      {["failed", "interrupted", "uncertain"].includes(operation.status) && <ProvisioningFailure code={operation.failure} />}
      {operation.status === "failed" && <p role="alert">Creation failed. The target is retained for inspection.</p>}
      <button type="button" disabled={disabled || busy} onClick={refresh}>Refresh creation status</button>
      {["succeeded", "failed"].includes(operation.status) && <button type="button" disabled={disabled || busy} onClick={() => { generation.current++; setPlan(null); setRequestKey(""); setConfirmed(false); setSubmitted(false); setOperation(null); setError(""); }}>Prepare another creation</button>}</>}
    {busy && <p role="status">Contacting the server…</p>}
    {error && <p role="alert" className="error">{error}</p>}
  </section>;
}
