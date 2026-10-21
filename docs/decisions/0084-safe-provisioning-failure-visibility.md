# 0084 — Safe provisioning failure visibility

Status: native protocol and React/browser acceptance pass; per-stage progress remains next.

## Decision

Remote creation/recreation and saved-operation recovery retain a typed optional
failure code alongside operation identity/status. The native decoder allows only
known provisioning-relevant codes; unknown codes, objects and unrelated mutation
failures are rejected through the existing fixed response error. Raw message,
stderr and provider output fields are ignored and never serialized to the renderer.
Older failed records lacking a code retain their generic status presentation.
Healthy queued/running/succeeded states cannot carry a failure, and a non-success
state cannot carry guest-readiness results.

The four React views share fixed local messages for package failure, unconfirmed
installation, retained-deadline expiration, snapshot preconditions/permission/state,
Cloud-Init/readiness and provider/approval errors. An unknown renderer-side code
gets a fixed fallback, never the raw value. Status still owns success/SSH gating;
a reason cannot turn an uncertain operation into success or authorize a mutation.
Package timeout explicitly remains uncertain. Snapshot task completion alone is
not treated as proof of a usable disk-only snapshot.

Messages introduce no execution, retry, consent or clear action. The same retained
operation is read after refresh/reconnect, and existing no-replay and stale-response
rules continue to apply. Old failure text disappears when a new validated status
replaces the operation. Per-stage scheduling/task/PID journals remain private.

## Evidence

Fifteen native create/prepare/recovery checks pass, including typed code redaction,
contradictory status/evidence refusal and actual generated-HTTPS retained GET with
private extra fields discarded. Twenty affected native recreation/recovery tests
also pass. Frontend typecheck/build and three real React/Firefox simulated-IPC
scenarios pass across active creation, recreation and both recovery views. They
assert package failure/timeout, snapshot uncertainty, generic unknown-code fallback,
no raw guest/provider text, no SSH on failure and unchanged exact retry/read behavior.
No server, database schema, dependencies, real VM or delivery artifact changed.
New exact-APK/native runtime evidence is not inferred from these separate checks.
