import { useEffect, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { setSystemRemembered, systemRemembered } from "./keychainPreference";
import { useAutoRun } from "./actionFlow";

export type ConsoleTarget = { host: string; port: number; username: string; label: string };
export type Session = { id: string; account: string; title: string };
type Inspection = { host: string; port: number; algorithm: string; fingerprint: string; status: "unknown" | "trusted" | "changed"; previousFingerprint: string | null; challengeId: string | null; expiresInSeconds: number };
type Selection = { id: string; host: string; port: number; username: string; identities: { algorithm: string; fingerprint: string }[]; expiresInSeconds: number };
type Step = "address" | "inspecting" | "trust" | "identity" | "connecting";
const loadTerminal = () => import("./SshTerminal");

/**
 * From a machine to a terminal session: verify the host key (asking only when it is
 * unknown or changed), open the application identity, then start the session. Trust
 * decisions and identity use stay explicit; an address is a hint, never host proof.
 */
export function Console({ native, target, onSession, onOpenKeychain, idPrefix = "console" }: { native: boolean; target: ConsoleTarget | null; onSession: (session: Session) => void; onOpenKeychain?: () => void; idPrefix?: string }) {
  const [host, setHost] = useState(target?.host ?? ""), [port, setPort] = useState(String(target?.port ?? 22)), [username, setUsername] = useState(target?.username ?? "debian");
  const [step, setStep] = useState<Step>("address");
  const [inspection, setInspection] = useState<Inspection | null>(null), [verified, setVerified] = useState(false);
  const [source, setSource] = useState<"application" | "system" | "agent">(systemRemembered() ? "system" : "application");
  const [passphrase, setPassphrase] = useState(""), [error, setError] = useState("");
  const [selection, setSelection] = useState<Selection | null>(null);
  const generation = useRef(0), challenge = useRef<string | null>(null), selected = useRef<string | null>(null);
  const label = target?.label ?? `${username}@${host}`;

  function release() {
    if (challenge.current) void invoke("cancel_ssh_host", { challengeId: challenge.current }).catch(() => {});
    if (selected.current) void invoke("cancel_ssh_authentication", { selectionId: selected.current }).catch(() => {});
    challenge.current = null; selected.current = null;
  }
  useEffect(() => () => { generation.current++; release(); }, []);
  // A machine with a known address starts by checking its host key (a read-only probe).
  useAutoRun(native && !!target?.host, () => void inspect(), []);

  function reset() { generation.current++; release(); setStep("address"); setInspection(null); setSelection(null); setVerified(false); setError(""); }

  async function inspect(event?: FormEvent) {
    event?.preventDefault();
    if (!native || !host) return;
    reset();
    const current = generation.current;
    setStep("inspecting");
    try {
      const result = await invoke<Inspection>("inspect_ssh_host", { host, port: Number(port) });
      if (current !== generation.current) { if (result.challengeId) void invoke("cancel_ssh_host", { challengeId: result.challengeId }).catch(() => {}); return; }
      challenge.current = result.challengeId; setInspection(result);
      setStep(result.status === "trusted" ? "identity" : "trust");
    } catch (failure) { if (current === generation.current) { setError(text(failure, "Cannot reach this host's SSH service.")); setStep("address"); } }
  }

  async function trust() {
    if (!inspection || !verified || !challenge.current) return;
    const current = generation.current, challengeId = challenge.current;
    challenge.current = null; setStep("inspecting");
    try {
      const result = inspection.status === "changed"
        ? await invoke<{ fingerprint: string }>("replace_ssh_host", { challengeId, previousFingerprint: inspection.previousFingerprint, fingerprint: inspection.fingerprint })
        : await invoke<{ fingerprint: string }>("confirm_ssh_host", { challengeId, fingerprint: inspection.fingerprint });
      if (current !== generation.current) return;
      if (result.fingerprint !== inspection.fingerprint) throw "The saved key does not match the inspected key.";
      await inspect(); // A fresh handshake must now match the saved key.
    } catch (failure) { if (current === generation.current) { setError(text(failure, "Host key was not saved. Inspect again.")); setStep("address"); } }
  }

  async function connect(event?: FormEvent) {
    event?.preventDefault();
    if (step !== "identity") return;
    const current = generation.current, secret = passphrase;
    setPassphrase(""); setError(""); setStep("connecting");
    try {
      await loadTerminal(); // Load the renderer before consuming a selection.
      const found = source === "application" ? await invoke<Selection>("list_app_ssh_identity", { host, port: Number(port), username, passphrase: secret })
        : source === "system" ? await invoke<Selection>("list_system_ssh_identity", { host, port: Number(port), username })
        : await invoke<Selection>("list_ssh_identities", { host, port: Number(port), username });
      if (current !== generation.current) { void invoke("cancel_ssh_authentication", { selectionId: found.id }).catch(() => {}); return; }
      selected.current = found.id;
      if (found.identities.length === 1) await start(found, found.identities[0].fingerprint);
      else { setSelection(found); setStep("identity"); }
    } catch (failure) {
      if (current !== generation.current) return;
      if (source === "system") { setSystemRemembered(false); setSource("application"); }
      setError(text(failure, "Cannot open the selected identity.")); setStep("identity");
    }
  }

  async function start(found: Selection, fingerprint: string) {
    const current = generation.current;
    setStep("connecting");
    try {
      const started = await invoke<{ id: string }>("start_ssh_session", { selectionId: found.id, fingerprint, cols: 80, rows: 24 });
      selected.current = null;
      if (current !== generation.current) { void invoke("close_ssh_session", { sessionId: started.id }).catch(() => {}); return; }
      onSession({ id: started.id, account: `${found.username}@${found.host} · port ${found.port}`, title: label });
      setStep("identity"); setSelection(null);
    } catch (failure) { if (current === generation.current) { setError(text(failure, "Cannot open the SSH session.")); setStep("identity"); } }
    finally { if (selected.current === found.id) { void invoke("cancel_ssh_authentication", { selectionId: found.id }).catch(() => {}); selected.current = null; } }
  }

  const busy = step === "inspecting" || step === "connecting";
  return <div className="console">
    <form className="inline-form" onSubmit={inspect}>
      <div className="field grow"><label htmlFor={`${idPrefix}-host`}>Address</label><input id={`${idPrefix}-host`} required value={host} autoCapitalize="none" autoCorrect="off" spellCheck={false} onChange={event => { reset(); setHost(event.target.value.trim()); }} placeholder="10.10.10.205" /></div>
      <div className="field port"><label htmlFor={`${idPrefix}-port`}>Port</label><input id={`${idPrefix}-port`} type="number" min={1} max={65535} value={port} onChange={event => { reset(); setPort(event.target.value); }} /></div>
      <div className="field user"><label htmlFor={`${idPrefix}-user`}>User</label><input id={`${idPrefix}-user`} required pattern="[a-zA-Z0-9_.\-]+" value={username} autoCapitalize="none" autoCorrect="off" spellCheck={false} onChange={event => { reset(); setUsername(event.target.value.trim()); }} /></div>
      <button type="submit" disabled={!native || busy || !host}>{step === "address" ? "Connect" : "Check again"}</button>
    </form>
    {!native && <p className="hint">Open the native application to connect.</p>}
    {target && !target.host && <p className="hint">No address was recorded for this machine yet. Enter the guest address.</p>}

    <ol className="console-steps" aria-label="Connection steps">
      <li className={stepClass(step, ["inspecting", "trust"], inspection?.status === "trusted")}>Host key {inspection?.status === "trusted" ? "verified" : inspection ? "needs your review" : ""}</li>
      <li className={stepClass(step, ["identity", "connecting"], false)}>Identity and session</li>
    </ol>

    {step === "inspecting" && <p role="status">Checking the host key…</p>}

    {step === "trust" && inspection && <div className="trust-card">
      {inspection.status === "changed"
        ? <p role="alert" className="error">This host presents a different key than the one you trusted. This happens after recreating a VM, or if someone is intercepting the connection.</p>
        : <p>First connection to {inspection.host}. Compare this fingerprint with the VM itself (for example <code>qm guest exec</code> on Proxmox) before trusting it.</p>}
      <dl className="facts">
        {inspection.previousFingerprint && <div><dt>Previous</dt><dd className="mono">{inspection.previousFingerprint}</dd></div>}
        <div><dt>{inspection.algorithm}</dt><dd className="mono">{inspection.fingerprint}</dd></div>
      </dl>
      <label className="consent"><input type="checkbox" checked={verified} onChange={event => setVerified(event.target.checked)} /> I verified this fingerprint through an independent trusted channel.</label>
      <div className="actions-row"><button type="button" className="primary" disabled={!verified} onClick={() => void trust()}>{inspection.status === "changed" ? "Replace trusted key" : "Trust this key"}</button><button type="button" onClick={reset}>Cancel</button></div>
    </div>}

    {(step === "identity" || step === "connecting") && !selection && <form className="identity-card" onSubmit={connect}>
      {source === "application" && <div className="field"><label htmlFor={`${idPrefix}-passphrase`}>Application key passphrase</label>
        <input id={`${idPrefix}-passphrase`} type="password" minLength={16} required autoComplete="off" value={passphrase} onChange={event => setPassphrase(event.target.value)} /></div>}
      {source === "system" && <p className="hint">Using the application key unlocked by your system key store.</p>}
      {source === "agent" && <p className="hint">Using an identity from your configured SSH agent.</p>}
      <div className="actions-row">
        <button type="submit" className="primary" disabled={busy}>{step === "connecting" ? "Opening session…" : `Open terminal as ${username}`}</button>
        <select aria-label="Identity source" value={source} onChange={event => setSource(event.target.value as typeof source)}>
          <option value="application">Application key (passphrase)</option>
          <option value="system">Application key (system key store)</option>
          <option value="agent">SSH agent</option>
        </select>
      </div>
      {onOpenKeychain && <p className="hint">Save the passphrase in your system key store from <button type="button" className="link" onClick={onOpenKeychain}>Keychain</button> to connect in one click.</p>}
    </form>}

    {selection && <div className="identity-card">
      <p>Choose the identity to authenticate with:</p>
      <div className="actions-row">{selection.identities.map(identity => <button key={identity.fingerprint} type="button" onClick={() => void start(selection, identity.fingerprint)}>{identity.algorithm} <span className="mono">{identity.fingerprint.slice(0, 20)}…</span></button>)}</div>
    </div>}
    {error && <p role="alert" className="error">{error}</p>}
  </div>;
}

function stepClass(step: Step, active: Step[], done: boolean) {
  return active.includes(step) ? "active" : done ? "done" : "";
}
function text(failure: unknown, fallback: string) { return typeof failure === "string" ? failure : fallback; }
