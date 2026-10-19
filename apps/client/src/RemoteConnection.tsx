import { useEffect, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { RemoteCreate } from "./RemoteCreate";
import { RemoteRecovery } from "./RemoteRecovery";

type Image = { image: string; node: string; templateVmid: number; templateVersion: string; sourceDiskGiB: number; guestUser: string };
type Environment = { id: string; definitionName: string; providerReference: { node: string; vmid: number }; status: "registered" };
type Connection = { connectionId: number; endpoint: string };
export function RemoteConnection({ native }: { native: boolean }) {
  const [endpoint, setEndpoint] = useState("https://");
  const [token, setToken] = useState("");
  const [connection, setConnection] = useState<Connection | null>(null);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(false);
  const [images, setImages] = useState<Image[] | null>(null);
  const [environments, setEnvironments] = useState<Environment[] | null>(null);
  const [imageError, setImageError] = useState("");
  const [environmentError, setEnvironmentError] = useState("");
  const [error, setError] = useState("");
  const generation = useRef(0);
  useEffect(() => () => {
    generation.current++;
    if (native) void invoke("disconnect_remote").catch(() => {});
  }, [native]);
  async function connect(event: FormEvent) {
    event.preventDefault();
    const current = ++generation.current;
    const suppliedToken = token;
    setToken(""); setConnection(null); setImages(null); setEnvironments(null); setImageError(""); setEnvironmentError(""); setBusy(true); setError("");
    try {
      const result = await invoke<Connection>("connect_remote", { endpoint, token: suppliedToken });
      if (generation.current === current) setConnection(result);
    } catch (failure) {
      if (generation.current === current) setError(typeof failure === "string" ? failure : "Cannot connect to this server.");
    } finally { if (generation.current === current) setBusy(false); }
  }
  async function disconnect() {
    generation.current++; setImages(null); setEnvironments(null); setLoading(false); setBusy(true); setError("");
    try { await invoke("disconnect_remote"); setConnection(null); }
    catch { setError("Cannot clear the connection. Close the app to forget it."); }
    finally { setBusy(false); }
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
      {environments !== null && (environments.length === 0 ? <p>No environments registered on this server.</p> : <ul>{environments.map(environment => <li key={environment.id}><strong>{environment.definitionName}</strong> · {environment.providerReference.node} · VM {environment.providerReference.vmid} · live state not checked</li>)}</ul>)}
      <RemoteRecovery key={`recovery-${connection.connectionId}`} connectionId={connection.connectionId} endpoint={connection.endpoint} disabled={busy} />
      <RemoteCreate key={connection.connectionId} connectionId={connection.connectionId} endpoint={connection.endpoint} disabled={busy} />
    </>}
    {error && <p role="alert" className="error">{error}</p>}
    {!native && <p>Open the native application to connect.</p>}
  </div>;
}
