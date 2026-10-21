# 0087 — Optional Linux system key store for SSH unlock

Status: native backend/owned Secret Service/SSH, React-browser and joined Linux
WebView/production IPC/vault/terminal checks pass. Other platforms remain open.

## Decision

Offer explicit save, unlock, forget and SSH-selection actions for the existing
application identity through Linux Secret Service. Store only its unlock passphrase
in the desktop's default collection. The Ed25519 private key stays in the existing
AES-256-GCM/bcrypt encrypted application file. This is OS-managed unlock-secret
storage, not a non-exportable hardware private key or an Android Keystore claim.
The manual passphrase path remains explicit; vault failures never select it silently.

Bind the entry to fixed application/purpose attributes, the public SSH fingerprint
and a SHA-256 of the canonical identity directory. Search only that attribute set
in the default collection; never enumerate unrelated secrets or fall back to another
collection. Refuse multiple matching entries. Save first verifies the passphrase
against the bounded encrypted key, then verifies the one stored secret by readback.
Repeated explicit saves address the same entry. Unconfirmed/ambiguous writes stay
errors. Forget requires confirmation and only deletes the one matching entry;
forgetting never deletes the encrypted private key. Moving/replacing the key changes
the lookup; keep the manual passphrase for recovery.

Use pinned Linux-only `dbus-secret-service` 4.1.0 with Rust crypto, encrypted DH
sessions and zero prompt timeout. This avoids adding a generic platform store or
external runtime CLI; the library uses the existing D-Bus native dependency.
Locked/unavailable/ambiguous stores produce fixed errors and require the user to
unlock the desktop collection independently. No collection creation, automatic
unlock/prompt or automatic credential access occurs. Scope attributes are lookup
metadata, not an access-control boundary against another compromised user process.
A compromised unlocked account/process remains outside this protection.

Five fixed commands join the reviewed local-main IPC surface (82 total). Vault
reads/decryption stay in blocking native code with the existing shared identity
busy lock. Retrieved passphrases have zeroizing wrappers and never cross back to
the renderer or server. Full process/library memory erasure is not claimed. The
SSH path verifies persisted host trust before reading the vault and retains owner,
single-use identity, cancellation and fresh authentication/session checks.

React requires separate unchecked save/forget consent and clears passphrase inputs
before IPC. System unlock is explicit. Changing SSH source clears connection consent
and requires the existing identity-selection and authentication/terminal actions.
Browser errors never display raw vault failures. Support detection is compile-time
only; it neither connects to the bus nor promises an available desktop service.

## Evidence and limits

Default tests do not access a system vault. Opt-in ignored checks require an owned
runner with a private D-Bus session, temporary XDG directories and a generated test
keyring. A signed-package user prefix supplies the real daemon without system
installation. Tests cover save/reopen/forget, duplicate/corrupt/locked refusal,
directory/key isolation, unchanged encrypted file and actual SSH authentication
with host/owner checks. Separate React/Firefox simulated-IPC checks cover controls,
secret clearing, consent, fixed errors and no implicit SSH. STATE.md records logs.

No personal bus, credential, real VM or production key store is accessed. macOS,
Android, iOS and Windows system-store implementations are absent. Current APKs
predate these new commands; no new device acceptance or artifact rebuild is inferred.
The joined Linux check now also uses the real React save/unlock/forget controls,
all five production IPC commands and the system SSH source against the private
keyring. It requires interactive shell stdout, matching remote PTY resize, closed
connections, unchanged trust after a rotated-host refusal, cleared consent/input
and the exact missing-secret error after forgetting. Normal manual-key SSH remains
a separate affected regression check. Neither establishes a normal desktop daemon
configuration, hardware key protection or another OS store. Next: Android Keystore
source implementation and its own compilation/runtime gates.

## References

- [Secret Service specification](https://specifications.freedesktop.org/secret-service/latest-single/)
- [dbus-secret-service API and encryption](https://docs.rs/dbus-secret-service/4.1.0/dbus_secret_service/)
- [Prompt timeout](https://docs.rs/dbus-secret-service/4.1.0/dbus_secret_service/struct.SecretService.html#method.connect_with_max_prompt_timeout)
