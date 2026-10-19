# ADR 0029 — Retained disk volume, resize task and stage outcomes

Date: 2026-10-18
Status: Durable disk storage and standalone HTTPS executor tested locally; production caller pending.

Migration 8 adds provisioning_disk_dispatch, a private one-row-per-intent record
of the exact volume identifier and configuration digest used for expansion. The
existing resource-stage digest column remains specific to CPU/memory; its schema
is unchanged. Migration 8 and user_version advance in the normal atomic migration
transaction. No dependency is added.

Only an existing expand_disk stage immediately after completed resources can be
claimed. Validate the complete clone/resource evidence chain, input/scope/instance
bindings, requested CPU/memory, stopped-target observation and revisions before
committing a claim. There is no disk stage for equal source/requested disk sizes.
Undispatched queued/interrupted work may be claimed after restart; uncertain work
requires a retained canonical target-bound resize task and the reconciliation
backoff. An unknown submission is not automatically reclaimed.

Before dispatch, bind the original full-plan approval, exact resource completion
revision and bounded volume on the configured storage. Persist the volume/revision
and the disk dispatch marker in one immediate transaction. Clear the operation's
current task reference at this boundary; the clone task remains in its own stage.
A canonical resize task must match the target node and VMID. Retaining the same
task is idempotent; replacing it or linking a source-bound/other-kind task fails.
Linking the disk task updates its stage and the operation's current task reference.

Successful disk completion requires its retained dispatch/task, successful task
status, exact volume/device/requested bytes, CPU/memory and stopped target, plus
instance/scope/plan bindings and a valid observed revision. Persist evidence only
on expand_disk. The workflow stays running with no completed result; later stages
remain queued. Failed/interrupted work keeps its local target association and the
existing incomplete-provisioning guard continues to block lifecycle mutations.

After a dispatch marker, only definitive authentication refusal without a retained
task or a known failed task may become failed. Other ambiguous submissions/tasks/
observations remain uncertain. Exclusive startup recovery preserves the dispatch
record/task/evidence and never makes an unknown submission replayable. A completed
disk cannot be claimed again. The standalone executor now performs fresh target/volume checks, one PUT, task
persistence/polling and GET-only reconciliation against local HTTPS fixtures. No
production worker/API invokes disk execution yet.

Evidence: six new SQLite tests cover binding/completion, pre/post-marker restart,
unknown submission exclusion, definite versus ambiguous failure, corrupt evidence/
revision/task with claim rollback, and schema-7 upgrade. Thirteen affected prior
intent/resource tests and two older migration cases pass in scoped runs. Scoped
library/intent-test Clippy passes. Those storage tests do not establish a real VM, backend allocation, guest
filesystem growth or readiness.


The executor checks new-dispatch capability, provider scope and current catalog
before its first attempt. It selects only the mapped device's observed volume,
validates the stopped clone/resources/source size and the retained resource
revision, then commits the volume/revision/dispatch marker. The fixed transport
repeats the read-only guard after that marker. Even a configuration change at this
boundary that prevents PUT leaves uncertainty with no automatic replay.

Retain a validated target-bound resize task before polling. Only successful task
status followed by exact independent configured-size observation can complete the
disk stage. Missing/malformed task replies remain unknown submissions, including
an actual dropped HTTPS connection. Known-task polling or observation errors stay
uncertain and resume with GET only. Capability disablement or an unavailable
current catalog does not prevent known-task reconciliation; a changed provider
scope still blocks all requests to the wrong endpoint.

Four targeted HTTPS executor tests pass: one resize across successful/failed/
unknown task and post-task observation cases; known versus unknown restart with
creation disabled; pre-dispatch capability/catalog/scope/volume/revision/resource/
pending/power gates; and a configuration change after marker commit with no PUT.
The four affected existing transport tests also pass after extracting the shared
read-only guard; scoped library Clippy passes. This adds no schema/dependency or
real-infrastructure change and still cannot finish whole-plan provisioning.
