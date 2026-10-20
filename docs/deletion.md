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
