import { lazy, Suspense, useEffect, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
const loadTerminal = () => import("./SshTerminal");
const SshTerminal = lazy(async () => ({ default: (await loadTerminal()).SshTerminal }));

type Identity = { algorithm: string; fingerprint: string };
type Selection = {
  id: string;
  host: string;
  port: number;
  username: string;
  identities: Identity[];
  expiresInSeconds: number;
};
type Authentication = { host: string; port: number; username: string; fingerprint: string };
function cancel(selectionId: string | null) {
  if (selectionId) void invoke("cancel_ssh_authentication", { selectionId }).catch(() => {});
}

export function SshAuthentication({ host, port }: { host: string; port: number }) {
  const [username, setUsername] = useState("");
  const [consent, setConsent] = useState(false);
  const [selection, setSelection] = useState<Selection | null>(null);
  const [fingerprint, setFingerprint] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [result, setResult] = useState<Authentication | null>(null);
  const [terminal, setTerminal] = useState<{ id: string; account: string } | null>(null);
  const [deadline, setDeadline] = useState(0);
  const [now, setNow] = useState(Date.now());
  const generation = useRef(0);
  const selectionId = useRef<string | null>(null);
  const terminalId = useRef<string | null>(null);
  function closeOwnedTerminal() {
    const sessionId = terminalId.current;
    terminalId.current = null;
    if (sessionId) void invoke("close_ssh_session", { sessionId }).catch(() => {});
  }
  useEffect(() => () => {
    generation.current += 1;
    cancel(selectionId.current);
    selectionId.current = null;
    closeOwnedTerminal();
  }, [host, port]);
  useEffect(() => {
    if (!selection) return;
    const timer = window.setInterval(() => setNow(Date.now()), 500);
    return () => window.clearInterval(timer);
  }, [selection]);
  function invalidate() {
    generation.current += 1;
    cancel(selectionId.current);
    selectionId.current = null;
    setSelection(null);
    setFingerprint("");
    setError("");
    setResult(null);
    closeOwnedTerminal();
    setTerminal(null);
    setBusy(false);
    setDeadline(0);
  }
  async function list(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!consent || busy) return;
    invalidate();
    const request = generation.current;
    setBusy(true);
    try {
      const found = await invoke<Selection>("list_ssh_identities", { host, port, username });
      if (request !== generation.current) { cancel(found.id); return; }
      if (found.host !== host || found.port !== port || found.username !== username) {
        cancel(found.id);
        setError("Identity selection did not match the requested SSH account. List again.");
        return;
      }
      selectionId.current = found.id;
      setSelection(found);
      const received = Date.now();
      setNow(received);
      setDeadline(received + found.expiresInSeconds * 1000);
    } catch (failure) {
      if (request === generation.current) setError(typeof failure === "string" ? failure : "Cannot list SSH agent identities.");
    } finally {
      if (request === generation.current) setBusy(false);
    }
  }
  async function authenticate(openTerminal = false) {
    if (!selection || !fingerprint || busy || Date.now() >= deadline || !consent) return;
    const current = selection;
    const request = generation.current;
    setSelection(null); // Each selection permits only one authentication attempt.
    setBusy(true);
    setError("");
    try {
      if (openTerminal) {
        // Load the renderer before consuming the native selection or opening SSH.
        await loadTerminal();
        if (request !== generation.current) return;
        const started = await invoke<{ id: string }>("start_ssh_session", { selectionId: current.id, fingerprint, cols: 80, rows: 24 });
        if (request !== generation.current) {
          void invoke("close_ssh_session", { sessionId: started.id }).catch(() => {});
          return;
        }
        terminalId.current = started.id;
        setTerminal({ id: started.id, account: `${current.username}@${host} · port ${port}` });
        return;
      }
      const authenticated = await invoke<Authentication>("check_ssh_authentication", { selectionId: current.id, fingerprint });
      if (request !== generation.current) return;
      if (authenticated.host !== host || authenticated.port !== port || authenticated.username !== current.username || authenticated.fingerprint !== fingerprint) {
        setError("Authentication result did not match the selected SSH account and identity.");
        return;
      }
      setResult(authenticated);
    } catch (failure) {
      if (request === generation.current) setError(typeof failure === "string" ? failure : openTerminal ? "Cannot prepare or open the SSH terminal. List identities to retry." : "SSH authentication failed. List identities to retry.");
    } finally {
      cancel(current.id);
      if (request === generation.current) { selectionId.current = null; setBusy(false); }
    }
  }
  const seconds = Math.max(0, Math.ceil((deadline - now) / 1000));
  return <div aria-busy={busy}>
    <h3>Connect with an SSH agent</h3>
    <p>Use the SSH agent configured when this application was launched. Choose one public identity for {host} · port {port}, then test authentication or open a terminal.</p>
    <form onSubmit={list}>
      <label htmlFor="ssh-username">SSH username</label>
      <div className="connection-fields">
        <input id="ssh-username" required maxLength={64} pattern="[a-zA-Z0-9_.\-]+" autoCapitalize="none" autoCorrect="off" spellCheck={false} value={username} onChange={event => { invalidate(); setUsername(event.target.value); }} />
      </div>
      <p><label><input type="checkbox" checked={consent} onChange={event => { invalidate(); setConsent(event.target.checked); }} /> Allow access to my configured SSH agent for this connection.</label></p>
      <button type="submit" disabled={!consent || busy}>List agent identities</button>
    </form>
    {busy && <p role="status">Verifying the host and contacting the SSH agent…</p>}
    {error && <p role="alert" className="error">{error}</p>}
    {selection && <>
      {selection.identities.length === 0 ? <p>The agent has no supported public identities. Load an identity in your agent, then list again.</p> : <fieldset>
        <legend>Choose the identity to authenticate with</legend>
        {selection.identities.map(identity => <p key={identity.fingerprint}><label>
          <input type="radio" name="ssh-identity" value={identity.fingerprint} checked={fingerprint === identity.fingerprint} disabled={busy || seconds === 0} onChange={() => setFingerprint(identity.fingerprint)} />
          {identity.algorithm} <span className="operation-id">{identity.fingerprint}</span>
        </label></p>)}
      </fieldset>}
      <p>{seconds > 0 ? `Selection expires in ${seconds} seconds.` : "Selection expired. List identities again."}</p>
      <div className="connection-fields">
        <button type="button" disabled={!fingerprint || busy || seconds === 0} onClick={() => authenticate()}>Check selected identity</button>
        <button type="button" disabled={!fingerprint || busy || seconds === 0} onClick={() => authenticate(true)}>Open terminal</button>
        <button type="button" onClick={invalidate}>Cancel</button>
      </div>
    </>}
    {result && <p role="status">Authenticated as {result.username} on {result.host} · port {result.port} with <span className="operation-id">{result.fingerprint}</span>. Connection closed. No terminal was opened.</p>}
    {terminal && <Suspense fallback={<p role="status">Preparing terminal…</p>}><SshTerminal key={terminal.id} id={terminal.id} account={terminal.account} /></Suspense>}
  </div>;
}
