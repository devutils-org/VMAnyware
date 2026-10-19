# 0043 — Confirmed minimum create API and production worker

Status: complete local API/worker HTTPS fixture journey; real/mobile gates open.

Expose POST /api/v1/provisioning/create behind independent exact opt-in
VMANYWARE_PROVISIONING_ENABLED (default false). Invalid values fail before opening
the database/listener. Preserve loopback binding and whole-API auth behavior.
Use one Idempotency-Key and the complete existing Approval: saved definition/hash,
provider scope, plan hash, source/target VMIDs, node, storage, bridge and confirmed.
Return 202/Location only after durable intent, association and snapshots commit.
The handler performs no provider calls. Scope/current plan mismatches fail before
reservation; unknown/oversized JSON, Origin and duplicate keys are rejected.

The first route supports empty packages and snapshotAfterProvisioning=false with
an explicit image-default-user attestation. Unsupported features return 422;
never pretend package/snapshot work completed. Preview exposes configured scope
and runtime creation capability, with no live verification/reservation claim.
Omit absent scope from stored plans to preserve historical snapshot serialization.
The existing preview UI's capability type is now boolean; it gains no create button.

Wire every durable stage into the real main-process worker with the same validated
flag/catalog as the router. Each round runs one read, one lifecycle claim and at
most one provisioning stage, checking later stages before new clones. Existing
legacy read/lifecycle helpers keep their default behavior. Stage executors recheck
capability/configuration before new mutations; known tasks reconcile by GET.
Fixed readiness commands are read-only and do not require VM-write capability.
No automatic cleanup, forced stop or unknown mutation replay is introduced.

Production spawning exposed a non-Send form serializer retained by the Cloud-Init
future. Limit its lifetime to synchronous body construction before network await;
keep identical form content and preserve the existing explicit-user path.

Evidence: targeted API/config/disabled-worker tests cover persisted idempotent
queueing without provider access, complete confirmation/error guards, capability/
scope preview and safe invalid-process startup. Generated HTTPS runs actual Axum
create plus the production worker for both 8 GiB/no-resize and 40 GiB/resize paths,
with clone/resources/disk/Cloud-Init/seed/start/clean guest proof, restart after
clone, inventory progress and no duplicate writes on replay. Affected catalog/API
and explicit-user form checks, Rust Clippy/fmt and client type check pass.

No actual Proxmox/guest command, VM, credentials, TLS deployment or native/mobile
runtime was tested or modified. Remote native creation commands/confirmation are
next; mobile SSH authentication/build/runtime and approved real infrastructure
acceptance remain separate. Existing readiness/metadata/ACL limitations still apply.
