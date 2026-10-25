# 0091 — Confirmed creation through the local native client

Date: 2026-10-25
Status: implemented with targeted native/transport and client-state checks;
actual home-provider creation remains pending explicit approval/execution.

The home acceptance journey exposed a product gap: local mode could inspect and
preflight a real template, but creation and application-key preparation were
available only in the remote HTTPS workspace. Expose the existing key UI locally
without rewriting the user's YAML. Add local confirmed creation at the existing
loopback control-plane boundary; do not relax the remote HTTPS requirements.

Two fixed native commands submit POST /api/v1/provisioning/create with a retained
Idempotency-Key, and read GET /api/v1/operations/<validated ID>. Reuse the remote
plan/approval and operation/readiness validation contract instead of introducing
a second wire contract. Preserve providerScopeId in local preview serialization.
The server independently validates the confirmation against saved definition,
current provider scope/catalog, runtime flags and dispatch preconditions.

Confirmation requires an enabled plan, matching preflight and all seven required
metadata observations passed. Image/readiness unknowns remain explicitly unproven.
The UI displays target/resources/declarations and requires explicit confirmation.
It stores only the public plan, request key and known operation/instance IDs in
localStorage before transport. An unknown reply retries only that same request;
a known operation resumes with GET only. Store failure prevents new submission,
corrupt retained state blocks new work, and stale responses retain operation IDs
without updating an abandoned view. Pending work locks local target/definition/
connection edits. Acknowledgement requires a terminal validated operation and
clears preview/preflight so another creation needs fresh review.

New commands have the same explicit local-main-window capability boundary as
existing commands. No dependency, database migration, arbitrary route transport,
provider passthrough, automatic VM cleanup or SSH trust is introduced. Guest-ready
success does not prove Mac-to-guest routing or SSH authentication. This slice
shows overall operation state; local per-stage progress and fuller history-based
recovery remain follow-up work. Native OS identity creation remains a user action.

Verification: three new native command tests, preview scope roundtrip regression,
six affected remote-create tests and five client-state tests pass. Frontend
TypeScript/build, production-library strict Clippy, native macOS build, formatting
and 84-command manifest/capability check pass. Test-target strict Clippy reports
five preexisting duplicate-module/await-held-lock errors in unrelated test files;
no warning suppression is added. No real Proxmox creation was executed by agent.
