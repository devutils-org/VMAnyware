import { useEffect, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";

type PublicIdentity = { publicKey: string; fingerprint: string };
export function AppIdentity({ disabled, onUse }: { disabled: boolean; onUse: (publicKey: string) => void }) {
  const [supported, setSupported] = useState(false);
  const [passphrase, setPassphrase] = useState(""), [confirmation, setConfirmation] = useState("");
  const [identity, setIdentity] = useState<PublicIdentity | null>(null);
  const [busy, setBusy] = useState(false), [error, setError] = useState("");
  const generation = useRef(0);
  useEffect(() => {
    let active = true;
    void invoke<boolean>("identity_support").then(value => { if (active) setSupported(value); }).catch(() => {});
    return () => { active = false; generation.current++; };
  }, []);
  async function read(create: boolean) {
    if (disabled || busy) return;
    if (create && passphrase !== confirmation) { setError("The passphrases must match."); return; }
    const current = ++generation.current;
    const secret = passphrase;
    setPassphrase(""); setConfirmation(""); setIdentity(null); setBusy(true); setError("");
    try {
      const result = await invoke<PublicIdentity>(create ? "create_app_identity" : "unlock_app_identity", { passphrase: secret });
      if (generation.current === current) setIdentity(result);
    } catch (failure) { if (generation.current === current) setError(typeof failure === "string" ? failure : "SSH identity unavailable."); }
    finally { if (generation.current === current) setBusy(false); }
  }
  function unlock(event: FormEvent) { event.preventDefault(); void read(false); }
  if (!supported) return null;
  return <details className="app-identity">
    <summary>Use an application SSH key</summary>
    <p>Create once, then unlock the same key after reopening the app. Keep the passphrase: it cannot be recovered. The private key is stored encrypted on this device; only its public key goes into the VM definition.</p>
    <p>Device migration and key backup are not available yet. Keep another public key authorized if you need independent access.</p>
    <form onSubmit={unlock}>
      <fieldset disabled={disabled || busy}>
        <label htmlFor="identity-passphrase">SSH key passphrase (at least 16 characters)</label>
        <input id="identity-passphrase" type="password" minLength={16} maxLength={1024} autoComplete="off" autoCapitalize="none" autoCorrect="off" spellCheck={false} value={passphrase} onChange={event => setPassphrase(event.target.value)} required />
        <button type="submit">Unlock existing SSH identity</button>
        <label htmlFor="identity-confirmation">Repeat passphrase to create a new identity</label>
        <input id="identity-confirmation" type="password" maxLength={1024} autoComplete="off" autoCapitalize="none" autoCorrect="off" spellCheck={false} value={confirmation} onChange={event => setConfirmation(event.target.value)} />
        <button type="button" disabled={passphrase.length < 16 || passphrase !== confirmation} onClick={() => read(true)}>Create encrypted SSH identity</button>
      </fieldset>
    </form>
    {busy && <p role="status">Opening the encrypted SSH identity…</p>}
    {identity && <>
      <p>SSH identity: <code>{identity.fingerprint}</code></p>
      <label htmlFor="identity-public-key">Application public key</label>
      <textarea id="identity-public-key" readOnly value={identity.publicKey} spellCheck={false} />
      <button type="button" disabled={disabled || busy} onClick={() => onUse(identity.publicKey)}>Add this public key to the definition</button>
    </>}
    {error && <p role="alert" className="error">{error}</p>}
  </details>;
}
