# ADR 0009 — Persist explicit endpoint-scoped SSH host trust in the experiment

Date: 2026-10-16
Status: Proven local store implementation; native inspection/confirmation
integration follows in [ADR 0010](0010-native-ssh-host-inspection.md). Production
credential handling and remote/mobile validation remain pending.

Extend the working localhost SSH fixture before adopting a product SSH stack.
Use a separate managed public-key store with explicit creation and reopening,
not a user's OpenSSH files. Canonical endpoint identity includes the requested
host and port without resolving DNS aliases into a shared identity.

Return an opaque first-use challenge for an unknown public key and require its
exact SHA-256 fingerprint when confirming it. Observation is never permission
to authenticate. After explicit confirmation, reconnect and verify the actual
key against persisted state. An existing different key is a failure, including
when first-use approval became stale due to another writer. Provide no automatic
replacement, rotation or reset method.

A strict bounded versioned JSON document, private managed directory, stable
sidecar lock and same-directory atomic replacement suffice for this small
experiment. Reread state under the lock before update, preserve concurrent
independent additions and allow identical confirmations idempotently. Fail
closed on corrupt, duplicate, unreadable, oversized, missing or unsupported data;
only explicit fresh-store creation initializes empty trust. Bound lock waiting
and refuse symlink leaves. This is not a sandbox against an attacker controlling
the user's parent filesystem or account.

Keep these dependencies and APIs outside the application's workspace. The
fixture proves the confirmation/state transitions and SSH rejection behavior,
not a human UI, persistent client private-key storage, OpenSSH interoperability,
mobile behavior or a production library decision. Before a native UI can trust
an observed key, define its Rust-owned IPC challenge lifetime/replay policy and
an explicit user confirmation with an independently verified fingerprint.
