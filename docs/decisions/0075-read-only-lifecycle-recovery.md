# 0075 — Instance-scoped saved lifecycle evidence

Status: server/store API, native stateless readers and React recovery UI implemented; actual Linux clean app/server restart passes; Android/real-target acceptance remains open.

## Decision

Add GET `/api/v1/environments/{id}/lifecycle-operations` and GET
`/api/v1/environments/{id}/lifecycle-operations/{operation_id}`. Reads work without
provider/catalog configuration or lifecycle execution enabled. They never claim,
submit, cancel or replay work. Whole-API bearer protection still applies.

Resolve instance/operation/kind scope in SQL before decoding saved intent, preventing
another instance's damaged journal from leaking through a different error. History
accepts registered or archived associations, filters start/shutdown before limiting
to the newest 100 by created_at descending then id descending, and validates every
returned job. Detail returns the original saved action and target, public operation
and submissionStarted flag. No confirmed flag, request key, provider task identifier
or replacement approval is returned. The target is historical, not current provider
configuration, current power/guest state, or renewed consent.

Reuse the durable lifecycle job reader to verify approval shape/hash, kind/instance
and task linkage. Bound its saved approval bytes to the API's 4096-byte contract in
the SELECT, before loading/decoding an oversized intent. This bound also protects
worker/job-reader callers. Validate terminal result action/scope/instance/task/power
against saved approval and serialize only typed public result evidence. Failed or
uncertain states remain recorded facts; GET does not alter them.

Invalid IDs, missing/wrong-instance/wrong-kind detail and missing history instances
return fixed 404. A corrupt matching journal returns fixed 503 without private saved
content; a corrupt entry makes its history fail closed instead of silently omitting
work. A valid unrelated instance can still return an empty history.

## Evidence and limits

Six router/SQLite tests pass: reopened saved start/shutdown without provider/flags,
archived association, queued/running/unknown reads without claiming, scoped/kind/ID
isolation and wrapped authentication, pre-limit filtering/latest-100 ordering,
corrupt/oversized approval handling without cross-instance disclosure, and success
sanitization/changed-power refusal. Nine affected lifecycle storage tests also pass.
No migration or provider/VM operation is added. SQL corruption/archival in tests
uses owned disposable fixtures and does not pretend to execute real deletion.

This is store reopen and HTTP-router evidence, not a new actual process restart,
native app, Android or real-provider proof. Next add stateless native fixed GET
readers, then explicit history/manual-ID UI and actual app/server restart recovery.
Do not reinstall a mutable prepared request or confirmation from this response.


## Native fixed readers

`read_remote_lifecycle_history(connectionId, instanceId)` and
`recover_remote_lifecycle(connectionId, instanceId, operationId)` now use fixed
HTTPS GET paths through the active profile. They install no key, review, approval,
consent or submitted operation. IDs are validated before network access; generation
checks reject stale disconnected replies. History validates instance, latest-100
bound, unique canonical IDs, descending timestamp/ID order, typed action/status,
known failure codes and status/task/failure coherence. Detail validates historical
target/hash/action/ID, dispatch/task consistency and matching saved outcome evidence.

The existing local/remote lifecycle operation validator now delegates to a pure
evidence matcher. Read-only recovery can reuse that matcher without constructing
an approval with fabricated digest or consent fields. Existing callers keep their
approval validation path. The two new commands use the explicit local-main ACL
(74 current production commands). Extra server fields are not forwarded.

Six focused native generated-HTTPS tests pass, plus three affected local lifecycle
and nine affected remote lifecycle tests. These are native Linux protocol tests;
recovery UI and actual app/server restart acceptance are still next. Existing server/
Android delivery snapshots are unchanged and predate this unfinished recovery path.


## Explicit recovery UI

RemoteLifecycleRecovery is rendered for registered and archived remote rows. Loading
history, selecting an entry, submitting a copied operation ID and refreshing detail
are explicit reads. Target/action/task/power evidence is checked before display;
recorded success is labelled historical and never establishes current VM existence,
power, readiness or SSH identity. The panel has no consent checkbox, submit/retry
mutation or SSH action. Unknown work remains visible without pretending success.

Editing the operation ID invalidates outstanding detail; component/connection cleanup
invalidates history and detail responses. No saved ID/confirmation is automatically
restored on reconnect. Empty history and failed read are distinct. Frontend build and
focused Firefox simulated-IPC evidence are recorded in STATE.md, separately from the
native protocol tests. Next acceptance is an actual isolated app/server restart
with provider removed, writes off and GET-only recovery traffic.


## Actual clean native restart acceptance

The --lifecycle-recovery owned Linux fixture now passes through actual React/WebKit,
production native HTTPS, authenticated server/worker and generated provider. After
start/shutdown and identical retry for each lost accepted reply, both app and server
are new processes on the same database. The new server has no provider/catalog and
all write flags false. Explicit history, both saved action details, copied ID and
refresh use GET only; both full journal records, instance/definition and provider
read/task/power/write counters are unchanged. No token, ID, intent or consent is
automatically restored in the new app. Recorded start success is labelled historical,
not current power/guest/SSH proof. This is clean isolated restart, not live crash or
real-provider acceptance. Shared restart regression outcome is recorded in STATE.md.
