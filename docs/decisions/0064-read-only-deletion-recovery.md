# ADR 0064 — Instance-scoped saved deletion recovery

Date: 2026-10-21
Status: Accepted for server HTTP/SQLite and owned process verification.

## Decision

Expose fixed GET deletion history/detail per retained instance, including deleted
associations, without provider configuration or write capability. Reads never claim,
re-enqueue, renew consent, reconcile against live infrastructure or change state.
Resolve operation kind and instance before decoding a journal so another instance's
damaged record remains indistinguishable from a missing operation.

Detail includes typed operation evidence, saved target, configuration digest,
full reviewed plan, approval hash and submission-started context. Consent flags,
idempotency key, raw provider UPID and raw approval JSON are not forwarded. The
saved digest/plan allow a future native reader to independently verify the full
hash; they are historical context, never a fresh review or replacement consent.

Validate retained approval/hash/target and matching qmdestroy linkage. Successful
records require the full matching task/absence/retention evidence and an actually
deleted association. Active records require a registered association. Historical
failed/interrupted records remain readable after a later successful deletion.
Known failed destroy tasks remain uncertain; a terminal failed record with a known
task is contradictory. Unknown submission remains uncertain without a known task.
Foreign failure codes and contradictory journal/result states fail closed.

Check persisted JSON sizes before parsing: approval at most 64 KiB, result 4 KiB,
with bounded timestamp fields. Strict result decoding rejects extra/private fields.
History filters by instance/kind before selecting the most recent 100 records in
descending creation-time/ID order. Validate all selected records in one read
transaction but return compact summaries, not 100 potentially large plans. Detail
remains available for older IDs. A new identity on a reused VMID cannot inherit
old-instance history.

## Evidence and limits

Eight targeted HTTP/SQLite tests pass: reopen without provider/flags, exact saved
context without consent/keys, unchanged queued/running/unknown/partial/interrupted/
failed states, cross-instance/kind privacy, filtered ordering/bounds/old detail,
authentication, saved-result corruption/size/task/hash/retirement refusal, old
failed/deleted history alongside a reused VMID, and changed saved-target refusal.
Corruption checks verify unchanged operation rows and SQLite data version.

The actual authenticated server/generated-provider deletion scenario now restarts
onto the same database with no provider or write flags and reads unchanged succeeded
detail/history for the deleted identity. No provider request is added. Synthetic
persisted-success encodings in negative tests are not provider-completion evidence.
No new schema, dependency, native command or client screen is introduced.

Next: native fixed GET saved-operation readers, then explicit history/manual-ID
UI and actual native restart evidence. Real infrastructure/platform acceptance
and recreation remain open.

## Native readers — 2026-10-21

Native `read_remote_deletion_history` and `recover_remote_deletion` now issue fixed
instance-scoped authenticated GETs without a prepared review. Typed history limits,
unique IDs, safe status/failure combinations and hash fields are validated. Detail
checks the saved target/digest/full-plan hash, exact operation ID, task/absence/
retirement evidence and dispatch consistency through the same native result
validator used by submission. Unknown or failed destroy tasks remain uncertain.

An internal typed approval is reconstructed only for deterministic saved-hash
verification. It is never placed in prepared state or returned as fresh consent;
no confirm/request key is recovered or sent. The public saved-context response
keeps distinct historical fields. Generation checks before/after validation discard
old replies on disconnect/reconnect. Read failures give safe read-only guidance.

Eight new focused native tests and all 14 affected submission/status tests pass.
The existing shared JSON/hash example remains the compatibility contract. Linux
native compilation/linking passes through these tests; no new schema/dependency,
GUI/mobile launch or real provider mutation is claimed. Next is explicit client
history/manual-ID recovery followed by the actual native restart journey.

## Client saved-record panel — 2026-10-21

A separate React panel is now mounted for both registered and retired rows. Explicit
history, row/manual-ID lookup and refresh use only native GET readers, without
mutation intent commands. Display saved reviewed/preserved scope and fixed planned
effects, recorded task/submission status and historical absence/retirement evidence;
never imply a current VM/storage observation from those records. Renderer guards
also bind target and full result hash. Edited IDs and new connections suppress old
results/errors and do not restore consent.

Frontend build and three sequential Firefox simulated-IPC scenarios (new deletion
recovery, affected submission and restore recovery) pass. No server/native/schema/
dependency change or actual application/provider/VM run is part of this UI slice.
Next is actual isolated native application/server restart acceptance.
