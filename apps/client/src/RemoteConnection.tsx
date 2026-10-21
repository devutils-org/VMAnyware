import { useEffect, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { RemoteLifecycleRecovery } from "./RemoteLifecycleRecovery";
import { RemoteLifecycle } from "./RemoteLifecycle";
import { RemoteDeletionRecovery } from "./RemoteDeletionRecovery";
import { RemoteRecreationRecovery } from "./RemoteRecreationRecovery";
import { RemoteRecreation } from "./RemoteRecreation";
import { RemoteDeletion } from "./RemoteDeletion";
import { RemoteCreate } from "./RemoteCreate";
import { RemoteSnapshotRestore } from "./RemoteSnapshotRestore";
import { RemoteSnapshotCreate } from "./RemoteSnapshotCreate";
import { RemoteInstanceDetail } from "./RemoteInstanceDetail";
import { RemoteInstanceStatus } from "./RemoteInstanceStatus";
import { RemoteSnapshots } from "./RemoteSnapshots";
import { RemoteRecovery } from "./RemoteRecovery";
import { SshHostTrust } from "./SshHostTrust";

type Image = { image: string; node: string; templateVmid: number; templateVersion: string; sourceDiskGiB: number; guestUser: string };
type Environment = { id: string; definitionName: string; providerReference: { scopeId: string; node: string; vmid: number }; status: "registered" | "deleted" };
type Connection = { connectionId: number; endpoint: string };
export function RemoteConnection({ native }: { native: boolean }) {
  const [endpoint, setEndpoint] = useState("https://");
  const [token, setToken] = useState("");
  const [caPem, setCaPem] = useState("");
  const [connection, setConnection] = useState<Connection | null>(null);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(false);
  const [images, setImages] = useState<Image[] | null>(null);
  const [environments, setEnvironments] = useState<Environment[] | null>(null);
  const [imageError, setImageError] = useState("");
  const [environmentError, setEnvironmentError] = useState("");
  const [error, setError] = useState("");
  const [recreationTarget, setRecreationTarget] = useState<Environment | null>(null);
  const [deletionTarget, setDeletionTarget] = useState<Environment | null>(null);
  const [snapshotTarget, setSnapshotTarget] = useState<Environment | null>(null);
  const [restoreTarget, setRestoreTarget] = useState<{environment: Environment; snapshotName: string} | null>(null);
  const [lifecycleTarget, setLifecycleTarget] = useState<{environment: Environment; action: "start" | "shutdown"} | null>(null);
  const [mutationLocked, setMutationLocked] = useState(false);
  const generation = useRef(0), sshGeneration = useRef(0);
  const [sshTarget, setSshTarget] = useState<{ host: string; username: string; generation: number; manualInstance?: Environment } | null>(null);
  function selectSshTarget(host: string, username: string) {
    setSshTarget({ host, username, generation: ++sshGeneration.current });
  }
  function selectManualSshTarget(environment: Environment) {
    if (busy || loading || mutationLocked || environment.status !== "registered") return;
    setRecreationTarget(null); setDeletionTarget(null); setSnapshotTarget(null); setRestoreTarget(null); setLifecycleTarget(null);
    setSshTarget({ host: "", username: "", generation: ++sshGeneration.current, manualInstance: environment });
  }
  useEffect(() => {
    const selected = sshTarget?.manualInstance;
    if (!selected || !environments) return;
    const current = environments.find(environment => environment.id === selected.id);
    if (!current || current.status !== "registered" || current.definitionName !== selected.definitionName || current.providerReference.scopeId !== selected.providerReference.scopeId || current.providerReference.node !== selected.providerReference.node || current.providerReference.vmid !== selected.providerReference.vmid) setSshTarget(null);
  }, [environments, sshTarget]);
  useEffect(() => {
    if (!sshTarget) return;
    const heading = document.getElementById("ssh-trust-heading");
    heading?.scrollIntoView({ block: "start" }); heading?.focus({ preventScroll: true });
  }, [sshTarget]);
  useEffect(() => () => {
    generation.current++;
    if (native) void invoke("disconnect_remote").catch(() => {});
  }, [native]);
  async function connect(event: FormEvent) {
    event.preventDefault();
    const current = ++generation.current;
    setSshTarget(null); setLifecycleTarget(null); setRecreationTarget(null); setDeletionTarget(null); setSnapshotTarget(null); setRestoreTarget(null); setMutationLocked(false);
    const suppliedToken = token;
    setToken(""); setConnection(null); setImages(null); setEnvironments(null); setImageError(""); setEnvironmentError(""); setBusy(true); setError("");
    try {
      const result = await invoke<Connection>("connect_remote", { endpoint, token: suppliedToken, caPem: caPem.trim() || null });
      if (generation.current === current) setConnection(result);
    } catch (failure) {
      if (generation.current === current) setError(typeof failure === "string" ? failure : "Cannot connect to this server.");
    } finally { if (generation.current === current) setBusy(false); }
  }
  async function disconnect() {
    generation.current++; setSshTarget(null); setLifecycleTarget(null); setRecreationTarget(null); setDeletionTarget(null); setSnapshotTarget(null); setRestoreTarget(null); setMutationLocked(false); setCaPem(""); setImages(null); setEnvironments(null); setLoading(false); setBusy(true); setError("");
    try { await invoke("disconnect_remote"); setConnection(null); }
    catch { setError("Cannot clear the connection. Close the app to forget it."); }
    finally { setBusy(false); }
  }
  function selectLifecycle(environment: Environment, action: "start" | "shutdown") {
    setRecreationTarget(null); setDeletionTarget(null); setSnapshotTarget(null); setRestoreTarget(null);
    setLifecycleTarget({environment, action});
  }
  async function loadWorkspace() {
    if (!connection) return;
    const current = ++generation.current;
    setLoading(true); setImageError(""); setEnvironmentError("");
    const results = await Promise.allSettled([
      invoke<Image[]>("read_remote_images", { connectionId: connection.connectionId }),
      invoke<Environment[]>("read_remote_environments", { connectionId: connection.connectionId }),
    ]);
    if (generation.current !== current) return;
    if (results[0].status === "fulfilled") setImages(results[0].value);
    else { setImages(null); setImageError(typeof results[0].reason === "string" ? results[0].reason : "Cannot load templates."); }
    if (results[1].status === "fulfilled") setEnvironments(results[1].value);
    else { setEnvironments(null); setEnvironmentError(typeof results[1].reason === "string" ? results[1].reason : "Cannot load environments."); }
    setLoading(false);
  }
  return <div className="remote-connection">
    <p>Connect securely to your server. Proxmox credentials stay on the server.</p>
    <form onSubmit={connect}>
      <label htmlFor="remote-endpoint">Server HTTPS address</label>
      <input id="remote-endpoint" type="url" inputMode="url" autoCapitalize="none" autoCorrect="off" required disabled={busy || connection !== null} value={endpoint} onChange={event => setEndpoint(event.target.value)} placeholder="https://vm.example.com" />
      <label htmlFor="remote-token">Control-plane access token</label>
      <input id="remote-token" type="password" autoComplete="off" autoCapitalize="none" autoCorrect="off" spellCheck={false} minLength={32} maxLength={256} required disabled={busy || connection !== null} value={token} onChange={event => setToken(event.target.value)} />
      <details>
        <summary>Private certificate authority (optional)</summary>
        <label htmlFor="remote-ca">Trusted public CA certificate (PEM)</label>
        <textarea id="remote-ca" value={caPem} onChange={event => setCaPem(event.target.value)} maxLength={16384} disabled={busy || connection !== null} autoCapitalize="none" autoCorrect="off" spellCheck={false} />
        <p>Use a public CA certificate obtained from your server administrator through a trusted channel. Never paste a private key. This connection will trust only that CA. Device certificates are unchanged.</p>
      </details>
      <p>The token is not saved. Enter it again after closing or disconnecting.</p>
      <div className="connection-fields">
        <button type="submit" disabled={!native || busy || connection !== null}>{busy ? "Checking…" : "Connect securely"}</button>
        {connection && <button type="button" disabled={busy} onClick={disconnect}>Disconnect</button>}
      </div>
    </form>
    {connection && <>
      <p role="status">Connected to {connection.endpoint}</p>
      <button type="button" disabled={busy || loading} onClick={loadWorkspace}>{loading ? "Loading remote workspace…" : "Load remote workspace"}</button>
      <h3>Configured templates</h3>
      {imageError && <p role="alert" className="error">{imageError}</p>}
      {images !== null && (images.length === 0 ? <p>No templates configured on this server.</p> : <ul>{images.map(image => <li key={image.image}><strong>{image.image}</strong> · {image.node} · template {image.templateVmid} · {image.sourceDiskGiB} GiB · {image.templateVersion}</li>)}</ul>)}
      <h3>Registered environments</h3>
      {environmentError && <p role="alert" className="error">{environmentError}</p>}
      {environments !== null && (environments.length === 0 ? <p>No environments registered on this server.</p> : <ul>{environments.map(environment => <li key={environment.id}><strong>{environment.definitionName}</strong> · {environment.providerReference.node} · VM {environment.providerReference.vmid} · {environment.status === "deleted" ? "deleted · definition and history retained" : "registered"}{environment.status === "deleted" && <button type="button" disabled={busy || loading || mutationLocked} onClick={() => {setLifecycleTarget(null);setDeletionTarget(null); setSnapshotTarget(null); setRestoreTarget(null); setRecreationTarget(environment);}}>Prepare recreation from VM {environment.providerReference.vmid}</button>}{environment.status === "registered" && <><button type="button" disabled={busy || loading || mutationLocked} onClick={() => selectManualSshTarget(environment)}>Prepare SSH for VM {environment.providerReference.vmid}</button><RemoteInstanceStatus key={`power.${connection.connectionId}.${environment.id}.${environment.providerReference.scopeId}.${environment.providerReference.node}.${environment.providerReference.vmid}`} connectionId={connection.connectionId} environment={environment} disabled={busy || loading} invalidated={mutationLocked} /><button type="button" disabled={busy || loading || mutationLocked} onClick={() => selectLifecycle(environment, "start")}>Prepare start for VM {environment.providerReference.vmid}</button><button type="button" disabled={busy || loading || mutationLocked} onClick={() => selectLifecycle(environment, "shutdown")}>Prepare shutdown for VM {environment.providerReference.vmid}</button><button type="button" disabled={busy || loading || mutationLocked} onClick={() => {setLifecycleTarget(null);setRecreationTarget(null); setSnapshotTarget(null); setRestoreTarget(null); setDeletionTarget(environment);}}>Prepare deletion for VM {environment.providerReference.vmid}</button><button type="button" disabled={busy || loading || mutationLocked} onClick={() => {setLifecycleTarget(null);setRecreationTarget(null); setDeletionTarget(null); setRestoreTarget(null); setSnapshotTarget(environment);}}>Prepare snapshot for VM {environment.providerReference.vmid}</button><RemoteSnapshots key={`${connection.connectionId}.${environment.id}.${environment.providerReference.scopeId}.${environment.providerReference.node}.${environment.providerReference.vmid}`} connectionId={connection.connectionId} environment={environment} disabled={busy || loading} restoreDisabled={mutationLocked} onRestore={snapshotName => {setLifecycleTarget(null);setRecreationTarget(null); setDeletionTarget(null); setSnapshotTarget(null); setRestoreTarget({environment,snapshotName});}} /></>}{environment.status === "deleted" && <RemoteRecreationRecovery key={`recreation-history.${connection.connectionId}.${environment.id}`} connectionId={connection.connectionId} source={environment} disabled={busy || loading} />}<RemoteInstanceDetail key={`detail.${connection.connectionId}.${environment.id}.${environment.providerReference.scopeId}.${environment.providerReference.node}.${environment.providerReference.vmid}.${environment.status}`} connectionId={connection.connectionId} environment={environment} disabled={busy || loading} /><RemoteLifecycleRecovery key={`lifecycle-history.${connection.connectionId}.${environment.id}`} connectionId={connection.connectionId} environment={environment} disabled={busy || loading} /><RemoteDeletionRecovery key={`deletion-history.${connection.connectionId}.${environment.id}.${environment.providerReference.scopeId}.${environment.providerReference.node}.${environment.providerReference.vmid}`} connectionId={connection.connectionId} environment={environment} disabled={busy || loading} /></li>)}</ul>)}
      {recreationTarget && <RemoteRecreation key={`${connection.connectionId}.${recreationTarget.id}`} connectionId={connection.connectionId} source={recreationTarget} disabled={busy || loading} onLock={setMutationLocked} onSshTarget={selectSshTarget} />}
      {lifecycleTarget && <RemoteLifecycle key={`${connection.connectionId}.${lifecycleTarget.environment.id}.${lifecycleTarget.action}`} connectionId={connection.connectionId} environment={lifecycleTarget.environment} action={lifecycleTarget.action} disabled={busy || loading} onLock={setMutationLocked} />}
      {deletionTarget && <RemoteDeletion key={`${connection.connectionId}.${deletionTarget.id}`} connectionId={connection.connectionId} environment={deletionTarget} disabled={busy || loading} onLock={setMutationLocked} onDeleted={instanceId => setEnvironments(current => current?.map(environment => environment.id === instanceId ? {...environment, status: "deleted"} : environment) ?? null)} />}
      {restoreTarget && <RemoteSnapshotRestore key={`${connection.connectionId}.${restoreTarget.environment.id}.${restoreTarget.snapshotName}`} connectionId={connection.connectionId} environment={restoreTarget.environment} snapshotName={restoreTarget.snapshotName} disabled={busy || loading} onLock={setMutationLocked} />}
      {snapshotTarget && <RemoteSnapshotCreate key={`${connection.connectionId}.${snapshotTarget.id}`} connectionId={connection.connectionId} environment={snapshotTarget} disabled={busy || loading} onLock={setMutationLocked} />}
      <RemoteRecovery key={`recovery-${connection.connectionId}`} connectionId={connection.connectionId} endpoint={connection.endpoint} disabled={busy || mutationLocked} onSshTarget={selectSshTarget} />
      <RemoteCreate key={connection.connectionId} connectionId={connection.connectionId} endpoint={connection.endpoint} disabled={busy || mutationLocked} onSshTarget={selectSshTarget} />
      {sshTarget && <div className="remote-ssh-target">
        {sshTarget.manualInstance ? <>
          <p>SSH preparation for {sshTarget.manualInstance.definitionName} · {sshTarget.manualInstance.providerReference.node} · VM {sshTarget.manualInstance.providerReference.vmid}.</p>
          <p>Enter the guest's reachable hostname or IP and port yourself. No address or readiness was discovered from power or the saved definition, and this association has not verified that endpoint. Independently verify its host key before choosing an account or unlocking an identity.</p>
        </> : <p>SSH target: {sshTarget.username ? `${sshTarget.username}@` : ""}{sshTarget.host} · port 22. Inspect and independently verify its host key before unlocking an identity.</p>}
        <SshHostTrust key={sshTarget.generation} native={native} initialHost={sshTarget.host} initialUsername={sshTarget.username} />
      </div>}
    </>}
    {error && <p role="alert" className="error">{error}</p>}
    {!native && <p>Open the native application to connect.</p>}
  </div>;
}
