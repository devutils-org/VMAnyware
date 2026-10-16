# ADR 0004 — Implement independent definition storage while infrastructure is pending

Date: 2026-10-16
Status: Implemented; Phase 0 real-host/mobile acceptance remains pending.

The user requested continued autonomous implementation. The roadmap allows
testable seams when credentials/hardware are unavailable. Prepare the definition
slice independently, without representing provisioning/SSH feasibility as proven
or beginning real VM operations. This does not waive the Phase 0 exit rule.

Keep domain/schema/storage modules in the server crate for now. Introduce:

- serde_norway for strict typed YAML, consistent with the sibling Omniscient
  project's choice; reject unknown fields and avoid exposing parser values.
- ssh-key for parsing actual OpenSSH public keys rather than trusting a string
  prefix or resolving user file paths on the server.
- SHA-256 for recording canonical desired configuration independently of VM IDs.
- Rusqlite with bundled SQLite for a consistent local database; SQL migration
  versioning is explicit and transactional. Run database operations off the
  asynchronous executor. Tempfile is test-only for restart/migration scenarios.

Saving creates only a definition. Duplicate names conflict atomically; no silent
overwrite, provider call or VM creation occurs. Update/delete and operation
state are deferred until their lifecycle semantics are implemented. Network
binding remains loopback-only; remote control planes await authentication.
