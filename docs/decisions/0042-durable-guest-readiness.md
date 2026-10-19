# 0042 — Durable bounded guest readiness

Status: standalone executor/storage locally tested; production API/worker pending.

Extend the schema-10 journal with optional readiness state: immutable start/deadline
(300 seconds), probe-dispatch marker, bounded attempt count and positive PID.
No migration, dependency or saved-plan/hash change. Claim only after the complete
validated successful power-start proof and earlier stages, never a succeeded flag
alone. A round performs bounded work and schedules another observation five seconds
later rather than blocking the worker for the full readiness window.

Before a new command, perform a read-only agent/network availability preflight.
Transient unavailability can be retried without recording an ambiguous submission.
Persist the probe marker before the fixed status command, then persist its PID
before observation. Scope/revision/marker/running checks stay mandatory. A retained
PID resumes with GET only after restart. A lost submission response has no known
PID: do not execute again; keep it uncertain until the original deadline expires.

[QEMU's guest-exec-status contract](https://www.qemu.org/docs/master/interop/qemu-ga-ref.html#command-guest-exec-status)
consumes a terminated process and its metadata. Distinguish a still-running probe
from a completed probe whose Cloud-Init/network result is not yet ready. Persist
completion by clearing that PID before permitting another fixed read-only probe.
The deadline never resets, and at most 60 commands may be started. A crash after
consuming a result but before recording it can lose evidence; do not invent success
or replay an unknown submission. Repeated unavailable status ends at the deadline.

Require clean Cloud-Init/IP evidence tied to instance, provider scope and plan.
Strict output or identity failures become guest_readiness_failed; ambiguous reads
remain unconfirmed. Deadline expiry (or backward clock movement/attempt exhaustion)
prevents late success and retains the VM for inspection. Readiness completes its
stage only. The overall operation succeeds only if no declared stages remain;
packages/snapshots are never silently skipped. Results omit raw stdout/stderr/PID
and do not prove SSH reachability, host identity or authentication.

Evidence: four focused storage tests cover proof/PID guards, retained deadline,
restart, completed pending probes, expiry and minimal-workflow success while
retaining later declared package work. Generated HTTPS tests prove PID recovery
with GET only, lost-command non-replay, deadline without provider calls and actual
exit-2 rejection. Three affected transport tests and library Clippy pass. These are
owned local fixtures; no real agent command, VM, infrastructure or credentials
were changed. Production create enqueue/worker, remote UI and real/mobile gates
remain the next work.
