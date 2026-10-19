# ADR 0023 — Planned durable, explicitly confirmed lifecycle execution

Date: 2026-10-18
Status: Implemented through confirmed HTTP enqueue and local worker/TLS integration;
local subprocess lifecycle/restart verified; real-provider verification pending.

Migration 5 and eight storage integration tests implement confirmed input binding,
120-second undispatched approval expiry, durable dispatch/task linkage, uncertain
recovery and per-instance write exclusion. The lifecycle executor implements
fresh preconditions, committed dispatch, task linkage, bounded polling, independent
power observation and read-only reconciliation. The default worker disables new
writes; explicit runtime opt-in is locally tested. Fair sequential rounds process
both job queues. Confirmed HTTP enqueue and capability reporting are locally tested. See
[lifecycle execution boundary](../lifecycle-execution.md).

Fixed start/graceful-shutdown execution is implemented with local generated-HTTPS
fixtures. Generated-HTTPS real binary start/restart/no-replay verification now
passes; native action confirmation and real-provider evidence remain separate.
Actual infrastructure tests remain separately approved and opt-in. No force-stop,
lock bypass, credential change or generic executable payload is included.

Before dispatch, persist the precise action, instance/scope/node/VMID, definition
hash, expected config revision and a hash binding all approved inputs to the
idempotency key. Require an explicit target/action confirmation in the request,
an initially disabled server lifecycle capability and fresh passing preconditions.
A public matching tag is only an administrative association check; it is not
standalone authorization. Bind the caller's explicit action to the complete saved
target and revision, and expire undispatched approvals after a bounded interval.

Commit a dispatch-started marker before the provider POST. Send exactly one fixed
start or graceful-shutdown request with retries disabled. Validate the returned
UPID against expected node, action kind and VMID, and persist it before polling.
Do not serialize raw provider task identity or errors into diagnostic logs.

A lost/invalid reply, interruption after dispatch or unknown provider outcome
must never cause a repeated POST. Introduce a distinct uncertain state for
mutations; keep interrupted read semantics and immutable terminal results.
Known task references can be reconciled by GET after restart. Unknown task
references remain visibly uncertain and block conflicting mutation attempts until
an explicit recovery policy is applied; a fresh power read alone cannot prove
the original command succeeded. No automatic compensating write is permitted.

Poll provider tasks asynchronously and verify resulting power separately. Never
promote task acceptance or power-on to guest/SSH readiness. Preserve provider task
outcome and power observation as distinct evidence. Keep per-instance conflicting
writes exclusive while allowing read-only diagnosis.

Rebuild/extend SQLite constraints transactionally while preserving current read
jobs and instance snapshots. Enable API dispatch only after the worker, recovery,
idempotency and local HTTPS failure cases work. Explicitly update the preflight
capability/client contract when execution becomes available; the current false
flag and absent mutation controls must not silently become authorization.

Implementation acceptance includes zero-POST rejection for disabled capability,
wrong scope/revision/marker, expired or mismatched approval and conflicting work;
one POST for duplicate same-key requests; persisted task linkage; bounded task/
power checks; and restart/timeout/malformed-UPID cases with no replay. Real-host
ACL and mutation behavior remain unproven until approved disposable-target tests.
