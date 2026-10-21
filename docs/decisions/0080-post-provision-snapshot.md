# 0080 — Optional final provisioning snapshot

Status: durable worker and generated-HTTPS/SQLite evidence implemented; API admission now passes under ADR 0081; native client integration pending.

## Decision

snapshotAfterProvisioning creates a disk-only snapshot as the final stage of the
original provisioning operation. The retained definition/plan already declare it.
The stage waits for guest readiness and successful declared packages, if any.
A deterministic `ready_<operation-id>` name and description containing the operation,
definition hash and plan hash bind the snapshot to this exact creation. No separate
operation or implicit new user approval is synthesized by recovery.

First dispatch requires both provisioning and the independent snapshot write gate,
the current matching catalog/approval and the existing storage/name/tree/identity/
pending-change preflight. The observed digest must equal the retained configured
clone digest. The marker and digest are committed atomically before the one POST.
The digest lives in an optional private seed-journal member: the existing SQL stage
config_digest column remains reserved for resource configuration. No schema change
or relaxation of that constraint is needed. Older journals omit the new member.

The returned task must match node, VMID and qmsnapshot kind, and is retained before
polling. One task-status read runs per worker round. Known task recovery requires
neither enabled write flags nor the current catalog and performs GETs only against
the same provider scope. Missing/invalid/lost task replies remain uncertain and are
never automatically resubmitted, including after restart. The original mutation
still blocks conflicting work. No rollback or cleanup VM mutation is implied.

Provider task success alone is insufficient. The executor verifies the retained
instance marker, unlocked non-template target and the exact snapshot metadata:
name, description, disk-only flag and no incomplete provider state. The digest may
change after snapshot creation. Only matching evidence completes the final stage
and provisioning operation; prior guest-readiness evidence remains its result.
Raw provider failure details are discarded. This is not guest application-consistency
proof, restoration proof or an independent backup.

## Evidence

Six focused tests pass through actual generated HTTPS and SQLite for both gates,
package ordering, lost reply, crash with unknown task, known-task GET recovery,
unsupported storage, failed task, forged completion/task binding and one full
POST/task/metadata success. The completion test additionally rejects corrupted
retained digest and malformed stored evidence. Earlier template/clone/resource/
disk/start fixture setup now accepts a YAML parameter while retaining its default
behavior, so this feature is tested with a real retained snapshot declaration.
Snapshot fixtures normalize only their owned operation identity before any dispatch.

Six affected package-worker tests, the existing confirmed-create API/worker journey
and the standalone snapshot production-round scheduling test pass. Logs are recorded
in STATE.md. Real snapshot storage/guest-state proof and API/native activation remain
open at this worker milestone. ADR 0081 records subsequent API activation; native
client integration and real guest acceptance remain open.
