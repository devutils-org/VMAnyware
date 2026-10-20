# ADR 0055: Durable snapshot creation intent in the shared operation journal

Date: 2026-10-20. Status: accepted; storage/executor/confirmed HTTP submission implemented; client controls pending.

Snapshot creation uses operation kind `vm_snapshot_create` in the existing shared
journal, rather than a second operation history. Approval binds the complete
instance/provider/definition/configuration target, exact validated name and
description, and explicit confirmation. A domain-separated input hash binds
idempotency; retrying the same key never changes identity or confirmation time.
The initial path remains disk-only as in ADR 0054.

Schema 11 rebuilds the operation table to admit this kind while retaining rows,
child provisioning journals and constraints. The common active-mutation index
covers snapshot/start/shutdown/provisioning. Failed/incomplete provisioning also
blocks new snapshot intents. The migration follows SQLite's
[generalized table-rebuild procedure](https://www.sqlite.org/lang_altertable.html#otheralter):
foreign keys are temporarily disabled on the unopened migration connection,
changes run in one immediate transaction, the complete reference graph is checked
before commit, and enforcement is restored before returning a usable store.
Violations abort the transaction. No schema text is edited directly.

Storage must claim an intent and check the exact retained approval and its
120-second age before committing an irreversible dispatch marker. That marker
is never reset. The retained task must match node, VMID and `qmsnapshot` worker.
Restart preserves uncertain marked work; unknown submissions are not reclaimed.
Only uncertain work with a known task may be reclaimed for read-only reconciliation,
with backoff. Undispatched running work is interrupted at exclusive startup.

Success requires a retained successful task plus matching observed snapshot name,
description, disk-only metadata and no incomplete provider state. Unconfirmed
snapshot metadata remains uncertain, not failed/succeeded. Snapshot journal
errors reuse the existing typed lifecycle-storage error vocabulary; this does not
change lifecycle operations or conflate their approval payloads/hash domains.

Tests cover migration preservation/rollback/enforcement, request identity,
concurrent connections, target conflicts, expiration, corruption, crash windows,
task identity and completion guards. The original storage slice implemented no provider write; the subsequent executor
and confirmed HTTP slices connect the journal as described below. The client
flow and real-provider acceptance remain open.


The following executor slice connects fixed disk-only transport to this journal.
`VMANYWARE_SNAPSHOTS_ENABLED` is independent of lifecycle/provisioning flags and
defaults false. The sequential production worker services at most one snapshot
claim after the existing queues. It repeats preflight before the marker, persists
the matching task before polling, then checks task success plus retained instance
marker and exact completed disk-only metadata. Recovered known tasks use GETs
only even if dispatch is disabled. Unknown submissions remain unreclaimable.
Generated HTTPS and real-process configuration tests pass. Confirmed HTTP enqueue is now guarded by that same independent flag and binds
the complete target/request with idempotency. Preflight reports runtime capability
independently of eligibility; client controls remain pending.
