# 0095 — Release the VMID of a creation that never produced a VM

Status: accepted (2026-10-25)

## Context

Creation registers the instance association (definition snapshot + VMID) when
the request is queued, before the clone runs. On the real home host, a clone
refused by its precondition check, and another refused with 403 because the
token lacked rights on the target VMID, both left an association `registered`
on a VMID where no VM exists. The unique registered-VMID index then blocked
every later creation on that VMID, and the UI listed a ghost "Active" VM that
could never be observed.

## Decision

- Schema 16 adds the instance status `released`, via the same table rebuild
  pattern as schema 14 (triggers referencing `instances` are dropped and
  recreated; foreign keys are checked after the rebuild).
- When the clone stage fails before dispatch, or at dispatch with no provider
  task and an authorization refusal (401/403), the failure transaction marks
  the association `released`. No provider VM can exist in those cases.
- Failures after a provider task exists (or uncertain submissions) keep the
  association `registered`: a VM may exist and must be inspected or deleted.
- Released associations are excluded from the environment list, stay readable
  by ID for history, are immutable, and reject new operations.

## Consequences

The VMID becomes reusable immediately after such failures. Existing ghost
associations created before schema 16 are not rewritten by the migration;
reset or inspect them explicitly. Schema 16 databases are refused by older
binaries, consistent with the upgrade policy.
