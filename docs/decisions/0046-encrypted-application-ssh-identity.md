# 0046 — Encrypted application SSH identity for the first mobile journey

Date: 2026-10-18
Status: Linux source/build/fixture validation; mobile runtime unproven.

The existing macOS agent cannot supply a key on a phone. Add one concrete app-owned
Ed25519 identity, explicitly created by the user and encrypted with their passphrase.
Its public key has an immediate use: add it to the confirmed remote-create form.
Do not import arbitrary key files or transmit private material to the control plane.
Do not introduce a generic identity/provider framework.

Use the already resolved ssh-key 0.7.0-rc.11 implementation: OpenSSH envelope,
AES-256-GCM authenticated encryption, random 16-byte bcrypt-pbkdf salt and 16 rounds.
Require a passphrase of 16 or more characters, at most 1024 bytes. An exact cipher,
algorithm and bounded KDF profile are checked before decryption. This is encrypted
application storage, not hardware-backed/non-exportable OS key storage or biometrics.
The passphrase's strength still determines resistance to offline guessing.

Store only in the fixed native app-local ssh-identity-v1 directory. Require current
UID ownership and private directory/file permissions; reject symlinks and excessive
input. Publish encrypted bytes atomically with no clobber and sync to disk. There
is no replacement, deletion or export command. Serialize KDF work with a nonblocking
mutex. Existing macOS agent behavior and server configuration remain unchanged.
The Unix source path includes Linux/macOS/Android/iOS; only Linux was compiled here.
Windows explicitly reports unsupported.

Native commands return only public key/fingerprint. Decrypted key material is
scoped to the operation and dropped; unlocking currently reads the public identity,
not an authenticated SSH session. The frontend clears passphrase fields before
awaiting IPC, never persists secrets and ignores late replies on unmount. JavaScript
and IPC copies are subject to runtime garbage collection, not guaranteed zeroization;
Rust wraps passphrases in Zeroizing and the key library owns private-key cleanup.

The form offers create/unlock and explicit public-key insertion, retaining other
entered keys and deduplicating additions. It explains passphrase recovery and the
current absence of migration/backup. No automatic key generation, network access,
VM mutation or credential-file discovery occurs. Actual creation by the app needs
its explicit button; tests create identities only in disposable directories.

Evidence: two focused native tests prove reopen/same fingerprint, encrypted bytes,
no replacement, wrong-passphrase refusal, busy handling, permissions, bounds,
symlinks, plaintext rejection and valid-envelope GCM tag tampering. Real Firefox/
React simulated IPC verifies passphrase confirmation/clearing, failed/successful
unlock and public-only deduplicated definition submission. Frontend build, native
library Clippy and changed-file formatting/diff checks pass.

Next: use this same identity for bounded, explicitly requested SSH authentication
and PTY through fresh strict host-key verification. Then validate native commands,
Android/iOS build, platform-private storage/backup behavior, keyboard and lifecycle
on an actual mobile runtime. No SSH/mobile roadmap item is closed by this slice.
