# ADR 0057: Bind destructive restore approval to a durable rollback journal

Date: 2026-10-20. Status: accepted; local storage, opt-in executor and confirmed HTTP, native commands and client review implemented; real acceptance pending.

Restore uses the shared operation history with kind `vm_snapshot_restore`.
Approval binds instance ID, provider scope, definition hash, node, VMID, live
40-character configuration digest, selected snapshot name and the 64-character
canonical saved-config fingerprint from ADR 0056. Both `confirmed` and
`acknowledgeDataLoss` must be present and true. Unknown fields, including start
or shutdown options, are rejected. The `vmanyware-snapshot-restore-v1` hash domain
separates this destructive request from snapshot creation and lifecycle approval.
The same idempotency key with identical inputs retains its original identity and
confirmation timestamp; changed inputs conflict.

Schema 12 rebuilds the shared operation table with the existing migration
transaction/foreign-key checks from ADR 0055. It retains existing records and
provisioning child journals. Restore joins the common active-mutation index for
start, shutdown, provisioning and snapshot creation. Incomplete provisioning,
including failed provisioning, blocks new restores even without active work.

Claim, exact approval and a nonnegative approval age of at most 120 seconds are
required before the durable dispatch marker. The marker is irreversible. Only a
`qmrollback` task matching node and VMID can be linked; a retained task cannot be
replaced. Exclusive startup recovery interrupts unmarked work and preserves marked
work as uncertain. Unknown submissions cannot be reclaimed. Known tasks can be
reclaimed only for reconciliation, with backoff, and cannot begin dispatch again.
No failure path may turn an ambiguous submission into a definite failure.

The completion record requires a retained task, successful provider task status,
matching selected name/fingerprint and an independent stopped observation for the
exact target. Unknown/paused/running QMP states are insufficient. These are guards
on evidence supplied by an executor, not provider calls made by storage. The
executor independently validates live ownership/lock/template/pending
state and the selected complete disk-only snapshot after task completion. A
provider-completion record does not prove restored guest file contents. Real
modify/restore/readback acceptance remains required by phase 0D and phase 4.

The storage slice adds no HTTP submission, runtime write capability, rollback
transport, executor or client consent UI. Preflight stays execution-disabled.
Tests use owned SQLite fixtures; no real VM or credentials are accessed.


The following executor slice connects this journal to a fixed one-shot rollback
POST with only `start=0`, following the
[upstream API contract](https://github.com/proxmox/qemu-server/blob/master/src/PVE/API2/Qemu.pm).
It repeats full preflight before dispatch and, after successful task polling,
requires fresh full preflight plus the original saved-config hash. Post-task live
configuration may differ from the old approved digest, but must be stable during
fresh reads. The result includes that independent final power observation; raw
configuration remains private. The worker services one restore after existing
queues. `VMANYWARE_RESTORES_ENABLED` is an independent exact opt-in, default false.
A known task is reconciled even with writes disabled; unknown submissions never
reclaim. The executor-only slice kept HTTP preflight execution-disabled; the following
HTTP slice connects capability reporting to submission.

Seven targeted generated-HTTPS tests, one exact configuration test and two actual
server-process configuration tests pass. Existing snapshot process/worker and
restore preflight evidence remain passing after targeted affected checks. No real
provider execution, guest readback, HTTP submission or client UI is claimed.


The confirmed HTTP slice exposes the roadmap's fixed per-instance/per-snapshot
restore route with a 4 KiB approval limit. Both path identities must match the
approval, both confirmations must be true and exactly one idempotency key is
required. The handler only enqueues; it does not contact Proxmox. The independent
restore flag now gates the route, is forwarded by the actual production router
and is reported by preflight independently of eligibility. Legacy router
constructors default it to false. Common bearer/Origin policy remains in force.
Same-key retries return the saved operation even after terminal/uncertain outcomes;
no consent renewal or provider replay occurs. Static diagnostics use the route
template, never the selected snapshot name. Ten new HTTP/generated-provider tests
and an affected actual-server router/worker process check pass. Native/client
review and real restored-content acceptance remain open.


Native commands now retain a separate restore review/key in the active connection.
Preparation binds the saved target, exact selected name, 13 required preflight
checks, destructive/stopped/disk-only constraints and saved-config hash. Failed
reviews remain readable without consent eligibility. Both confirmation booleans
are mandatory; native constructs the approval, never accepting renderer target/
revision/hash inputs at confirmation. Unknown submission retains the same key/body
for retry; known operations use GET only. Uncertain status cannot be cleared, and
success requires matching task-backed stopped target/snapshot evidence. Connection
generation changes discard late replies and local state without cancelling the
server operation. The shared bounded authenticated HTTPS helper now uses a generic
snapshot-action denial message for both creation and restore. No new dependency,
client persistence, server behavior or schema is introduced in this native slice.
Fourteen native tests plus two affected creation helper tests pass. React controls,
actual native restore journey and real guest-state acceptance remain open.


The client UI slice now selects restore from the named snapshot row, displays the
VM/snapshot and current-data loss, and requires two initially unchecked consent
controls after explicit preflight. A single parent selection locks restore and
snapshot-creation target changes during review/submission. Unknown response keeps
the same key/consent; a known operation exposes GET refresh only. The UI rejects
mismatched target/snapshot success evidence and distinguishes provider completion
from verified recovered guest contents. Generation guards suppress disconnected
review/status replies. TypeScript/Vite and focused Firefox simulated-IPC restore,
creation and listing checks pass. The subsequent actual native WebView restore now passes against generated HTTPS
infrastructure, including identical API retry after a deliberately lost response
and exactly one provider rollback POST. Read-only restore reconnect/restart
recovery and real storage/guest modify-restore-readback gates remain open.

Read-only per-instance restore history/detail routes now recover saved context
without provider configuration, write flags or a prepared native approval. They
filter kind/instance before decoding and bound history after that filtering, in a
consistent read transaction. Detail exposes recorded target/request/configuration
fingerprint and submission marker, not grant flags, input key/hash, raw task ID or
saved configuration. The target shape reuses the existing immutable recovery DTO;
no write payload or state machine is generalized. Stored approval/task/hash and
status/dispatch/failure consistency must pass, including rejecting unrelated
workflow failure codes. Successful result evidence is typed and checked against
that original approval and stopped observation, dropping unknown nested fields.
Eleven HTTP/SQLite tests pass with provider/write capabilities absent. This proves
database reopen/read isolation, not a new native UI or actual server-process
restart. Native/client saved-ID recovery and real guest readback remain open.

Native GET-only saved restore history/detail now use the retained authenticated
HTTPS profile and fixed instance/operation paths. They bind recorded target,
selected request/configuration fingerprint, status/dispatch/task markers and
successful stopped observation, without creating a prepared review or consent.
A connection generation change invalidates pending replies. Reuse the existing
restore request/operation validator for this second read use case and enforce
restore's pre-dispatch failure vocabulary. History independently bounds and
validates its reduced records; raw errors and unexpected nested data stay private.
Nine generated-HTTPS recovery tests and two affected restore validator/happy-path
tests pass. UI lookup, native app/server restart recovery and actual guest-state
proof remain open; no server/schema/dependency change accompanies these commands.

The remote client now provides explicit saved-restore history/row/manual-ID reads
and refresh independently of active restore consent. It binds recorded target/
operation/fingerprint evidence, distinguishes saved completion from current VM or
guest contents, and invalidates pending detail on ID edits or disconnect. Fresh
connections start empty but can retrieve saved server records. There is no prepare,
confirm, retry-write or clear control in this recovery panel. TypeScript/Vite,
focused Firefox restore recovery and affected snapshot creation-recovery checks
pass. Actual native app/server restart recovery remains the next integration gate.

The actual isolated native combined restore/recovery scenario now passes. After
one rollback with a lost-response API retry, both the actual application and server
restart. The server has no provider configuration and all write flags false.
Explicit native/client history, saved-ID and refresh reads use GET only, without
provider access or journal change. This is clean succeeded-record recovery; live
provider crash recovery, real disk/guest readback and platform/release gates remain
open. The source fixture is opt-in and never linked into normal product startup.
