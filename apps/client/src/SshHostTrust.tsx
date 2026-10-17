import { useEffect, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { SshAuthentication } from "./SshAuthentication";

type Inspection = {
  host: string;
  port: number;
  algorithm: string;
  fingerprint: string;
  status: "unknown" | "trusted" | "changed";
  challengeId: string | null;
  expiresInSeconds: number;
};
type Confirmation = { host: string; port: number; fingerprint: string };
let supportRequest: Promise<boolean> | undefined;
function cancelChallenge(challengeId: string | null) {
  if (challengeId) void invoke("cancel_ssh_host", { challengeId }).catch(() => {});
}

export function SshHostTrust({ native }: { native: boolean }) {
  const [supported, setSupported] = useState<boolean | null>(null);
  const [host, setHost] = useState("");
  const [port, setPort] = useState("22");
  const [inspection, setInspection] = useState<Inspection | null>(null);
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  const [verified, setVerified] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [deadline, setDeadline] = useState(0);
  const [now, setNow] = useState(Date.now());
  const generation = useRef(0);
  const challenge = useRef<string | null>(null);

  useEffect(() => {
    let active = true;
    if (native) {
      supportRequest ??= invoke<boolean>("ssh_host_support").catch(() => false);
      void supportRequest.then(value => { if (active) setSupported(value); });
    }
    return () => {
      active = false;
      generation.current += 1;
      cancelChallenge(challenge.current);
      challenge.current = null;
    };
  }, [native]);

  useEffect(() => {
    if (!inspection?.challengeId) return;
    const timer = window.setInterval(() => setNow(Date.now()), 500);
    return () => window.clearInterval(timer);
  }, [inspection]);

  function invalidate() {
    generation.current += 1;
    cancelChallenge(challenge.current);
    challenge.current = null;
    setInspection(null);
    setConfirmation(null);
    setVerified(false);
    setError("");
    setBusy(false);
    setDeadline(0);
  }

  async function inspect(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!native || !supported || busy) return;
    invalidate();
    // Rust applies the authoritative endpoint canonicalization and validation.
    if (!host || host.length > 253 || !/^[a-zA-Z0-9:.\-]+$/.test(host) || !/[a-zA-Z0-9]/.test(host)) {
      setError("Enter a hostname or IP address without a URL, username or path.");
      return;
    }
    if (!/^\d+$/.test(port) || Number(port) < 1 || Number(port) > 65535) {
      setError("Enter an SSH port between 1 and 65535.");
      return;
    }
    const requestGeneration = generation.current;
    setBusy(true);
    try {
      const result = await invoke<Inspection>("inspect_ssh_host", { host, port: Number(port) });
      if (requestGeneration !== generation.current) {
        cancelChallenge(result.challengeId);
        return;
      }
      challenge.current = result.challengeId;
      setInspection(result);
      const received = Date.now();
      setNow(received);
      setDeadline(received + Math.max(0, result.expiresInSeconds) * 1000);
    } catch (failure) {
      if (requestGeneration === generation.current) setError(typeof failure === "string" ? failure : "Cannot inspect this SSH host. Please retry.");
    } finally {
      if (requestGeneration === generation.current) setBusy(false);
    }
  }

  async function confirm() {
    if (!inspection || inspection.status !== "unknown" || !verified || busy || Date.now() >= deadline || !challenge.current) return;
    const challengeId = challenge.current;
    const requestGeneration = generation.current;
    challenge.current = null;
    setBusy(true);
    setVerified(false);
    // A confirmation is consumed locally before IPC; retry requires inspection.
    setInspection({ ...inspection, challengeId: null });
    try {
      const result = await invoke<Confirmation>("confirm_ssh_host", { challengeId, fingerprint: inspection.fingerprint });
      if (requestGeneration !== generation.current) return;
      if (result.host !== inspection.host || result.port !== inspection.port || result.fingerprint !== inspection.fingerprint) {
        setError("The confirmation did not match the inspected endpoint and fingerprint. Inspect again.");
        return;
      }
      setConfirmation(result);
      setInspection(null);
    } catch (failure) {
      if (requestGeneration === generation.current) setError(typeof failure === "string" ? failure : "Host trust was not confirmed. Inspect again before retrying.");
    } finally {
      cancelChallenge(challengeId);
      if (requestGeneration === generation.current) setBusy(false);
    }
  }

  const seconds = Math.max(0, Math.ceil((deadline - now) / 1000));
  return <section aria-labelledby="ssh-trust-heading" aria-busy={busy}>
    <p className="panel-kicker">04 / Secure shell</p>
    <h2 id="ssh-trust-heading" tabIndex={-1}>SSH host trust</h2>
    <p>Inspect and verify a host key before choosing an SSH identity. A separate authentication check is available after host verification.</p>
    {!native ? <p>Open the native application to inspect SSH hosts.</p> : supported === false ? <p>SSH host inspection is available only in the supported macOS prototype.</p> : supported === null && <p>Checking SSH inspection support…</p>}
    <form onSubmit={inspect}>
      <label htmlFor="ssh-host">Hostname or IP address</label>
      <div className="connection-fields">
        <input id="ssh-host" required maxLength={253} autoCapitalize="none" autoCorrect="off" spellCheck={false} value={host} onChange={event => { invalidate(); setHost(event.target.value); }} />
        <label htmlFor="ssh-port">Port <input id="ssh-port" type="number" min="1" max="65535" step="1" required value={port} onChange={event => { invalidate(); setPort(event.target.value); }} /></label>
        <button type="submit" disabled={!native || !supported || busy}>{busy ? "Working…" : "Inspect host key"}</button>
      </div>
    </form>
    {error && <p role="alert" className="error">{error}</p>}
    {inspection && <div>
      <dl className="plan-details">
        <dt>Endpoint</dt><dd>{inspection.host} · port {inspection.port}</dd>
        <dt>Algorithm</dt><dd>{inspection.algorithm}</dd>
        <dt>SHA-256 fingerprint</dt><dd className="operation-id">{inspection.fingerprint}</dd>
      </dl>
      {inspection.status === "changed" && <p role="alert" className="error">This host key differs from the trusted key. Connection is refused. Verify the change with the host administrator through an independent trusted channel.</p>}
      {inspection.status === "trusted" && <>
        <p role="status">The observed key matches the trusted key for this endpoint. No login or shell was opened.</p>
        <SshAuthentication key={`${inspection.host}:${inspection.port}`} host={inspection.host} port={inspection.port} />
      </>}
      {inspection.status === "unknown" && <>
        <p>This endpoint has no trusted key. Compare this fingerprint with the host administrator or a trusted local console through an independent channel before confirming.</p>
        <p>{seconds > 0 && inspection.challengeId ? `Confirmation expires in ${seconds} seconds.` : "This confirmation is unavailable or expired. Inspect again to request a new one."}</p>
        <label><input type="checkbox" checked={verified} disabled={busy || seconds === 0 || !inspection.challengeId} onChange={event => setVerified(event.target.checked)} /> I verified this endpoint and fingerprint through an independent trusted channel.</label>
        <div className="connection-fields">
          <button type="button" disabled={!verified || busy || seconds === 0 || !inspection.challengeId} onClick={confirm}>Trust this host key</button>
          <button type="button" disabled={busy} onClick={invalidate}>Cancel</button>
        </div>
      </>}
    </div>}
    {confirmation && <p role="status">Saved host-key trust for {confirmation.host} · port {confirmation.port}, fingerprint <span className="operation-id">{confirmation.fingerprint}</span>. Inspect again to verify a fresh handshake. No login or shell was opened.</p>}
  </section>;
}
