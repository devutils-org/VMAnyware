// A per-device hint that the application identity's unlock secret was saved in the
// system key store. The key store stays authoritative: a failed read falls back to
// the passphrase and clears this hint.
const KEY = "vmanyware.identity.systemRemembered";

export function systemRemembered(): boolean {
  try { return localStorage.getItem(KEY) === "1"; } catch { return false; }
}

export function setSystemRemembered(value: boolean) {
  try { if (value) localStorage.setItem(KEY, "1"); else localStorage.removeItem(KEY); } catch { /* optional hint */ }
}
