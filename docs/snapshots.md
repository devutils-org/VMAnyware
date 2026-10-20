# Snapshot workflows

Registered-instance snapshot listing works through the remote client and actual
native/server fixture. Disk-only creation now has preflight, a durable journal,
a confirmed HTTP enqueue route and an opt-in runtime executor. The remote client has review/consent/status controls; actual isolated native creation now passes against generated infrastructure.
Client read-only recovery passes browser and actual isolated app/server restart
checks. Restore and real-provider validation remain open. Snapshots depend on
the original VM/storage; they are not independent backups.

## List

`GET /api/v1/environments/{id}/snapshots` uses the retained provider scope, node
and VMID. Caller query parameters cannot retarget the request. Example response
(illustrative, not a real infrastructure result):

```json
{
  "instanceId": "0123456789abcdef0123456789abcdef",
  "providerScopeId": "<configured endpoint scope hash>",
  "node": "pve-lab",
  "vmid": 202,
  "snapshots": [{
    "name": "before-work",
    "description": "Before changing the workspace",
    "createdAt": 1700000000,
    "includesRam": false,
    "parent": null,
    "providerState": null
  }]
}
```

Entries are sorted by name. `createdAt` is Unix seconds, or null when missing/
zero. Missing RAM metadata is null, not false. The synthetic Proxmox `current`
entry describes the live VM and is excluded. Incomplete `snapstate` is retained
as `providerState`; neither listing nor an absent state proves restore eligibility.
Duplicate/unsafe names or malformed metadata fail the whole read. Unknown provider
fields and raw error bodies are not forwarded. The provider response limit is
2 MiB; existing bounded provider timeouts apply.

Unknown instance: 404; missing provider: 503; changed scope: 409 before networking.
Provider authentication/permission failures: 502 `provider_unauthorized`/
`provider_forbidden`; timeout: 504 `provider_timeout`; other provider/TLS/invalid
responses: 502 `provider_unavailable`. Empty success is distinct from those
failures and does not establish snapshot support. The fixed provider GET is
`/nodes/{node}/qemu/{vmid}/snapshot`, documented with `VM.Audit` in the
[upstream implementation](https://github.com/proxmox/qemu-server/blob/master/src/PVE/API2/Qemu.pm).

## Creation preflight

`POST /api/v1/environments/{id}/snapshot-preflight` accepts:

```json
{"name":"before-work","description":"Before changing the workspace"}
```

Body limit: 32 KiB. Name: 2–40 ASCII characters, starting with a letter, followed
by letters/digits/underscore/hyphen, excluding exact `current`. Description is
optional, at most 4 KiB UTF-8, without controls other than newline/tab. Unknown
fields, including `includesRam`, are rejected. This path is fixed disk-only.

The report contains `instanceId`, `providerScopeId`, `definitionHash`, `node`,
`vmid`, `configDigest`, the exact `request`, `includesRam: false`, `checks`,
`creationPreconditions` and `executionEnabled`. The last field reflects only the
independent runtime opt-in; it can be true while prerequisites fail. Caller must
review both. All provider reads have an overall 20-second deadline: retained
identity/template/lock/pending/digest guards, actual snapshot feature support,
exact name availability, incomplete snapshot work and a final target recheck.
No approval, operation, reservation or provider mutation is created by preflight.

Invalid input: 400; missing instance: 404; changed scope: 409; missing provider:
503. Failed checks return HTTP 200 with `creationPreconditions: failed`.
Provider/invalid-data errors return 502 `snapshot_preflight_unavailable`, timeout
504 with the same code. No raw provider details are exposed.

## Confirmed creation

`POST /api/v1/environments/{id}/snapshots` requires exactly one `Idempotency-Key`
(1–64 letters/digits/dots/hyphens/underscores) and the complete approval JSON:
`instanceId`, `providerScopeId`, `definitionHash`, `node`, `vmid`, `configDigest`,
`request: {name, description}` and `confirmed: true`. Use the exact target/request
reviewed in preflight. The body limit is 32 KiB; unknown fields are rejected.
Origin-bearing requests are forbidden, and configured bearer authentication
protects this route as it does the rest of the API.

Only independent `VMANYWARE_SNAPSHOTS_ENABLED=true` enables submission. Absent or
exact false disables it; other values refuse process startup without echoing
input. Lifecycle/provisioning flags do not enable snapshots. Historical router
constructors also default this new capability off.

An accepted request returns HTTP 202 with a durable `vm_snapshot_create` operation
and `Location: /api/v1/operations/{operationId}`. Enqueue never contacts Proxmox or
claims snapshot success. Poll that operation explicitly. Identical same-key
requests retain identity and confirmation age, including terminal results.
Changed same-key inputs, a different active mutation or incomplete provisioning
conflict. Disabled mode rejects submission; retained operations remain readable
through the operation/history GET routes.

Errors include 400 `invalid_request_key`/`invalid_snapshot_approval`, 403
`snapshots_disabled`/`origin_forbidden`, 404 `instance_not_found`, 409
`provider_scope_changed`/`snapshot_target_changed`/`operation_conflict`, and 503
`provider_not_configured`/`storage_unavailable`. JSON type/unknown-field rejection
uses 422; malformed JSON 400, oversized bodies 413, wrong content type 415.
Authentication failures return 401 when bearer protection is configured.

## Durable execution and recovery

Schema 11 shares operation history and active-mutation exclusion with lifecycle/
provisioning. Approval hashes bind exact request and target. The worker repeats
preflight and reviewed digest checks before committing an irreversible dispatch
marker, with a confirmation age of 0–120 seconds. Thus stale live configuration
can be accepted into the queue but fails before provider dispatch. One fixed
URL-encoded POST sends only `snapname`, `description`, `vmstate=0`; redirects and
transport retries stay disabled. Only the approved node/VMID/`qmsnapshot` task is
persisted, before polling.

Success requires task completion plus independent retained instance-marker,
non-template/unlocked state and exact complete disk-only snapshot metadata. The
old digest is not required after completion because snapshots change VM config/
parent metadata. Unknown submission stays uncertain and cannot be reclaimed.
Known tasks reconcile by GET only even with dispatch disabled. Unconfirmed
metadata uses `snapshot_state_unconfirmed` and keeps the instance occupied.
A task failure does not mean partial provider state has been cleaned up.

This proves neither guest/application consistency, quiescing, free space,
VM.Snapshot write permissions on a real host nor recoverability. Restore, deletion,
RAM capture and automatic post-provision snapshots remain open. See
[ADR 0054](decisions/0054-snapshot-creation-preflight.md) and
[ADR 0055](decisions/0055-durable-snapshot-creation-journal.md).

## Client and evidence

After connecting and explicitly loading the remote workspace, choose **List
snapshots for VM …**. The production native command uses the active verified HTTPS
profile/token, bounds metadata and discards disconnected replies. The UI compares
instance/scope/node/VMID before displaying unknown/incomplete metadata, parents,
escaped descriptions, target and backup wording. Errors clear prior results.
The creation form described below is available; restore controls remain open.

Server preflight, storage, executor and HTTP tests use owned SQLite and generated
HTTPS peers. HTTP coverage includes exact consent/target, key conflicts/concurrent
retries, origin/authentication/size guards, runtime capability and actual queue ->
worker -> provider -> operation success or stale-target failure. No HTTP enqueue
itself performs a provider read/write.

Listing additionally passes frontend build, isolated Firefox simulated IPC,
actual native HTTPS command tests and the real isolated Linux WebView -> Tauri ->
verified HTTPS proxy -> authenticated Axum -> generated HTTPS provider journey.
That fixture requires exactly three provider GETs and zero provider writes.
Evidence: `.lab/native-deps/snapshot-native-tests.log` and `snapshot-ui-check.log`;
see the [native harness](../experiments/lifecycle-native/README.md) and
[owned native prefix](native-linux-prefix.md). Minimal-session portal/accessibility
warnings remain. Listing evidence does not prove native creation, packaged startup,
Android/physical-device behavior, real-provider access or any real snapshot/restore.

## Native retained creation commands

The actual client crate now registers four creation commands, using the active
verified HTTPS profile and in-memory token:

- `prepare_remote_snapshot(connectionId, requestKey, instanceId, name, description)`
  reads the exact registered instance, then POSTs only the read-only preflight.
  It checks instance/scope/definition/node/VMID, exact request, disk-only mode and
  all nine prerequisite codes/statuses before retaining the review. Raw server
  check messages and unknown fields are not forwarded. A revision guard discards
  superseded preparations and stops their follow-up request.
- `confirm_remote_snapshot(connectionId, requestKey, confirmed)` accepts only
  explicit consent for an eligible retained review. Approval JSON is constructed
  from those retained values, rather than new renderer-supplied target fields.
  An unknown submission keeps its key/body and prevents replacement. A known
  operation cannot be POSTed again; read its status instead.
- `read_remote_snapshot_operation(connectionId, requestKey)` GETs only the retained
  operation ID. It verifies operation kind, instance, identity and complete
  task-backed disk-only success evidence against the review. Failed/interrupted/
  uncertain states require a recognized safe failure code, with no success result.
- `clear_remote_snapshot(connectionId, requestKey)` cancels an unsubmitted review
  or acknowledges a verified terminal result. It refuses in-flight/unknown/
  uncertain work. Disconnect forgets native memory but never cancels server work.

Native responses are bounded to 256 KiB and must confirm bearer authentication.
Errors do not expose response bodies or tokens. Submission status must be 202,
reads/preflight 200. A socket loss, malformed response or rejected status keeps
submission uncertainty; retry can only use the retained key/approval.

Ten targeted native tests pass against generated verified HTTPS peers, covering
exact requests, consent, retained retry, actual socket loss, missing authentication
marker, response bounds, target/check/result mismatch, disabled prerequisites,
disconnect and stale preparation. The crate and normal command registration
compile with the owned WebKitGTK prefix. The remote creation form calls these commands and its complete isolated native
journey now passes as recorded below. This
slice adds no persistence: after closing/disconnecting, operation recovery needs
the server's retained operation/history through a future explicit client flow.


## Remote creation form

Choose **Prepare snapshot for VM …** under a loaded registered environment, enter
a name/description, then **Review snapshot creation**. Selecting a VM alone makes
no request. Review shows the target, exact disk-only request, prerequisites and
backup/consistency limitations. Submission requires an enabled server, passed
checks and a separate unchecked-by-default confirmation checkbox. Blocked reviews
can be cancelled without submission.

Only one snapshot review/operation is active in the remote workspace, matching
the native retained intent. Target and inputs stay locked during review and after
submission. A lost reply exposes only **Retry same snapshot request**, preserving
key and inputs. Once an operation ID is known, **Refresh snapshot operation** uses
GET only. Uncertain work cannot be acknowledged or replaced. A verified terminal
result can be acknowledged before preparing another snapshot. Results never imply
verified restoration or application consistency.

Disconnect/unmount suppresses pending results and does not cancel server work.
The UI displays the operation ID for later recovery; it does not persist the
request. The separate read-only recovery flow below finds server records after
reconnect without restoring consent or a submission intent. Preserve the distinction between that
limitation and in-connection same-key retry.

Verification: frontend TypeScript/Vite build and the focused Firefox
`--snapshot-create` simulated-IPC fixture pass. It covers target selection without
automatic work, disabled/failed/mismatched review refusal, escaped descriptions,
consent, input/target exclusion, same-key lost-reply retry, queued/running/uncertain/
success handling, terminal acknowledgement and stale preparation after disconnect.
This is real React/browser interaction with mocked IPC, not native creation proof.


## Real native creation evidence — 2026-10-20

The isolated Linux WebView -> production retained commands -> verified HTTPS
proxy -> actual authenticated Axum/worker -> generated HTTPS provider creation
journey passes with embedded frontend assets. The driver reviews/cancels without
consent, reviews again with unchecked consent, confirms, refreshes through the
known operation GET, independently lists the snapshot, acknowledges and disconnects.
The provider checks the exact fixed target and UTF-8/special-character description.

Runner assertions require one provider snapshot POST, five snapshot GETs, at least
one task poll, unchanged registered instance and exactly one succeeded durable
snapshot operation. Its result must report a known successful task and complete
disk-only snapshot metadata. This strengthens the UI assertions with the actual
server journal. Native invoke is never replaced; TLS, WebKit sandbox and generated
X11 cookie verification stay enabled. The owned process groups, listeners and
temporary data are cleaned up. Minimal-session D-Bus/portal/accessibility warnings
remain without preventing the asserted flow.

Use the [native harness](../experiments/lifecycle-native/README.md) `--snapshot-create`
mode; evidence is `.lab/native-deps/snapshot-create-ui-check.log`. This supersedes
the earlier native-creation-unverified boundary, not the real-provider, mobile,
packaging, guest-consistency or restore gates. Reconnect/restart operation recovery
still needs an explicit read-only client path.

## Read-only server operation recovery

Two instance-scoped GET routes now support recovery without a prepared native
intent, provider configuration or enabled write capability:

- `GET /api/v1/environments/{id}/snapshot-operations` returns `{instanceId,
  operations}`. Each entry contains `id`, snapshot `name`, `status`, safe `failure`
  code, `providerTaskKnown` and `createdAt`. The newest 100 entries are selected
  after filtering by the exact instance and snapshot operation kind. History omits
  descriptions, results, request keys, approval hashes and raw provider task IDs.
- `GET /api/v1/environments/{id}/snapshot-operations/{operation_id}` returns
  `{operation, target, request, submissionStarted}`. Target contains the recorded
  instance/provider scope/definition hash/node/VMID; request contains the original
  validated name/description. The response is saved context, not a new approval
  or a current provider observation. It omits consent, configuration digest,
  request key, input hash and raw provider task identity.

Reads use a consistent SQLite transaction and validate the retained approval hash
and task linkage. Succeeded results must match the recorded target/request and
complete disk-only/task evidence; only typed public evidence is forwarded, so
unexpected result fields are discarded. Corrupt own-instance journals fail with
503 `storage_unavailable`, without partial history or raw details. Unknown,
non-snapshot or another instance's operation returns 404
`snapshot_operation_not_found`, before decoding that other journal. Unknown
instance history returns 404 `instance_not_found`. IDs are canonical lowercase
32-character hex. Configured bearer protection applies to both routes.

Neither route contacts Proxmox, claims a job, extends confirmation, enqueues work
or modifies status. Running and uncertain records remain unchanged. A stored
success does not establish that the snapshot still exists or can be restored.
Client recovery must keep those historical/current-state distinctions explicit.

Seven targeted HTTP/SQLite tests pass, including reopen without provider or write
capability, unchanged running/uncertain state, cross-instance/kind isolation,
per-instance bounded history, corruption refusal, saved-success normalization and
bearer guards. Native commands and client recovery controls are now connected as described below;
their complete actual-native restart journey remains unverified.

## Native read-only recovery commands

`read_remote_snapshot_history(connectionId, instanceId)` and
`recover_remote_snapshot(connectionId, instanceId, operationId)` now use only the
fixed scoped GET routes above. They require canonical IDs, an active verified
HTTPS/token profile and the server's bearer-authentication marker. Shared native
response bounds apply. Disconnected/replaced connections discard pending replies.

History validation bounds entries to 100, rejects duplicate IDs, wrong instance,
invalid names and inconsistent status/failure/task flags. Detail validation binds
both requested IDs, recorded target and request, submission marker, recognized
status/failure codes and exact completed disk-only evidence for success. Unknown
fields are not forwarded. Errors use static read-oriented messages without raw
provider responses or tokens.

These reads work with fresh connection memory and never install a prepared
creation approval, request key or retry state. The existing creation and recovery
paths share operation/evidence validation without constructing a fictional
preflight. Seven generated-HTTPS tests pass; the two affected creation validator/
happy-path tests also pass. Native command registration compiles. Client recovery
controls now pass the focused browser check below; their complete real native
journey remains the next task.


## Client history and saved-ID recovery

Each registered environment now provides **Find snapshot operations for VM …**
and a saved operation ID form. Reads happen only on explicit action. History rows
show the snapshot name, recorded status/time and an inspect control. Empty history
and errors remain distinct. Inspecting a row, entering a saved ID or refreshing
uses only the native read commands above.

The detail view checks the selected instance/scope/node/VMID and operation identity,
then shows the saved target, original request and recorded status. Success also
requires matching disk-only/task evidence. Wording explicitly separates saved
completion from current snapshot existence and restoration. Unknown submissions
show that no task was confirmed; failed/interrupted records do not claim creation.
There is no creation consent, cancellation, acknowledgement or retry-write control
in this recovery panel.

Editing the ID invalidates pending results; disconnected/unmounted panels ignore
late success/error replies. After a new connection, the panel starts empty and can
read the server history or a saved ID without a prepared creation request. It
stores no client approval or credential. Native/server checks remain the data
boundary; the UI adds selected-target matching before display.

Verification: TypeScript/Vite build and the focused Firefox `--snapshot-recovery`
scenario pass with simulated IPC. It covers explicit empty/error/history reads,
row/manual-ID recovery, saved/current wording, escaped description, uncertain/
failed states, target refusal, edited-ID stale detail, disconnected stale error
and recovery in a fresh connection without creation commands. Real WebView IPC,
actual app-process restart and real infrastructure recovery remain separate gates.


## Actual native app/server restart recovery evidence — 2026-10-20

The combined isolated fixture now creates one snapshot through the real native
UI/commands/server/provider, closes that application, stops the server cleanly,
and starts new server/application processes over the same SQLite journal. The
restarted server has snapshot writes disabled and no Proxmox configuration. The
fresh application explicitly reconnects, finds the operation via history, reads
its saved ID and refreshes the record. It does not restore token fields, creation
intent or operation ID implicitly, and shows recorded/current-state distinctions.

Assertions require GET-only proxy traffic in the recovery phase, no additional
provider reads or POST, and an unchanged complete saved operation after recovery.
The original creation still requires exactly one provider POST, task polling and
completed disk-only metadata. Example/server builds and the targeted
`--snapshot-recovery` native scenario pass. Evidence:
`.lab/native-deps/snapshot-recovery-ui-check.log`; see the
[native harness](../experiments/lifecycle-native/README.md).

This proves recovery of a succeeded recorded operation after clean app/server
restart in the owned Linux/generated-infrastructure fixture. It does not prove a
crash during live provider execution, current snapshot existence, restoration,
normal installed startup, mobile recovery or real Proxmox behavior. Running/
uncertain recovery has separate journal/native/browser test evidence.

## Read-only restore preflight

`POST /api/v1/environments/{id}/snapshot-restore-preflight` accepts only
`{"snapshotName":"before-work"}`, with a 1 KiB body limit and the existing 2–40
character named-snapshot grammar. `current`, paths and additional fields such as
`start`, `skiplock` or confirmation are rejected. This does not expose rollback.

The bound response contains instance/scope/definition/node/VMID, live
`configDigest`, final `observation`, the request, optional `selectedSnapshot` metadata and
`snapshotConfigHash`, static `checks`, `restorePreconditions`, `destructive: true`,
`startsVm: false` and `executionEnabled` (the independent restore runtime flag). Missing or unsupported selection
returns a failed report rather than inventing a snapshot. Creation/lifecycle write
flags do not enable restore. A passed report is not authorization or restore proof.

The initial path requires stopped power and a complete disk-only snapshot whose
saved configuration retains this instance marker. It checks list metadata and
canonical API-visible configuration fingerprint twice, selected-configuration
snapshot support, and final live identity/pending/power. Full reads are bounded
to 20 seconds. No raw saved config, provider credentials or disk configuration is
returned. No claim/enqueue, timestamp renewal, provider write or automatic power
change occurs.

Input errors return 400 `invalid_restore_request`; unknown instance 404, scope
change 409, missing provider 503. Provider/invalid response failure returns 502
`restore_preflight_unavailable`, timeout 504 with that code. No raw provider body
is exposed. Provider snapshot support is not verification of rollback contents or
write permission. RAM/unknown RAM, incomplete/foreign/missing/changed snapshots,
running VMs and changed live configuration cannot pass.

Six generated-HTTPS/API tests pass, including the full fixed GET sequence,
canonical ordering, recorded-target binding, disabled execution despite creation
opt-in, unsupported/changed states, input/scope guards and private provider errors.
Existing lifecycle identity checks and snapshot-name validation also pass after
sharing helpers. See [ADR 0056](decisions/0056-disk-only-restore-preconditions.md).
The following slices add destructive approval/journal and rollback task execution.
Actual restored contents and client flow remain unverified.

## Restore approval and journal

`POST /api/v1/environments/{id}/snapshots/{snapshotName}/restore` enqueues a
`vm_snapshot_restore` intent. Approval contains `instanceId`,
`providerScopeId`, `definitionHash`, `node`, `vmid`, `configDigest`,
`request: {snapshotName}`, `snapshotConfigHash`, `confirmed: true` and
`acknowledgeDataLoss: true`. Both confirmations are mandatory. The hash binds the
exact snapshot configuration reviewed by preflight, with a separate restore hash
domain. Unknown fields/start options are rejected. Idempotent retries preserve the
original approval age; dispatch expires after 120 seconds.

Schema 12 preserves the shared history/provisioning graph and excludes concurrent
restore/create/start/shutdown/provisioning mutations on the same instance.
Incomplete provisioning also blocks restore. A dispatch marker is committed once;
only a matching `qmrollback` node/VMID task can be retained. After restart, unknown
submissions remain uncertain and cannot be reclaimed; known tasks permit read-only
reconciliation, never a second write. Completion requires a retained successful
task plus matching snapshot name/fingerprint and a stopped observation of the
exact VM. This records provider completion, not restored guest-content proof.

The production worker now services retained restore intents sequentially after the
existing queues. `VMANYWARE_RESTORES_ENABLED` is a separate exact `true`/`false`
flag, absent/false by default; creation/lifecycle/provisioning flags cannot enable
rollback. Invalid configuration stops startup before creating a database. The same
flag gates HTTP submission and is reported by preflight independently of eligibility.
Existing router constructors keep restore off by default. Client restore controls
remain pending.

Before a new dispatch, the executor repeats full preflight and requires the
confirmed live digest and selected-config fingerprint, then commits the durable
marker. The fixed one-shot POST is
`/nodes/{node}/qemu/{vmid}/snapshot/{snapshotName}/rollback` with only `start=0`.
It retains the matching task before bounded polling. After task success it repeats
full read-only restore preflight, requiring the original selected-config hash,
complete disk-only metadata and stopped owned target. The old live digest may
change during rollback; it must instead remain stable during these fresh reads.
Failure of post-task checks stays uncertain. Recovered known tasks use GET only,
including with the runtime flag disabled. Unknown submissions never replay.

Seven generated-HTTPS executor/transport tests pass, plus exact opt-in and actual
server-process configuration tests. They cover one fixed POST, changed/unsafe
preconditions, lost/invalid submission responses, GET-only restart recovery,
failed postchecks and no redirect/retry. Evidence is synthetic provider completion,
not storage rollback or guest-content verification. Native retained commands are
now available as described below; client controls and generated/native journeys
remain next. See
[ADR 0057](decisions/0057-durable-snapshot-restore-journal.md).


### Confirmed restore HTTP behavior

Body limit: 4 KiB. Supply exactly one `Idempotency-Key` (1–64 letters, digits,
dots, underscores or hyphens) and both explicit confirmations. The path instance
and snapshot name must exactly match the body. Query parameters cannot retarget
the operation. Unknown fields, including start/skiplock, are rejected. The handler
checks configured provider scope and saved instance target before enqueue; it does
not contact the provider. The executor revalidates both revisions and all eligibility
checks before any dispatch. Body shape and approval fields are listed above.

Response: `202 Accepted`, the current operation record and
`Location: /api/v1/operations/{operationId}`. A same-key identical retry returns
that same record, including terminal or uncertain status, without renewing consent
or creating another submission. Read status through the existing operation GET.
The handler never returns the approval, raw provider task ID or saved configuration.
A succeeded result describes provider task completion plus independently checked
stopped target/snapshot evidence; it does not establish recovered guest contents.

Origin-bearing requests are refused. Common bearer protection covers this route
when configured. Invalid key/path agreement/false confirmation/bad hash: 400;
malformed JSON: 400; missing/unknown typed fields: 422; oversized body: 413;
capability off: 403 `restores_disabled`; provider absent: 503; saved instance absent:
404; changed scope/target, reused key with changed input or active mutation: 409.
All errors are static and omit request/provider details. Ten focused new HTTP and
generated-provider tests pass, with an affected production-process check proving
that the actual server applies the same flag to router and worker. Client review
and real-provider modify/restore/readback acceptance remain open.

## Native retained restore review

Production native commands now register:

- `prepare_remote_snapshot_restore(connectionId, requestKey, instanceId, snapshotName)`
- `confirm_remote_snapshot_restore(connectionId, requestKey, confirmed, acknowledgeDataLoss)`
- `read_remote_snapshot_restore_operation(connectionId, requestKey)`
- `clear_remote_snapshot_restore(connectionId, requestKey)`

Prepare reads the saved instance, then sends only the selected name to the fixed
restore-preflight path using the retained verified HTTPS/token/CA profile. Review
binds both responses' instance/scope/definition/node/VMID, exact snapshot name,
current digest, saved-config fingerprint, stopped observation and all 13 distinct
check codes. Destructive must be true and startsVm false. A passed report also
requires complete disk-only selected metadata and a valid fingerprint; a failed
missing-snapshot report can be displayed but cannot be confirmed. Server messages
and unexpected fields are dropped. The renderer cannot supply a URL, approval,
revision or selected-config hash to the confirm command.

Both independent confirmation booleans must be true. Only then does native build
the full retained approval and submit it at the matching instance/snapshot path.
It retains the key, review and submitted state before awaiting a response. Unknown
outcomes allow only the identical key/body retry; replacing or clearing that
intent is blocked. Once an operation ID is known, further submission is blocked
and status uses GET only. Successful evidence must match target/name/fingerprint,
retained successful task marker and stopped target observation. Inconsistent
uncertain/definite status combinations are rejected. Uncertain work cannot be
acknowledged as terminal; unsubmitted or verified terminal work can be cleared.
Clearing forgets local review, not a server operation.

Disconnect/reconnect resets native restore state and rejects late preparation,
submission or status replies without cancelling server work. No client approval or
credentials are persisted. Fourteen targeted native tests pass with generated
verified HTTPS, including both confirmation flags, retained inputs, unknown-response
retry, disabled/failed review, malformed/authentication/size bounds, mismatched
selection/result, in-flight/disconnected/stale replies, missing snapshot and
GET-only uncertain-to-terminal tracking. The actual Tauri crate and production
command registration compile using the isolated `.lab/native-deps` prefix. Two
affected shared-helper creation tests also pass. This is native command evidence,
not a React/WebView restore journey, Android rebuild or real VM rollback. Next:
the actual native restore WebView/server/provider journey after the client controls
described below.

## Remote client restore review

After explicitly listing an instance's snapshots, choose the restore review action
on the named snapshot. Selection itself performs no prepare or submission. One
restore panel displays the retained instance, node/VMID and exact selected name,
then offers an explicit prerequisite review. It explains current-data/settings loss,
the already-stopped/disk-only requirement, absence of automatic power changes and
that snapshots depend on original storage rather than independent backup.

The checked report shows the same selected snapshot and static prerequisite labels.
Disabled capability or failed/inconsistent checks cannot enable confirmation.
Two independent checkboxes start unchecked: confirm the displayed VM/snapshot and
acknowledge losing changes since that snapshot. Both are required before the
native confirm command. The renderer sends only connection/key and both booleans;
retained native state supplies target/revisions/approval. Selection of another
snapshot/VM or snapshot creation is locked while review/submission is retained.
User description text is escaped. Cancelling an unsubmitted review forgets it.

If no operation ID is received after confirming, keep the displayed key and use
only the same-request retry. Once an operation ID is known, only GET refresh is
offered. Queued/running/uncertain states are distinct; uncertain work cannot be
acknowledged. Success needs exact task-backed target/name/fingerprint and stopped
observation evidence, with explicit wording that guest contents/application
consistency remain unverified. Save the operation ID before disconnecting; closing
the panel or connection does not cancel server work. Only verified terminal work
can be acknowledged. Reconnect starts with no restored consent, and generation
checks suppress late review/status replies after unmount/disconnect.

Verification: TypeScript/Vite production build and
`node experiments/instances-browser/check.mjs --snapshot-restore` pass. The focused
Firefox scenario uses actual React controls with simulated native IPC: selection
without requests, disabled/running/mismatched review, separately required consent,
escaped description, one retained unknown retry, GET-only status, wrong success
hash refusal, terminal acknowledgement, cross-target/creation exclusion, fresh
connection and stale review/status suppression. The affected existing
`--snapshot-create` and `--snapshots` scenarios also pass. No Rust/server/schema
or dependency changed in this UI slice. Next: actual isolated native WebView ->
commands -> authenticated server/worker -> generated-provider rollback journey.
Browser evidence alone does not prove that journey, Android behavior or a real
storage/guest restore; phase 0D and phase 4 remain open.

## Actual native restore integration evidence

The owned Linux `--snapshot-restore` fixture now passes actual React/WebKit ->
production retained native commands -> verified HTTPS/token proxy -> actual
Axum/SQLite/worker -> generated HTTPS provider. The example registers the production
restore commands and uses embedded frontend assets, a private temporary WebView
context, generated certificate/token/display cookie and the isolated dependency
prefix. Fixture scripts are not linked into normal application startup.

The driver selects the named existing snapshot, refuses the first running-target
review, cancels an eligible review, prepares again with both consents unchecked,
confirms independently, handles a deliberately lost HTTP response and retries the
same native key/approval. The API sees two identical path/key/body submissions;
the provider sees exactly one rollback POST with only start=0. Saved operation GET,
post-restore snapshot list, terminal acknowledgement and disconnect succeed.

Independent runner assertions require one succeeded restore journal record with
task evidence, exact target/name, the expected canonical saved-config fingerprint
computed in Python and stopped power. Snapshot-config/power/list reads cover the
review and full before/after dispatch checks. The live digest changes after the
synthetic rollback, and the instance association is unchanged. Runtime creation/
lifecycle opt-ins remain absent; only the explicit restore flag enables this queue.
No real provider, VM, credential file or infrastructure is contacted.

Verification: embedded native example and current server builds pass; the targeted
actual native scenario passes after correcting its new CA-textarea driver helper.
Log: `.lab/native-deps/snapshot-restore-ui-check.log`; initial driver failure kept
separately. JS/Python syntax, Rust formatting and diff review pass. Product source,
dependencies/schema and previously passing unit/browser/Android suites did not
change and were not rerun. See the [native fixture](../experiments/lifecycle-native/README.md).
This proves the isolated Linux integration and no-replay API retry, not real storage
rollback, recovered guest files, app/server restart recovery or mobile packaging.
Next: explicit read-only saved restore-operation recovery after reconnect/restart.
Phase 0D/phase 4 real modify/restore/readback acceptance remains open.

## Read-only saved restore operation recovery

The server now exposes:

- `GET /api/v1/environments/{id}/restore-operations`
- `GET /api/v1/environments/{id}/restore-operations/{operation_id}`

Both routes work without Proxmox configuration or any write capability. They only
read the shared journal; no preflight, claim, task poll, submission, consent renewal
or automatic operation recovery occurs. Responses describe recorded work and its
historical stopped observation, never current VM/snapshot existence or recovered
guest contents. Common configured bearer protection still applies.

History filters by instance and `vm_snapshot_restore` before limiting to the newest
100 records, ordered by createdAt then ID descending. Rows contain `id`,
`snapshotName`, `status`, safe `failure`, `providerTaskKnown` and `createdAt`.
Potentially larger request/result bodies are excluded. Existing instance with no
restore history returns an empty list; unknown instance returns 404.

Detail contains `operation`, recorded `target` (instance/scope/definition/node/VMID),
original `request: {snapshotName}`, recorded `snapshotConfigHash` and
`submissionStarted`. The fingerprint is a historical comparison token, not a new
approval or disk-content proof. No confirmation/data-loss flags, live config digest,
request key, confirmation time, input hash, raw task ID or saved configuration is
forwarded. Reads use a consistent SQLite transaction. Scope and kind are resolved
before decoding another instance's journal; missing/wrong-kind/wrong-instance or
invalid operation IDs return 404 `restore_operation_not_found`.

Stored approval/hash/task identity and dispatch/status/failure consistency are
validated. A saved success needs matching retained task, selected name/fingerprint,
exact target and stopped power evidence. Unknown fields, including nested fields,
are discarded through typed result normalization. Changed snapshot/hash/power,
wrong task kind, uncertain work falsely marked failed/interrupted, missing-task
completion claims or failure codes belonging to another workflow are refused with
503 `storage_unavailable`, without altering the record or leaking raw journal data.

Eleven focused HTTP/SQLite tests pass, including database reopen, pending/terminal/
uncertain states, scope/kind isolation, authentication, history filtering/order/
bound, unknown-field suppression, safe corruption refusal and unchanged journal.
This slice adds no schema, dependency, provider mutation or client controls.
Next: native fixed GET commands and explicit client history/saved-ID lookup after
reconnect, followed by actual app/server restart recovery evidence.

## Native saved restore recovery

The production native bridge now registers
`read_remote_restore_history(connectionId, instanceId)` and
`recover_remote_restore(connectionId, instanceId, operationId)`. They accept only
canonical lowercase IDs and use fixed GET paths on the retained verified HTTPS/
token profile. Responses require the bearer-authentication marker, exact HTTP 200
and the existing 256 KiB bound. Failures use static read-oriented messages and
never forward response bodies, provider details or access tokens.

History validates selected instance, at most 100 unique IDs, valid selected names,
bounded creation timestamps and consistent status/failure/task markers. Detail
binds requested instance/operation IDs to recorded target/request/fingerprint,
submission marker and exact task-backed successful snapshot/stopped observation.
The shared restore validator rejects uncertain work presented as failed and
pre-dispatch failure codes inconsistent with restore. Unknown returned fields,
including nested observation fields, are discarded through typed serialization.

Both reads recheck connection generation after await; old replies cannot survive
disconnect or reconnect. They do not install a review, consent, request key,
retry marker or new submission state. Reading a historical record does not prove
current VM power, snapshot existence or recovered guest contents, and cannot grant
permission for another restore.

Nine focused native recovery tests pass against generated verified HTTPS,
including fixed authenticated GETs without intent, recorded states, mismatch/
fingerprint/power/dispatch refusal, bounded unique history, authentication/status/
size/private-error handling, unknown field suppression, bad IDs before access and
late disconnected/reconnected replies. Actual Tauri command registration compiles
with the isolated native prefix. Two affected existing restore validation/happy-path
tests also pass. No server, schema, dependency or frontend changed in this slice.
Next: explicit client saved-restore history/ID recovery, then actual app/server
restart proof through the native WebView.

## Remote saved restore lookup

Each instance now has a saved-restore panel independent of snapshot listing and
restore consent. It offers explicit recent history, row inspection, manual saved
operation ID and read-only refresh. No automatic history/detail request occurs.
This works from a fresh connection without a prepared restore; native/server reads
require no provider/write capability. Creation and restore histories stay separate.

Recovered detail must match the selected instance/scope/node/VMID and requested
operation ID/kind. Success additionally requires recorded submission/task markers,
matching snapshot name/fingerprint and stopped target observation. The panel calls
these saved records and explicitly leaves current VM state, snapshot existence
and recovered guest contents unchecked. Uncertain/failed/interrupted records are
not displayed as verified restoration; reads never offer consent, rollback retry,
cancellation or acknowledgement. Save an operation ID from active restore before
leaving, then use it here after reconnect.

Editing the ID invalidates pending results and clears old detail/errors; late
responses cannot replace the new selection. Unmount/disconnect similarly discards
late reads or failures. A fresh connection starts with an empty ID/result rather
than restoring consent or retained native submission state.

Verification: TypeScript/Vite build and focused Firefox `--restore-recovery`
scenario pass with actual React/simulated native IPC. Covers explicit history,
empty/read-failure separation, row/manual-ID lookup, recorded/current wording,
unknown/failed outcomes, wrong target/fingerprint refusal, stale edited-ID detail,
late disconnected history error and fresh-connection lookup. It asserts no creation
or restore preparation/confirmation/clear command is invoked. The affected
existing `--snapshot-recovery` scenario also passes. No Rust/server/schema or new
dependency changed. Next: actual native app/server restart with provider settings
removed and writes disabled, verifying GET-only saved restore recovery and an
unchanged journal. Browser evidence alone does not prove that process journey or
real guest restoration.

## Actual saved restore recovery after app/server restart

The native `--restore-recovery` combined fixture passes a real native restore once,
then exits that app and cleanly restarts the actual server on the same database.
The new server removes all provider configuration and explicitly disables restore,
snapshot creation, provisioning and lifecycle writes. A second native application
process has no old token, operation input, review or consent automatically restored.
It explicitly receives the owned fixture connection and reads saved restore history,
row detail, copied ID and refresh through production native commands and API.

Acceptance assertions pass: recovery proxy traffic is GET-only, provider read and
POST counts do not increase, and the entire succeeded operation remains identical.
The initial restore still proves one provider POST despite two identical API
submissions after a lost response, with task/hash/stopped-target evidence. The
recovery panel describes historical completion and exposes no submission controls.

Evidence: `.lab/native-deps/restore-recovery-ui-check.log`; current embedded native
example/server builds and the combined scenario pass. Syntax/format/diff checks
also pass. No product/schema/dependency changes or real provider access occurred.
This proves clean succeeded-record recovery in isolated Linux, not a crash during
real rollback, restored guest files, actual current VM state, other platforms or
normal installed startup. Real phase-0D/phase-4 modify/restore/readback stays open.
Next source task: retained-target read-only deletion preconditions and explicit
scope for later destructive delete/recreate workflows from the roadmap.
