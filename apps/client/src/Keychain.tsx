import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AppIdentity } from "./AppIdentity";
import { setSystemRemembered } from "./keychainPreference";
import { useAutoRun } from "./actionFlow";

type TrustedHost = { host: string; port: number; algorithm: string; fingerprint: string };

/** The single place that owns the application identity and the trusted host keys. */
export function Keychain({ native, panel }: { native: boolean; panel: (title: string, body: React.ReactNode, action?: React.ReactNode) => React.ReactNode }) {
  const [hosts, setHosts] = useState<TrustedHost[] | null>(null), [error, setError] = useState(""), [pending, setPending] = useState<string | null>(null);
  async function load() {
    try { setHosts(await invoke<TrustedHost[]>("list_ssh_hosts")); setError(""); }
    catch (failure) { setError(typeof failure === "string" ? failure : "Cannot read trusted host keys."); }
  }
  useAutoRun(native, () => void load(), []);
  useEffect(() => { if (!pending) return; const timer = window.setTimeout(() => setPending(null), 5000); return () => window.clearTimeout(timer); }, [pending]);
  async function forget(host: TrustedHost) {
    const id = `${host.host}:${host.port}`;
    if (pending !== id) { setPending(id); return; } // Second click confirms.
    setPending(null);
    try { await invoke("forget_ssh_host", { host: host.host, port: host.port, fingerprint: host.fingerprint }); await load(); }
    catch (failure) { setError(typeof failure === "string" ? failure : "Cannot forget this host key."); }
  }
  return <div className="stack">
    {panel("Application SSH identity", <>
      <p className="muted">One encrypted key for this device. Its public key goes into VM definitions; consoles use it to log in. Save its passphrase in the system key store to connect in one click.</p>
      <AppIdentity open disabled={!native} onVault={action => setSystemRemembered(action !== "forget")} />
    </>)}
    {panel("Trusted host keys", <>
      {error && <p role="alert" className="error">{error}</p>}
      {hosts === null ? <p className="muted">Loading…</p> : hosts.length === 0 ? <p className="muted">No host keys trusted yet. They appear here after your first verified connection.</p> :
        <div className="table">{hosts.map(host => <div className="row" key={`${host.host}:${host.port}`}>
          <strong>{host.host}<span className="muted">:{host.port}</span></strong><span>{host.algorithm}</span><span className="mono grow">{host.fingerprint}</span>
          <button type="button" className={pending === `${host.host}:${host.port}` ? "danger solid" : "danger"} onClick={() => void forget(host)}>{pending === `${host.host}:${host.port}` ? "Confirm forget" : "Forget"}</button>
        </div>)}</div>}
      <p className="hint">Forgetting a key makes the next connection ask you to verify the host again.</p>
    </>, <button type="button" className="ghost" onClick={() => void load()}>Refresh</button>)}
  </div>;
}
