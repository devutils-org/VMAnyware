# ADR 0027 — Durable CPU/memory stage and read-only reconciliation

Date: 2026-10-18
Status: Storage and standalone executor tested locally; production caller pending.

Migration 7 adds a private configuration digest to the provisioning stage journal.
Only a dispatched configure_resources stage may retain one; malformed digests are
rejected. A claim validates completed clone evidence, instance/scope/plan bindings,
source-bound task success and stopped-target observation before advancing. A
retained resource dispatch revision must match the completed clone's revision.
The claim, input validation and status change share one immediate transaction.

The resource stage can be claimed after clone success while the workflow is
running, or after exclusive startup recovery. An interrupted resource stage with
no dispatch marker can resume after fresh capability/catalog/target checks. Once
its marker is committed, every subsequent execution uses only GET, including
when creation is disabled or the current catalog is unavailable. Uncertain work
has the existing five-second reconciliation delay. Missing/changed provider scope
remains uncertain after dispatch; it never authorizes a replacement PUT.

The original 120-second approval window gates the first clone dispatch. Approval
covers the entire retained plan: later resource work may outlast that window but
cannot change inputs, restart clone or renew confirmation. Before a first resource
PUT, require explicit capability, the matching current catalog/plan, the exact
stopped clone revision and no pending changes; atomically retain the revision and
resource dispatch marker. The fixed transport repeats target checks and passes
the digest to Proxmox. This implementation has no production worker/enqueue route.

The synchronous resource API has no UPID. Retain the earlier clone task as workflow
history, not as evidence for resource completion. Read applied resources after a
successful, malformed or missing PUT response. Exact independent observation may
complete this stage even after connection loss; otherwise retain an uncertain
outcome. No post-dispatch error is converted into a definite failure or automatic
retry. Pre-dispatch capability/catalog/target failures are definite stage failures.

Completion requires the dispatched stage, matching instance/scope/plan, exact CPU
and memory and a valid stopped-target observation. It persists the new observed
digest and resources only on this stage. Clone evidence and association remain;
operation result stays absent and its status stays running. Disk, network,
Cloud-Init, start and readiness have not executed. Failure/uncertainty continues to
block independent lifecycle mutations and retains the local target association.

Evidence: five new SQLite tests cover stage ordering/input binding/completion,
pre/post-marker restart, retained approval time, corrupt clone evidence and claim
rollback, schema-6 upgrade and dispatch-revision corruption. Three local HTTPS
executor tests cover one PUT, actual connection loss/malformed response, applied
or uncertain outcomes, disabled GET-only restart and pre-dispatch gates. Existing
intent cases and affected schema-2/schema-4/future-version checks pass in targeted
runs; scoped library/intent-test Clippy passes. No complete provisioning flow,
normal application launch or real provider mutation is established by these tests.
