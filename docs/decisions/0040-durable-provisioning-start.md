# 0040 — Durable power start after seed regeneration

Status: standalone start executor locally verified; production worker/readiness pending.

Extend the schema-10 seed journal with an optional power substep. Omission retains
old serialized reports; existing saved plan steps/hashes stay unchanged. Claiming
power creates an empty substep under an immediate transaction. Before-marker
crashes can be claimed after startup recovery; a live claimed job cannot be claimed
twice. A distinct configuration revision records power dispatch, independently of
the existing start_vm seed marker. No database migration or dependency is needed.

New submissions require regenerated_seed() and the validated prior stage chain,
current write capability, same provider scope and matching configured catalog.
Repeat the full stopped-target/settings/seed preview inspection and require the
retained revision, exact user/network hashes and compatible format. Commit power
intent before the fixed POST status/start. Proxmox's existing endpoint offers no
atomic digest guard; this remains bounded preflight, not a cross-request lock.

Bind returned qmstart task references to the target node and VMID, not the clone
source. Persist them on the stage and operation before polling. A lost/malformed
submission response leaves an unknown task and is never reclaimed. Known uncertain
tasks reconcile after their deadline with GET only, even with capability disabled
and without a current catalog. Startup recovery keeps both substeps and task
references intact. No seed or power mutation is replayed.

Require successful task completion and a separate running VM observation, rejecting
paused or other non-running QMP states. The typed scoped/plan-bound result explicitly
has guestReady=false. Only start_vm succeeds; the operation remains running and
wait_for_guest_readiness remains queued. A failed task is visible as failure;
unconfirmed task/power observations stay uncertain. Fresh precondition failures
never acquire a power-dispatch marker.

Journal/task reads reject mismatched seed proofs, power revisions and task targets.
Claim transactions roll back on corrupt evidence. Seed-only marker/results cannot
be interpreted as submitted power. A future readiness executor must validate this
start result and independently inspect the guest before reporting ready.

Evidence: two scoped storage tests cover both disk shapes, confirmation/proof/task
binding, before/after marker crashes, retained tasks and completion. Another storage
test covers corrupt proof/revision/task rejection and rollback. Generated HTTPS
exercises the actual seed then start executors across seven cases: success, lost
start reply, known-task recovery, changed seed, disabled writes, failed task and
paused QMP. Recovery GETs succeed with writes disabled/catalog absent. Three
seed-journal regression cases and scoped library Clippy also pass. All fixtures
are owned local state; no real provider, VM or credentials were touched.
