# ADR 0025 — Provisioning intent and fixed stage journal

Date: 2026-10-18
Status: Storage, clone and CPU/memory executors implemented locally; production worker and enqueue API pending.

Migration 6 extends durable operations with fixed vm_provision intent and adds a
journal of the existing plan's ordered steps. It stores the original approval,
server confirmation time, canonical definition association, catalog snapshot and
exact plan JSON. Reads rebuild/validate the definition/catalog/plan and input hash
before using journal rows. No generic executable payload or workflow engine is
introduced. Clone and CPU/memory stages have claim/dispatch/completion methods; task linkage
exists only for clone. Other stages retain planned inputs but cannot execute yet.

Allocate the local association in the same immediate SQLite transaction as the
operation and journal. Its existing scope/VMID uniqueness excludes both manual
registration and another provisioner, across nodes. The association remains
`registered`: this means a saved local reference, never that a VM exists, is owned
or is ready. Failed/interrupted intent retains its association for explicit
recovery; no silent release or infrastructure cleanup is implemented.

Idempotency keys share the operations table with existing jobs. Identical key and
approved inputs return the original validated snapshot, even if the current
catalog changes. Changed input/cross-kind keys conflict and cannot extend approval
expiry. New requests must match the current saved definition and derived plan,
including explicit source/target/storage/bridge as well as hashes and scope.

An instance with incomplete provisioning rejects start/shutdown through a database
trigger, including terminal failure/interruption; diagnostic reads remain allowed.
The existing active mutation index additionally covers active provisioning work.
A later explicit recovery/release policy is required before abandoning an intent.

For clone dispatch, atomically claim the operation and first stage, revalidate
approved inputs and the original 120-second confirmation window, then commit both
workflow and stage dispatch markers. Link only a canonical source-bound qmclone
UPID to that stage and operation. Linking the same task is idempotent; replacing
it conflicts. Public operation JSON exposes only whether a task is retained.

Exclusive startup recovery transactionally recovers running stage rows and their
operations. A dispatched clone becomes uncertain and can only be claimed with a
retained task for future GET-only reconciliation. An unknown submission is never
automatically reclaimed. Undispatched work becomes interrupted; no confirmation
is renewed. No current worker claims these jobs, and no provisioning enqueue HTTP
route exists yet. Later-stage transitions must be implemented before enabling a caller. Later task kinds require corresponding identity checks;
the current reader fails closed on unsupported stage/task linkage.

Verification is scoped to the new SQLite intent tests, old migration/recovery
cases affected by schema 6, library/new-test Clippy and changed-file formatting.
It does not rerun unrelated provider, frontend, native or GUI suites. No new crate
or dependency change is involved. Real mutations remain explicitly approved and
opt-in, independently of this local reservation/confirmation machinery.

The clone executor checks the current catalog/plan before a first dispatch. Seven
observational preflight gates, a valid source digest, no lock or pending changes,
and stopped source power are required. Template revision provenance, execution
permissions and guest readiness remain unknown in public preflight; passing these
clone gates does not establish complete provisioning feasibility. The original
approval deadline is rechecked immediately before the durable dispatch marker.

A retained task reconciles by GET even when new creation is disabled or the current
catalog is unavailable. Unknown submissions never replay. Successful task and exact
stopped target observation persist evidence only on the clone stage; the operation
stays running, without a completed result, and all later stages remain queued.
After restart, completed clone evidence survives and the clone cannot be reclaimed.
Known uncertain tasks have a five-second reconciliation delay. Failed/uncertain
outcomes retain the target association and continue to block lifecycle writes.

Scoped evidence: eight intent storage tests, four generated-HTTPS executor tests,
nine affected preflight tests, and warnings-denied library/intent-test Clippy pass.
The completed-stage restart assertion was added and rerun separately. No production
worker/API wiring, real infrastructure execution or complete provision is implied.

Schema 7 retains resource dispatch revisions and adds its standalone executor; see
[ADR 0027](0027-durable-resource-stage.md).
