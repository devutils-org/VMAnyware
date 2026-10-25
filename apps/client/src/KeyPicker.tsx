import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { setSystemRemembered, systemRemembered } from "./keychainPreference";

type PublicIdentity = { publicKey: string; fingerprint: string };

/**
 * Adds the existing application public key to a definition. Creating or managing the
 * identity lives in Keychain only, so a definition can never receive a new key by accident.
 */
export function KeyPicker({ disabled, added, onUse, onOpenKeychain }: { disabled: boolean; added: string | null; onUse: (identity: PublicIdentity) => void; onOpenKeychain: () => void }) {
  const [useSystem, setUseSystem] = useState(systemRemembered());
  const [passphrase, setPassphrase] = useState(""), [busy, setBusy] = useState(false), [error, setError] = useState("");
  async function add() {
    if (disabled || busy) return;
    const secret = passphrase;
    setPassphrase(""); setBusy(true); setError("");
    try {
      onUse(useSystem ? await invoke<PublicIdentity>("unlock_system_app_identity") : await invoke<PublicIdentity>("unlock_app_identity", { passphrase: secret }));
    } catch (failure) {
      if (useSystem) { setSystemRemembered(false); setUseSystem(false); setError("The system key store did not open the identity. Enter its passphrase."); }
      else setError(typeof failure === "string" ? failure : "Cannot open the application identity.");
    } finally { setBusy(false); }
  }
  if (added) return <div className="key-picker added"><span className="chip registered">Application key</span><span className="mono">{added}</span></div>;
  // Rendered inside the creation form: no nested form, Enter adds the key here.
  return <div className="key-picker inline-form">
    {!useSystem && <div className="field grow"><label htmlFor="key-passphrase">Application key passphrase</label>
      <input id="key-passphrase" type="password" autoComplete="off" onKeyDown={event => { if (event.key === "Enter") { event.preventDefault(); void add(); } }} disabled={disabled || busy} value={passphrase} onChange={event => setPassphrase(event.target.value)} /></div>}
    <button type="button" disabled={disabled || busy || (!useSystem && passphrase.length < 16)} onClick={() => void add()}>{busy ? "Opening…" : "Add application key"}</button>
    <p className="hint full">Uses your existing application identity. Create or manage it in <button type="button" className="link" onClick={onOpenKeychain}>Keychain</button>.</p>
    {error && <p role="alert" className="error full">{error}</p>}
  </div>;
}
