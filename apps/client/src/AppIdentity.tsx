import { useEffect, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";

type PublicIdentity = { publicKey: string; fingerprint: string };
export function AppIdentity({ disabled, onUse }: { disabled: boolean; onUse: (publicKey: string) => void }) {
  const [supported, setSupported] = useState(false), [systemSupported, setSystemSupported] = useState(false);
  const [rememberConsent, setRememberConsent] = useState(false), [forgetConsent, setForgetConsent] = useState(false), [notice, setNotice] = useState("");
  const [passphrase, setPassphrase] = useState(""), [confirmation, setConfirmation] = useState("");
  const [identity, setIdentity] = useState<PublicIdentity | null>(null);
  const [busy, setBusy] = useState(false), [error, setError] = useState("");
  const generation = useRef(0);
  useEffect(() => {
    let active = true;
    void invoke<boolean>("identity_support").then(value => { if (active) setSupported(value); }).catch(() => {});
    void invoke<boolean>("system_identity_support").then(value => { if (active) setSystemSupported(value); }).catch(() => {});
    return () => { active = false; generation.current++; };
  }, []);
  async function read(create: boolean) {
    if (disabled || busy) return;
    if (create && passphrase !== confirmation) { setError("The passphrases must match."); return; }
    const current = ++generation.current;
    const secret = passphrase;
    setPassphrase(""); setConfirmation(""); setIdentity(null); setBusy(true); setError(""); setNotice(""); setRememberConsent(false); setForgetConsent(false);
    try {
      const result = await invoke<PublicIdentity>(create ? "create_app_identity" : "unlock_app_identity", { passphrase: secret });
      if (generation.current === current) setIdentity(result);
    } catch (failure) { if (generation.current === current) setError(typeof failure === "string" ? failure : "SSH identity unavailable."); }
    finally { if (generation.current === current) setBusy(false); }
  }
  async function vault(action: "remember" | "unlock" | "forget") {
    if (disabled || busy || (action === "remember" && (!rememberConsent || passphrase.length < 16)) || (action === "forget" && !forgetConsent)) return;
    const current = ++generation.current, secret = passphrase;
    setPassphrase(""); setConfirmation(""); setRememberConsent(false); setForgetConsent(false); setIdentity(null); setNotice(""); setError(""); setBusy(true);
    try {
      if (action === "forget") {
        await invoke("forget_app_identity", { confirmed: true });
        if (generation.current === current) setNotice("Saved unlock secret forgotten. The encrypted SSH key is unchanged; keep its passphrase.");
      } else {
        const result = action === "remember" ? await invoke<PublicIdentity>("remember_app_identity", { passphrase: secret, confirmed: true }) : await invoke<PublicIdentity>("unlock_system_app_identity");
        if (generation.current === current) { setIdentity(result); setNotice(action === "remember" ? "Unlock secret saved in the system key store." : "Identity unlocked using the system key store."); }
      }
    } catch { if (generation.current === current) setError("System key store unavailable or unable to open this identity. Retry the read, use the passphrase, or explicitly forget the saved secret before saving again."); }
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
    {systemSupported && <fieldset disabled={disabled || busy} className="identity-vault">
      <legend>System key store</legend>
      <p>Optionally protect this key's unlock passphrase with the operating system key store (Linux Secret Service or Android Keystore). The SSH key remains encrypted in application storage. Access depends on this device and key store; keep the passphrase for recovery.</p>
      <label><input id="remember-identity-consent" type="checkbox" checked={rememberConsent} onChange={event => setRememberConsent(event.target.checked)} /> Save the entered passphrase for this application SSH identity.</label>
      <button type="button" disabled={!rememberConsent || passphrase.length < 16} onClick={() => vault("remember")}>Remember SSH unlock secret</button>
      <button type="button" onClick={() => vault("unlock")}>Unlock using system key store</button>
      <label><input id="forget-identity-consent" type="checkbox" checked={forgetConsent} onChange={event => setForgetConsent(event.target.checked)} /> Forget only this identity's saved unlock secret.</label>
      <button type="button" disabled={!forgetConsent} onClick={() => vault("forget")}>Forget saved SSH unlock secret</button>
    </fieldset>}
    {notice && <p role="status">{notice}</p>}
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
