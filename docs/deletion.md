# Deletion and recreation preparation

`GET /api/v1/environments/{id}/delete-preflight` reviews only the saved target.
Query parameters cannot retarget or enable purge. It creates no approval/operation,
changes no association/definition and performs GETs only. Product deletion dispatch
remains disabled; standalone executor evidence is described below. Recreation
remains unimplemented.

Report: retained instance/scope/definition/node/VMID, current config digest,
canonical scopeHash, final power observation, candidate volumes with source
references/ownerVerified/effect, snapshot names, preserved media, observed preserved
owned images, fixed versioned effects, target haManaged/replicationJobs and checks,
destructive=true, definitionRetained=true, purge=false,
destroyUnreferencedDisks=false and executionEnabled=false.

Review covers identity/template/locks/pending changes, already-stopped power,
protection, live/unused/Cloud-Init/EFI/TPM disks and saved snapshot/RAM-state
references. Two scope reads must match; image ownership comes from filtered storage
inventory VMID, never filenames. Private config/inventory fields and raw host paths
are not returned. Missing/foreign ownership, unsafe/ambiguous references, incomplete
snapshot work or changing target/scope produce failed checks. Storage permission
failure cannot become an empty success.

HA and replication membership is now read twice around the volume review. The
selected VM must have no configured HA resource (including disabled) or replication
job (including disabled/removal-pending), and membership must stay stable. Effective
VM.Audit visibility is checked before and after each replication list, because the
upstream list filters results by that privilege. The returned privilege value is
its propagation flag; an existing VM.Audit entry with 0/false still grants audit.
The HA list itself requires Sys.Audit on /. These are read requirements, not
permission changes made by VMAnyware. Refused/revoked/malformed visibility is an
explicit provider-read error, never an empty management success. Only target
membership/IDs are returned; unrelated resources and job settings stay private.

When every target, volume, management and effect-scope check passes,
deletionPreconditions can be passed. This is read evidence, not destructive consent. No existing lifecycle/provision/snapshot/
restore flag enables deletion. The API-visible inventory is not authorization or proof of completed cleanup.
Next: explicit approval and durable no-replay journal before delete transport. Definitions
and operation history must survive deletion so recreation can use the saved inputs.

Limits: 20-second deadline, at most 64 snapshots, 256 combined candidate/preserved owned images and 1024
entries per referenced storage. Unknown instance: 404; changed provider scope: 409;
provider absent: 503; provider/invalid response: 502 delete_preflight_unavailable,
timeout: 504 with that same code. Errors omit raw provider details.

Nine targeted generated-HTTPS/API checks pass: live/detached/Cloud-Init/snapshot/RAM
candidate scope, separate ISO, stable fingerprint, no writes/private config, fourteen
blocked cases, duplicate inventory, scope guard, retained target/definition and
independent disabled execution, plus missing/provider/scope/permission/malformed
errors. Additional coverage includes exact management GET paths, nonpropagated
audit visibility, permission revocation, target dependencies appearing during
review, disabled/removal-pending jobs, malformed/duplicate/bounded lists and
private unrelated cluster data. Positive full review now uses 23 GETs. These prove read-only source behavior, not deletion permission, disk removal,
actual storage/backend effects, recreation or real infrastructure acceptance.
See [ADR 0058](decisions/0058-deletion-target-and-volume-review.md).


## Fixed provider effects and cleanup scope

The report now includes version-1 fixed `effects`: remove VM configuration/snapshot
metadata, VM permissions/firewall configuration, cleanup of VM IPAM references and
recorded fleecing images; keep purgeJobs=false and destroyUnreferencedDisks=false,
with definition/history retention required. These are the expected upstream DELETE
side effects to acknowledge later, not permission to run them now. No delete
transport, operation approval or runtime enable flag exists yet.

API-visible `special-sections.fleecing.fleecing-images` volume references join the
candidate scope and need the same authoritative owner VMID check. A malformed
section/list, unknown special section or foreign/unknown-owner fleecing image blocks
eligibility. The provider's cleanup itself does not guarantee an owner check;
VMAnyware requires one before any future dispatch. Cloud-Init special settings are
hashed privately and never exposed. Current config retains special sections in the
reviewed upstream read contract; real-host version support remains unverified.

Per-volume effects distinguish delete_owned_volume, fleecing_cleanup and
snapshot_reference. A disk referenced only by an old snapshot without a live/unused
or RAM-state deletion path is reported but blocks this fixed operation: removal
is not promised by the upstream destroy loop. Supported live/snapshot-shared
volumes and explicit snapshot RAM artifacts remain reviewable. This does not
silently widen deletion to all matching-VMID disks.

Non-Cloud-Init CD-ROM references are preserved, even when their backing image is
owned by the VM; Cloud-Init recognition describes drive kind, not ownership. Extra
owned images observed in the inspected storage inventories are reported as
preservedOwnedVolumes and are not a cluster-wide orphan inventory. Unreferenced
images on other storages are not scanned or granted for deletion. Fixed bulk
cleanup remains false. Canonical scope v2 hashes candidate effects, observed
preservation set, special config and fixed effect contract into the approval token.
Changes across either scope read block eligibility. Candidate plus observed owned
preservation entries are bounded to 256; inventories retain their existing bounds.

Ten targeted checks pass in this effect-scope slice: six affected preflight checks
plus four new groups covering valid/foreign/unsafe/changing fleecing, unknown
sections, snapshot-only disk refusal, non-Cloud-Init media preservation, unsupported
device slots and aggregate bounds. Public responses still omit raw private config
and host paths, preserve association/definition and keep execution disabled.
Actual storage deletion, backend warning handling, post-delete absence and retained
recreation inputs remain future acceptance work. Task success must not replace
independent cleanup evidence.

## Durable deletion approval (storage only)

The local journal now supports vm_delete intents. Approval binds `instanceId`,
`providerScopeId`, `definitionHash`, `node`, `vmid`, `configDigest`, a full `plan`,
`confirmed: true` and `acknowledgeDataLoss: true`. Plan contains `scopeHash`,
`volumes`, `snapshots`, `preservedMedia`, `preservedOwnedVolumes` and fixed `effects`.
It retains all normalized scope information needed for future reconciliation.
The caller cannot widen effects to purge/bulk destruction or drop retention.
Unknown fields, invalid references/unverified owners/unsupported effects, duplicate
or overlapping scope entries and approval JSON above 64 KiB fail validation.

Schema 13 shares active mutation exclusion with provisioning, lifecycle, snapshot
creation and restore. Failed provisioning remains visible but permits explicit
cleanup preparation; active/uncertain provisioning still excludes deletion.
Idempotent same-key inputs preserve the original confirmation age. Dispatch needs
an exact approval at most 120 seconds old and a durable marker written once. Only
a matching qmdestroy task is retained. Restart never reclaims an unknown submission;
known-task claims are for reconciliation only, with no second dispatch.

A failed provider destroy task remains uncertain because partial cleanup may have
occurred. Mutation exclusion stays held. No success/retirement method exists yet;
it must be added together with independent VM/volume absence evidence. Definition,
association and history remain unchanged by this journal. There is no deletion HTTP
submission, runtime worker flag, DELETE transport or client action connected yet.
Eight new storage tests pass, plus 67 affected migration/storage/preflight checks.
See [ADR 0059](decisions/0059-durable-deletion-approval.md). Next: independent absence
observation contract and atomic retained association retirement, then the fixed
opt-in executor and HTTP/client flow. No actual deletion is proven here.

## Independent absence observation

`Proxmox::deletion_absence` accepts validated retained approval for the same
provider scope. It makes GET requests only, with a 20-second overall deadline.
The explicit `/cluster/nextid?vmid=...` check must return the exact integer VMID
before and after storage reads. Errors, including 404, never establish absence.
This uses the cluster-wide allocation assertion rather than a permission-filtered
VM list or a missing per-VM configuration endpoint.

For each storage containing reviewed deletion candidates, require effective
`Datastore.Allocate` before and after its image inventory. The permission's value
is its propagation flag; a present false/0 grant is valid. `Datastore.Audit` alone
cannot establish full visibility after VM-specific ACLs disappear. No permission
is changed. Inventory deliberately has no VMID filter, so a reviewed volume with
changed ownership remains detectable. Invalid, duplicate, wrong-storage or more
than 4096 image entries fail closed; existing bounded-response/TLS rules apply.
The result binds the approval hash and lists remaining reviewed volumes.

This is point-in-time evidence, not an allocation lock. It does not prove retained
media/images survived, audit IPAM/ACL/firewall cleanup, complete an operation,
retire an association or enable deletion. Provider task status remains a separate
required observation for future execution. Empty inventories on unavailable or
unsupported backends must fail at the provider; real backend behavior remains
unverified.

Primary contracts: [cluster allocation assertion](https://raw.githubusercontent.com/proxmox/pve-manager/master/PVE/API2/Cluster.pm),
[content inventory filtering](https://raw.githubusercontent.com/proxmox/pve-storage/master/src/PVE/API2/Storage/Content.pm),
and [volume visibility rules](https://raw.githubusercontent.com/proxmox/pve-storage/master/src/PVE/Storage.pm).

## Atomic retirement

`verify_deletion_completion` additionally requires a matching `qmdestroy` task
independently observed as succeeded before absence reads. It returns an opaque,
non-deserializable proof bound to the full approval hash and exact task. Store
completion rejects mismatched or more than 30-second-old proof, missing dispatch,
missing linked task and an already retired or changed target.

Schema 14 atomically stores succeeded task/VM/volume evidence and changes the
association to `deleted`. Its YAML, definition, provider reference and history are
retained. A database failure rolls back the operation and association together.
Deleted identities refuse new operations and target resolution. Scope/VMID
uniqueness now applies to registered associations: a replacement needs a new
identity, and old identities cannot act on it. Native/local/remote lists support
the retained deleted state without exposing active VM controls. Same-key retries
retain their original operation even after retirement. See ADR 0060.

These transitions are locally implemented and tested; deletion execution remains
disconnected and no real VM has been deleted as acceptance evidence.


## Standalone deletion executor

The fixed transport sends one DELETE with query options `purge=0` and
`destroy-unreferenced-disks=0`, an empty body and no skiplock/power operation.
The executor checks retained identity, complete-absence read visibility and a
fresh full preflight matching every approved plan field/digest. Approval expiry
is not renewed. A durable dispatch marker precedes the request; a verified
qmdestroy task is retained once. Other than explicit 401/403 refusal, submission
errors stay uncertain and are never retried automatically.

Known tasks reconcile through GETs even with dispatch disabled. Provider/task/read
failures or partial cleanup keep mutation exclusion and the association intact.
After the original task succeeds and all reviewed VM/volume absence checks pass,
atomic retirement can complete; no second DELETE is issued. An unverified lost
submission has no task and cannot be reclaimed automatically.

Eight targeted generated-HTTPS tests pass, plus the affected public-preflight
fixture; library compilation passes. This is standalone library execution only:
no runtime flag, worker, HTTP submission or client deletion control is connected.
Preflight execution stays false. Real infrastructure remains untested.

The provider DELETE API does not offer digest comparison: our consent review and
the provider mutation are separate requests. Local exclusion does not prevent
external administrator changes between them. ADR 0061 records this limitation,
the fixed upstream protocol and the remaining real-target review requirement.

## Runtime capability and worker

`VMANYWARE_DELETIONS_ENABLED` is independently default-off. Only exact lowercase
`true`/`false` are accepted; invalid or non-Unicode configuration stops startup
before opening the database, without logging its supplied value. Other write
flags never enable deletion. Existing router/runner constructors default this
new capability to false.

The production worker now services one deletion claim after its existing read,
lifecycle, provisioning, snapshot and restore queues, sequentially. Known tasks
can reconcile through GETs while the flag is false. Unknown submissions without a
verified task remain excluded from automatic claims. The preflight response's
`executionEnabled` reports the runtime flag; it does not replace read prerequisites,
full consent, authorization or executor revalidation. HTTP deletion submission is
not connected yet.

Actual isolated server tests verify strict startup/default-off behavior, queue
fairness, flag propagation to the authenticated GET preflight, one generated-
provider DELETE and independent retirement. Another process is killed while its
verified task is running, then a new server on the same database completes through
seven provider GETs with deletion disabled. The saved task is never resubmitted.
These are generated-provider process tests, not real VM cleanup acceptance.

## Confirmed HTTP submission

`POST /api/v1/environments/{id}/delete` now accepts the full reviewed plan/target/
digest with both explicit confirmation and data-loss acknowledgement. It requires
independent deletion capability, matching saved provider scope and exactly one
idempotency key; the 64 KiB JSON route never reads or mutates the provider.
Unknown/unsafe scope/effects are rejected. Same-key/full-input retries keep the
original operation and expiry, including after successful retirement. Changed
inputs, active mutations or attempts to use a retired identity for a new operation
conflict. `202` exposes the retained operation through its provider-independent GET
location. POST retries still require current capability/provider; known operation
reads do not. See the API contract for status codes and the exact plan projection.

Five focused HTTP/SQLite tests pass using an unreachable provider, proving enqueue
needs no provider access. One actual authenticated server/generated-provider test
passes API -> journal -> worker -> exactly one DELETE -> independent retirement,
same-key retries before/after completion, unchanged expiry and GET result recovery.
Its provider traffic is the executor's exact 30-request sequence for a disk-only VM
without snapshots; HTTP enqueue adds no reads. No native deletion commands, UI,
scoped deletion history or real infrastructure acceptance are claimed yet.

## Retained native commands

The native bridge now registers `prepare_remote_deletion`,
`confirm_remote_deletion`, `read_remote_deletion_operation` and
`clear_remote_deletion`. Preparation takes only connection ID, request key and
retained instance ID. It reads registered target/preflight through fixed GETs and
retains their complete validated public scope in native memory. IPC confirmation
supplies two true consent flags and the same key, never a replacement plan/target.
Initial review expires after 120 monotonic seconds.

After any unverified submission reply, retain that same key/body; preparing another
intent or clearing unknown/nonterminal work is refused. A received operation ID
uses GET only. Partial destroy failures remain uncertain until verified completion,
and result success must match the complete approval hash and absence/retirement/
definition evidence. Disconnect discards native memory while server work may
continue; it cannot cancel or resubmit it. See ADR 0063 for the shared hash example
and native/server compatibility checks.

Fourteen focused native tests, the shared server hash test and five affected
workspace/restore tests pass; Linux native compilation/linking is proven. The
client screen, scoped saved-operation recovery, actual GUI/native deletion journey,
fresh mobile runtime and real infrastructure acceptance remain open.

## Client confirmation and status

Registered remote rows now offer explicit deletion selection. A separate review
button calls native preparation; selecting a row alone does not read or submit
consent. The screen displays the node/VMID, owned candidate references, recorded
fleecing, removed snapshot metadata, preserved media/other observed images, fixed
provider effects, retained definition/history and prerequisite results. Independent
target/scope and permanent-loss checkboxes start unchecked; disabled/failed review
cannot submit. No implicit stop/start, purge or extra-volume option is offered.

A submitted unknown response retains its key and frozen consent; only an explicit
identical retry is offered. Known operation IDs expose refresh through reads,
with partial cleanup kept uncertain and no new submit/clear. Native code validates
the complete approval hash; the screen also checks target/task/absence/retirement
flags before showing success. Verified retirement removes active controls for the
old row. Acknowledgement clears local intent without reviving that deleted identity.

Shared mutation selectors stay locked through review/submission, while disconnect
still clears client consent without cancelling server work. Generation/component
keys suppress late review/status replies on a new selection/connection. Consent
and request keys are not restored automatically after reconnecting.

Frontend build and the dedicated Linux Firefox simulated-IPC deletion scenario
pass. Affected snapshot creation and restoration scenarios pass too; the restore
scenario needed a sequential rerun after a parallel-run timeout of unconfirmed
cause. This proves React interaction, not a launched native deletion journey or
real VM cleanup. Scoped saved deletion history/recovery remains the next task.

## Saved deletion history and detail

Instance-scoped GET `deletion-operations` and `deletion-operations/{operationId}`
now read the durable journal without provider access or write capability. Deleted
associations retain this history even if a new instance reuses their VMID. Detail
returns saved target/digest/full plan/hash and submission state, without consent
flags, request key or raw UPID. It never prepares replacement approval. History
contains at most 100 compact summaries; older saved IDs remain readable.

Readers validate full approval/task/target/status/result consistency and require
actual association retirement for successful records. Active work stays active;
partial/unknown work remains uncertain and old terminal failures remain visible.
Corrupt or oversized evidence fails closed before exposing private fields, with
scope filtering before journal decoding. ADR 0064 records the bounds and contract.

Eight focused HTTP/SQLite tests and the extended actual authenticated server
scenario pass. The latter completes deletion against a generated provider,
restarts with provider removed/all writes off, and reads unchanged succeeded
history/detail without another provider request. Native/client saved-operation
recovery and actual native restart acceptance remain next; real cleanup is unproven.

## Native saved-operation recovery

The bridge registers `read_remote_deletion_history(connectionId, instanceId)` and
`recover_remote_deletion(connectionId, instanceId, operationId)`. Both use fixed
GET paths, the current HTTPS/authenticated connection and canonical IDs; no
preparation, submit, clear, provider request or write capability is needed.

The native reader independently validates saved target/digest/complete plan and
its full approval hash, then exact operation/result/dispatch consistency. History
is bounded to 100 unique IDs with safe partial/failed/interrupted states. Known
failed destroy tasks stay uncertain. Disconnect/reconnect rejects old replies;
unknown response fields are not forwarded and server errors do not become mutation
retry instructions. No saved record installs a prepared intent or preselected
consent. The internal hash reconstruction is verification only.

Eight focused native recovery tests and 14 affected retained-submission/status
tests pass. Native Linux compilation/linking is proven. The client panel and actual native
restart journey are now verified below; fresh mobile runtime and real VM cleanup
remain open.

## Client saved-record panel

Every retained remote row now exposes saved-deletion history and manual operation-ID
lookup, including rows marked deleted. Loads, row inspection and refresh call only
the dedicated native GET readers; no live review, consent or request key is restored.
The panel displays recorded submission/task status, reviewed volumes/references,
fixed planned cleanup effects, snapshot metadata and media/images excluded from
deletion. Result evidence is explicitly historical: it does not observe current
VM/storage state, particularly when a VMID has been reused.

Success presentation additionally binds target and result approval hash, task,
absence, retention and retirement flags. Partial/unknown cleanup remains uncertain
with refresh-only controls; historical failed attempts remain visible. Editing an
ID or disconnecting invalidates pending replies, and a new connection starts with
no restored saved ID/result/consent. Errors remain distinct from empty history.

Production frontend build and the focused Firefox simulated-IPC saved-deletion
scenario pass. Affected deletion submission and saved-restore scenarios pass too,
run sequentially. This is React interaction evidence; actual native application/server restart
evidence follows. Real infrastructure deletion acceptance remains open.


## Actual Linux native journey

The opt-in `experiments/lifecycle-native/check.py --deletion-recovery` scenario
passes through real React/WebKit, production native IPC/HTTPS commands, an
actual authenticated Axum worker and generated HTTPS provider. It checks a
running-target refusal, cancelled review, two independent consent flags, a lost
accepted HTTP response and exact same-key/body retry with one provider DELETE.
Verified task/VM/volume-absence results retire the association while retaining
its complete definition. New app/server processes then recover history, saved ID
and refresh with provider configuration removed and all five write flags false.
Recovery is GET-only, provider counters and the complete operation are unchanged.

This proves the isolated Linux fixture, not real Proxmox cleanup, normal packaged
startup, fresh mobile behavior or live-provider crash recovery. Reproduction and
log locations are in the native fixture README.
