# 0090 — macOS Keychain unlock-secret backend

Status: source, isolated Apple Silicon typecheck and full macOS CI build/link
plus the scope-only test pass. No Keychain/runtime/GUI acceptance is inferred.

## Decision

Extend the five existing explicit system-identity actions to macOS without adding
IPC commands. Keep the SSH private key in the existing AES-GCM/bcrypt encrypted app
file. Store only the optional unlock passphrase as a generic password in one
explicit default macOS keychain. Resolve that handle once per action, then address
only fixed service `dev.vmanyware.client.ssh-unlock.v1` and a purpose/directory/public
fingerprint SHA-256 account. Do not use the unrestricted keychain search list,
enumerate accounts, import keys, change the default/search list, create a keychain
or unlock one programmatically. No access occurs during support detection/startup.

Use pinned, macOS-only security-framework 3.7.0 with default features disabled. This
version is already in the production lock; no new dependency packages are resolved.
Its explicit SecKeychain interface keeps every operation on one keychain and avoids
raw application FFI. These legacy macOS APIs do not establish data-protection,
ThisDeviceOnly, iCloud, Touch ID or hardware-backed SSH protection. macOS can prompt
for access after the user's action. No process-global interaction policy is changed.
Prompt/cancellation/locked-store and stable signing identity require actual macOS
acceptance; a native worker can remain pending on an OS prompt even after an SSH
caller deadline. No automatic retry or fallback is introduced.

Verify the manual passphrase and encrypted-file policy before opening the store
for save. Update an existing exact item, add only after errSecItemNotFound, and
refuse other lookup errors. Do not use the library's set helper whose fallback
attempts add after every lookup error. Confirm the saved secret by readback. Forget
requires confirmation and deletes only the exact item; its legacy delete wrapper
has no status, so success additionally requires a subsequent item-not-found result.
Missing forget is idempotent; other failures stay fixed errors. The SSH key and
existing sessions are not deleted/revoked. Moving/replacing the file or changing
the default keychain can make the saved unlock secret unavailable; manual recovery
remains explicit. Orphan cleanup is not implemented through broad enumeration.

Reads are bounded before copying; Rust copies use zeroizing buffers. Security
Framework-owned allocations are released by the library and full erasure is not
claimed. No raw native status/message, passphrase or keychain item is serialized.
Shared identity busy, host-verification-first, owner, TTL and session gates stay in
place. System-store labels mention all three implemented platforms while retaining
separate initial consent and input clearing.

## Verification layers

A real-source probe compiles the actual IdentityStore and macOS backend against
Apple Silicon Rust std and production-locked dependency versions, without linking
Apple frameworks, building Tauri or executing Keychain calls. It lives under ignored
.lab, not a new workspace crate. An ordinary host scope test uses only newly
generated encrypted files. Frontend/browser and strict Linux client Clippy checks
cover affected callers; they do not prove macOS behavior.

A separate path-filtered macos-15 CI workflow builds the actual client with bundled
assets/frameworks and runs only the scope test (no Keychain access). It records its
actual OS/architecture/Xcode/Rust environment. No app launch, credential operation,
AVD or Proxmox operation is included. CI evidence is recorded only after observing
the job's result. Run [40028754564](https://github.com/devutils-org/VMAnyware/actions/runs/40028754564)
succeeded for source commit 26eb8a2, including the full client build and scope test.
Full runtime acceptance must use an explicitly approved disposable
keychain/store target, not a personal login-keychain test. Windows/iOS and real
infrastructure/deployment gates remain open.

## References

- [security-framework password APIs](https://docs.rs/security-framework/3.7.0/security_framework/passwords/)
- [Explicit macOS Keychain interface](https://docs.rs/security-framework/3.7.0/security_framework/os/macos/keychain/struct.SecKeychain.html)
- [GitHub-hosted runner labels](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
