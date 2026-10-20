# ADR 0060 — Atomic retirement after independently observed deletion

Date: 2026-10-20
Status: Accepted for local implementation; real infrastructure acceptance remains open.

## Context

A successful Proxmox destroy task can leave disks behind. The retained association
must not disappear or silently target a replacement VM when its numeric VMID is
reused. Definition and operation history must survive deletion and support later
explicit recreation.

## Decision

A provider-created `DeletionCompletion` is required for success. Its constructor
performs GET-only reads: validate full approval/provider scope and the matching
qmdestroy node/VMID, independently read the exact task as succeeded, then check VM
and every approved candidate volume absent using ADR 0058's reviewed scope and the
permission-backed observer documented in deletion.md. A running/failed task,
remaining candidate or read error cannot produce completion evidence.

The proof has private fields, no deserialization or public constructor, and binds
the complete approval hash plus exact linked task. Store completion accepts it
only for a running, marked job with that task and the unchanged registered target.
It expires after 30 monotonic seconds; expired evidence needs fresh reads, never
another delete. A proof is not an external allocation lock.

Schema 14 rebuilds instances with registered/deleted states, retaining every
column and all operation/provisioning foreign keys. An immediate transaction
atomically writes approved absence/task evidence and marks the association deleted.
A failure rolls back both. The same request key still returns its original record.
Deleted identities cannot be updated or receive new operations. Target resolution
also refuses deleted associations, including observations queued before retirement.

Uniqueness of provider scope/VMID applies to registered associations. Recreation
may therefore use a new identity on an old VMID, while the deleted identity retains
its definition snapshot, provider reference and history. Another registered
association on that provider/VMID remains prohibited, even on a different node.
This does not itself submit recreation or prove the VMID stays available.

Both native list decoders accept deleted entries. Local and remote lists retain
those rows without active VM controls. Existing history reads remain available.

## Verification and limits

Generated HTTPS/SQLite tests cover independent task/absence proof, mismatched
approval/task, missing marker, expiration, atomic rollback, reopen, retained YAML/
history, new identity on a reused VMID and refusal of old-identity actions. Actual
schema-13 upgrade/orphan rollback and affected shared storage tests pass. Native
HTTP readers and simulated-IPC Firefox lists cover the shared status contract.

No DELETE transport, runtime flag, worker, HTTP confirmation or deletion UI is
enabled by this decision. Proof covers VM/candidate-volume absence, not preserved
media survival or auxiliary IPAM/ACL/firewall cleanup. Real backend cleanup,
partial failure reconciliation, native deletion journey and recreation remain open.
