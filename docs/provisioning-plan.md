# Explicit template mappings and read-only provisioning checks

This slice prepares a reviewable plan. It does not clone, configure, start,
reserve or delete anything in Proxmox. It does not save a definition, enqueue a
job or persist a plan through these preview endpoints. A plan is a deterministic description of intended inputs,
not authorization to execute them.

## Configure the catalog

Copy `examples/templates.yaml` to your own configuration file and replace every
example value with metadata from a known test template. These example values
have not been verified against a real host. Set its path explicitly:

```sh
VMANYWARE_TEMPLATES=/absolute/path/to/templates.yaml make server
```

No catalog is loaded by default. Runtime credentials remain in their existing
Proxmox variables; the catalog has no credential fields. It is read at startup
and requires a server restart after edits. Unreadable, invalid, non-UTF-8 or
oversized files refuse startup with a redacted configuration error.

The file is UTF-8 YAML, at most 64 KiB:

```yaml
apiVersion: vmanyware.dev/v1
kind: TemplateCatalog
images:
  - image: debian-13
    node: pve-test
    templateVmid: 9000
    templateVersion: debian-13-cloud-20261016
    storage: local-lvm
    bridge: vmbr0
    diskDevice: scsi0
    sourceDiskGiB: 8
    guestUser: debian
```

Every listed field is required. Unknown fields, duplicate object fields and
duplicate image identifiers are refused. Up to 128 mappings are supported. An
explicitly empty catalog is allowed but cannot resolve any images.

| Field | Meaning and limits |
| --- | --- |
| `image` | Must match the definition's image identifier exactly; 1–64 lowercase letters/digits/dots/hyphens |
| `node` | Source and target node for this initial same-node full-clone plan; 1–63 alphanumeric/dot/underscore/hyphen characters, starting alphanumeric |
| `templateVmid` | Explicit source template VMID, 100–999999999 |
| `templateVersion` | Operator-recorded template revision label, 1–128 alphanumeric/dot/underscore/plus/hyphen characters |
| `storage` | Target storage identifier, 1–64 identifier characters |
| `bridge` | Target network bridge, 1–15 identifier characters |
| `diskDevice` | Source boot disk to expand: `scsi0`, `virtio0` or `sata0` |
| `sourceDiskGiB` | Recorded template disk size, integer 4–4096 GiB; prepare/resize the template explicitly when needed |
| `guestUser` | Cloud-Init guest username, 1–32 lowercase letters/digits/underscore/hyphen, starting lowercase or underscore |

Storage/bridge identifiers start with an alphanumeric character and contain only
alphanumeric/dot/underscore/hyphen characters. These checks validate inputs, not
the existence or capabilities of a storage backend, bridge or template. Network
mode is explicitly IPv4 DHCP for this first plan; static addresses/IPv6 and
cross-node clones are not implemented.

`templateVersion` is a required operator declaration, not an automatically
verified image digest. Actual template metadata, trusted TLS, ACLs and storage
support must be checked against the selected Proxmox host before execution.

## Opt-in image-default Cloud-Init user

An operator may add this optional declaration to a mapping only after inspecting
that exact template revision's installed Cloud-Init default-user configuration:

```yaml
    imageDefaultUser:
      name: debian
      templateVersion: debian-13-cloud-20261016
      passwordLocked: true
      noAuthorizedKeys: true
```

The name must equal guestUser and must not be root. The version must equal
that mapping's templateVersion. Both booleans must be true: the image default
account has a locked password and no inherited authorized keys. This is an
operator declaration, not observed image evidence; changing the template requires
reviewing it again. Unknown/missing fields and mismatches are rejected.

The declaration changes the plan hash and is retained with its source mapping.
Absent declarations preserve existing serialization/hash inputs and explicit
ciuser behavior. Opted-in settings remove ciuser together with inherited
cicustom/cipassword; exact observation requires ciuser to be absent. Generated
user-data must omit user entirely and still contain users/default and the exact
requested public keys. Null, scalar and named user entries are rejected in this
mode. Omission without attestation is rejected. Named-user/scalar classification
for existing mappings remains unchanged.

This avoids the known scalar-user deprecation without silently choosing a guest
account from its distribution name. Generated previews do not prove attached
seed contents, installed image defaults or successful first boot. Strict readiness
and real-template testing remain mandatory; the production create worker/API
is not yet wired. See [ADR 0037](decisions/0037-attested-image-default-user.md).

## API

`GET /api/v1/images` returns the configured catalog. Without one, HTTP 503 with
`template_catalog_not_configured` is returned.

`POST /api/v1/provisioning/plan` requires JSON:

```json
{"yaml":"<current environment YAML>","targetVmid":201}
```

The YAML is validated with the same strict v1 schema as saved definitions.
The target VMID must be in the Proxmox range and differ from the selected source
template. Disk size must be at least the recorded source disk size; shrinking
is refused. No Proxmox connection is required or attempted. Browser origins are
refused; the native application posts JSON through Rust.

Successful HTTP 200 responses include:

- Definition name and canonical SHA-256 hash.
- Provider name, complete source mapping and target node/VMID/name.
- Target CPU/memory/disk/storage/bridge, guest username and IPv4 DHCP mode.
- Public SSH key count, declared packages and snapshot-after-provisioning intent.
- Ordered conceptual steps: full clone, resources, optional disk expansion,
  Cloud-Init, start, guest readiness, optional packages and optional snapshot.
- `liveVerified: false`, `vmidReserved: false`. `creationEnabled` reflects explicit runtime opt-in, a configured provider and supported mapping/features; it is not approval or live verification. Configured previews include providerScopeId for confirmation.
- `planHash`: SHA-256 of the compact JSON tuple
  `[definitionHash, selectedImageMapping, targetVmid]`, using the mapping struct's
  field order. YAML comments/formatting do not affect it; declared input changes do.

Public key contents are not repeated in the plan; their configuration is bound
by `definitionHash`. Steps describe future behavior: package installation,
snapshots and guest readiness are not implemented yet. The clone stage has a locally tested executor, without a production enqueue caller.

HTTP 422 errors use fixed safe codes: `invalid_definition`, `image_not_mapped`,
`invalid_target_vmid`, `disk_shrink_unsupported` or `invalid_plan`. Unknown JSON
fields/types and unsupported content types are rejected without echoing values.
Submitted YAML is limited to 64 KiB independently of its JSON encoding. The
request envelope is bounded to six times that size plus 1024 bytes, allowing
escaped UTF-8/control characters without relaxing the YAML limit.

## Native application

Edit the current YAML, enter a candidate **Requested VMID**, and click **Preview
provisioning**. Saving the definition is optional. The result shows the source
template, target, resources, storage/network and requested software/access.
Editing the YAML, service port or target VMID hides the previous plan, including
responses that arrive after those inputs change. No create/approve button exists.

The preview does not prove VMID availability, reserve resources or confirm the
template/bridge/storage exists. Real execution will require fresh provider
checks and explicit approval of the concrete disposable target. A hash alone
cannot replace those checks or approve a VM mutation.


## Read-only provider preflight

After an offline preview, **Check against Proxmox** submits the same current YAML
and requested VMID to `POST /api/v1/provisioning/preflight`. The server recomputes
the plan from its current catalog, then makes bounded, verified-TLS GET requests.
Both a catalog and an explicitly configured provider are required. The operation
does not save a definition or create an operation record.

The response contains `planHash`, a `checks` array of `{code, status, message}`,
a runtime `creationEnabled` capability and `vmidReserved: false`. Each status is `passed`,
`failed` or `unknown`. An unavailable endpoint, missing permission or insufficient
metadata must not become a successful check. Messages are fixed application text;
raw provider configuration and responses are not forwarded to the client.

Checks inspect source template configuration, the declared disk and Cloud-Init
drive, target storage and reported capacity, network bridge and requested VMID.
The dedicated `GET /cluster/nextid?vmid=<candidate>` check is used for VMID
availability. Missing entries in a permission-filtered VM inventory cannot prove
that a candidate is free. A successful check observes availability at that moment;
it never reserves the ID, storage or network resources.

Some requirements cannot be proven through these reads: the operator's template
revision label, working DHCP, guest readiness and permission to perform the full
clone/configuration workflow. They remain explicit unknowns. Storage observations
also cannot guarantee that a later full clone succeeds or that snapshots are
supported. These checks are preparation for a real disposable-host test, not a
substitute for it.

The native view binds results to the exact YAML, service port and requested VMID.
Input edits hide old results. A catalog change on the server may produce a new
plan hash; the client must not present that result as verification of an older
preview. No create or approval action is enabled, even if all observable checks
pass. Future execution must perform fresh checks and obtain approval of the
concrete disposable target and operations.


## Earlier implementation milestones

The following boundaries record intermediate evidence. The production integration
is now described in [the confirmed create API](#confirmed-minimum-create-api);
old statements about missing callers are historical.

### Fixed clone transport preparation

The plan can now construct validated full-clone transport inputs for a generated
instance ID. Local HTTPS tests cover fixed same-node/full-copy form fields, exact
instance/definition/plan description, source-bound qmclone task validation and
independent stopped-target identity observation. This does not change these
preview/preflight endpoints: no provisioning enqueue route or worker calls the
clone transport yet. Durable schema-6 intent/stage storage and a standalone clone
executor are locally tested; no production worker or HTTP enqueue caller is wired. See
[ADR 0024](decisions/0024-provisioning-clone-boundary.md).


## Durable intent storage boundary

Schema 6 adds vm_provision operations, atomic local target association/reservation,
canonical definition/catalog/plan snapshots and ordered stage rows. Full-plan
confirmation and global idempotency are checked in storage. Clone dispatch/task
guards, completion evidence and interrupted/uncertain reconciliation are tested
locally. Clone and CPU/memory success complete only their own stages, leaving the
operation incomplete; disk also has a standalone locally tested executor, while
Cloud-Init and readiness have no executor yet. Existing
preview/preflight endpoints remain read-only and creationEnabled=false. See
[ADR 0025](decisions/0025-durable-provisioning-journal.md).


## CPU/memory transport boundary

A fixed stopped-clone CPU/memory PUT and independent applied-value GET observation
now pass local HTTPS tests. They require exact clone identity, configuration
revision, stopped power and no pending changes. A standalone durable resource-stage caller is now tested with schema-7 revision
retention and GET-only restart reconciliation. A fixed disk-expansion transport/observer is also locally tested; standalone durable disk execution is also locally tested. Network and Cloud-Init
configuration remain pending. See
[ADR 0026](decisions/0026-stopped-clone-resource-configuration.md) and
[ADR 0027](decisions/0027-durable-resource-stage.md). This does not
change public creation capability or establish a completed provisioning flow.


## Primary disk expansion boundary

The plan constructs fixed expansion inputs for its mapped primary device, exact
observed volume and current revision. Local HTTPS tests cover absolute size,
source-size gates, target-bound resize tasks and independent exact configured-size
observation. Schema-8 disk journal/dispatch/task/outcome and restart guards are locally tested;
a standalone disk executor now passes local HTTPS tests, including GET-only
known-task restart and unknown submission exclusion. Provider
configuration size is not guest filesystem or readiness evidence. See
[ADR 0028](decisions/0028-fixed-disk-expansion.md).

See [ADR 0029](decisions/0029-durable-disk-stage.md) for the durable disk boundary.


## Network and Cloud-Init settings boundary

A fixed single-virtio-NIC/IPv4-DHCP/NoCloud/passwordless-user/public-key transport
and exact settings observation pass local HTTPS tests. Definition/plan bindings,
retained volume/revision/network, current disk/resources and target-specific
Cloud-Init drive are required. Schema-9 durable stage/dispatch/outcome/recovery storage is locally tested; the
standalone executor and GET-only recovery now pass local HTTPS tests, while the
known scalar-user first-boot compatibility issue remains open. Settings observation
explicitly reports guest readiness false. See
[ADR 0030](decisions/0030-fixed-cloud-init-settings.md).

See [ADR 0031](decisions/0031-durable-cloud-init-stage.md) for retained Cloud-Init dispatch inputs and stage evidence.


## Generated seed preview boundary

A fixed read-only user/network dump preflight now validates the supported generated
shape and stable target revision. It rejects unexpected data and reports the known
scalar user as incompatible. Hash-only reports still mark guest readiness false.
This does not inspect an attached ISO or close seed compatibility/start/readiness.
See [ADR 0032](decisions/0032-generated-seed-preflight.md).

## Seed regeneration and power-start execution boundary

The standalone seed executor now retains its expected/completed reports in the
schema-10 journal. The standalone start executor requires that proof and repeats
stopped-target/settings/seed checks before a separately recorded power submission.
Known target-bound start tasks reconcile with GET only; lost replies never permit
another POST. Task success plus running power completes start_vm only, explicitly
not guest readiness or the whole create workflow. These stages now run through the opt-in production worker. Remote controls and
real infrastructure validation remain pending. See [ADR 0039](decisions/0039-durable-seed-regeneration.md)
and [ADR 0040](decisions/0040-durable-provisioning-start.md).

The standalone readiness executor now persists a five-minute deadline, probe
marker and PID in the existing journal. Known PIDs resume via GET; unknown
submissions are not repeated. Completed pending probes may be refreshed without
extending the deadline. A minimal workflow finishes only after clean Cloud-Init
and usable MAC-bound IPv4 evidence; later declared stages keep the operation
running. This does not prove SSH authentication/reachability. See
[ADR 0042](decisions/0042-durable-guest-readiness.md). Production enqueue/worker are now integrated; remote creation controls remain pending.


## Confirmed minimum create API

Set VMANYWARE_PROVISIONING_ENABLED=true explicitly to permit new creation writes.
It is independent of VMANYWARE_LIFECYCLE_ENABLED and defaults false. Only exact
true/false values are accepted; invalid configuration fails before database/listen
startup. The listener remains loopback; remote TLS/authentication setup is separate.
Preview/preflight remain read-only. Previews expose the configured providerScopeId;
creationEnabled indicates capability, not successful preconditions or permission.

Save the exact definition first, then POST /api/v1/provisioning/create with one
Idempotency-Key and the complete confirmation:

```json
{
  "definitionName": "kernel-lab",
  "definitionHash": "<64 lowercase hexadecimal characters>",
  "providerScopeId": "<scope from the current preview>",
  "planHash": "<hash from the current preview>",
  "targetNode": "pve-test",
  "targetVmid": 202,
  "sourceVmid": 9000,
  "storage": "local-lvm",
  "bridge": "vmbr0",
  "confirmed": true
}
```

This shape example is not approval of real infrastructure. Hashes bind the exact
resources, public keys, declared packages and final snapshot shown in the definition/plan.
Creation requires an explicit validated imageDefaultUser declaration. Bounded Debian
package declarations (optional exact versions) use the fixed guest-agent installer;
invalid/removal/shell/release selectors return HTTP 422. snapshotAfterProvisioning
requires VMANYWARE_SNAPSHOTS_ENABLED=true in addition to provisioning; otherwise
creationEnabled=false and enqueue returns HTTP 403 snapshot_disabled. Declared
stages must all succeed; they are never silently omitted. See ADRs 0079–0081 for
PID/task recovery and actual-guest validation limits. The native form now offers explicit package/final-snapshot inputs and exact reviews;
React/browser, native protocol and joined Linux native API/worker/provider checks
now pass. Refreshed-artifact, Android and actual guest acceptance remain open. Browser Origin and duplicate request keys are
rejected. Missing/invalid confirmation returns 400, changed plan/scope or conflicting
work 409, absent definition 404, disabled capability 403, missing provider/catalog
or unavailable storage 503. Unknown JSON fields return 422; body limit is 4096 bytes.

The handler makes no provider request. It commits the association, immutable
snapshots and stage journal before HTTP 202 with an operation Location. Repeating
the same key and confirmation returns that operation without new work, provided
current runtime/configuration gates still permit enqueue. Changed inputs cannot
reuse a key. GET /api/v1/operations/{id} retrieves retained outcomes independently
of the write capability.

Each sequential worker round handles one queued read, one lifecycle claim and at
most one provisioning stage, prioritizing later provisioning stages before new
clones. It executes clone, resources, optional mapped disk expansion, Cloud-Init,
seed regeneration, power start and strict bounded guest checks. New writes recheck
capability/catalog/scope; retained known tasks reconcile without another mutation.
Readiness probes remain fixed read-only commands and can continue after disabling
VM writes. Unknown mutation submissions never replay. Failures retain the target
and visible operation; there is no automatic deletion, force-stop or blind retry.

Local evidence covers real Axum enqueue plus the production worker with generated
HTTPS provider fixtures, with and without disk expansion, restart after clone,
read-queue progress, terminal guest evidence and no duplicate mutation on enqueue
retry. This does not prove an actual Proxmox/guest, SSH or native/mobile runtime.
The remote form still needs native create commands and visible confirmation UI.

### Continue from readiness to SSH

For a succeeded, guest-ready remote creation, choose **Prepare SSH connection**.
The app displays its recorded IPv4 address and the account from the retained
creation plan, then opens the normal SSH host-inspection screen at port 22.
Independently compare the host fingerprint before trusting an unknown key; a
fresh inspection, explicit identity consent/unlock and terminal action are still
required. Guest readiness never grants SSH trust automatically.

The same action on a recovered successful operation uses its recorded address.
Enter the guest account yourself after host inspection because recovery does not
retain a verified username. Historical readiness is not current reachability.
Queued/failed operations have no readiness handoff. Selecting a target again or
disconnecting clears the previous SSH flow; no account, credential or automatic
connection is restored from browser storage. ADR 0051 records verification limits.


The initial resource PUT now assigns the new clone's sole VMAnyware instance tag
and verifies it with CPU/memory. This replaces inherited template tags on that fresh
clone and enables subsequent strict lifecycle/snapshot checks; it does not repair
existing VMs or replay an uncertain resource write. Real ACL/target validation is
still required. API/worker tests are not evidence of actual guest apt execution.


### Recorded stage progress

Authenticated `GET /api/v1/operations/{id}/provisioning` reads retained progress
without a provider connection or enabled writes. It returns operation/instance/source
identity, provider scope, node/VMID, definition/plan hashes, recorded status/failure
and ordered stages (`position`, `kind`, `status`, `submissionStarted`). It returns
neither private journal nor guest output. A started marker records an attempted
stage dispatch, including read-only probes; it does not establish completion,
power or readiness. Missing/wrong-kind IDs yield 404; corrupt storage yields a
fixed 503. No repair or automatic retry occurs. Native/client creation, recreation
and saved recovery offer explicit **Read recorded steps** controls with historical
wording, exact known plan/source/target checks and stale-response suppression.
Refreshed delivery artifacts include this workflow; Android execution and real
guest acceptance remain open. See ADRs 0085–0086.
