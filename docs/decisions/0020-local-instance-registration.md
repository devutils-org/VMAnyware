# ADR 0020 — Persist definition/target associations before lifecycle execution

Date: 2026-10-18
Status: Implemented for local registration and reads; lifecycle execution pending.

Use one real preparatory use case: explicitly associate a saved definition with
a proposed existing Proxmox target. Persist an opaque instance ID, exact saved
canonical YAML/hash/revision, endpoint scope, node and VMID. Return only after
commit. No provider discovery, write, ownership assertion or readiness inference
is part of registration. The instance state is `registered` only.

Derive provider scope from SHA-256 of the configured canonical HTTPS origin.
Never accept it from the request or store credentials/endpoint URLs in the
instance record. Token rotation retains scope; endpoint changes produce a new
scope. This is an address scope, not a verified cluster UUID: aliases, failover,
VMID reuse and infrastructure behind a reused hostname need future reconciliation.
The hash is not secret and must never serve as authentication or authorization.

Enforce uniqueness on (provider scope, VMID), independent of node. A cluster VMID
moving nodes must not silently gain a second local association. Do not silently
update the recorded node. Keep definitions reusable: one definition may have
multiple registered instances, and its snapshot is copied transactionally so
future edits cannot rewrite the original instance inputs. Require the caller's
expected definition hash; stale inputs conflict instead of binding newer data.

SQLite migration 3 adds one table; no crate or runtime dependency is needed.
The loopback API rejects browser Origin writes and returns fixed errors. List
reads return the newest 100 summaries without YAML; detail reads remain available
by ID across provider reconfiguration. There is no deletion, retention or UI yet.

Before any lifecycle operation, independently verify current configured scope,
VM identity/ownership, node and state, then persist intended action and provider
task linkage with an explicit uncertain-outcome/recovery policy. An association,
matching VMID or prior power observation cannot authorize a write. In particular,
this decision does not replace approval for concrete disposable infrastructure
operations or complete the Phase 0/Phase 2 acceptance gates.
