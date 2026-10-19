# ADR 0021 — Extend the fixed read worker with instance observations

Date: 2026-10-18
Status: Implemented; tested against generated localhost HTTPS fixtures only.

An instance power read can take the existing provider deadline. Offer both a
synchronous point observation and an explicit queued refresh so the HTTP request
need not wait for Proxmox. Reuse the single durable worker with exactly two
read-only kinds: inventory_refresh and instance_observe. This is not a generic
job payload, lifecycle executor or mutation retry mechanism.

SQLite migration 4 retains all old operation IDs/keys/results/timestamps and adds
an optional instance foreign key. Inventory jobs have no instance; observations
must reference one. Request keys remain globally unique and are bound to kind
and instance identity. Reusing a key for a different kind/target conflicts. One
active observation per instance and one active inventory refresh may be queued;
the worker claims them sequentially with the existing atomic compare-and-set.

Before each instance provider read, match the persisted endpoint scope to the
current provider and validate the saved node/VMID. Scope mismatch fails without
transport access. Never replace the association or update registration state.
Successful results retain instance/scope identity and power/QMP observation;
they establish neither readiness nor ownership. Summary history exposes instance
identity but omits result bodies.

Recovery marks running reads interrupted and preserves them; it does not replay
them. Previously queued reads can run after restart, subject to fresh scope
checks. Same-key retries return the original record, including terminal failures.
A new explicit request key is needed for another read attempt. No VM write,
provider UPID linkage, cancellation or guest readiness is added by this decision.
