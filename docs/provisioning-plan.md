# Explicit template mappings and read-only provisioning checks

This slice prepares a reviewable plan. It does not clone, configure, start,
reserve or delete anything in Proxmox. It does not save a definition, enqueue a
job or persist a plan. A plan is a deterministic description of intended inputs,
not authorization to execute them.

## Configure the catalog

Copy `examples/templates.yaml` to your own configuration file and replace every
example value with metadata from a known test template. These example values
have not been verified against a real host. Set its path explicitly:

```sh
POCKETLAB_TEMPLATES=/absolute/path/to/templates.yaml make server
```

No catalog is loaded by default. Runtime credentials remain in their existing
Proxmox variables; the catalog has no credential fields. It is read at startup
and requires a server restart after edits. Unreadable, invalid, non-UTF-8 or
oversized files refuse startup with a redacted configuration error.

The file is UTF-8 YAML, at most 64 KiB:

```yaml
apiVersion: pocketlab.dev/v1
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
- `liveVerified: false`, `vmidReserved: false`, `creationEnabled: false`.
- `planHash`: SHA-256 of the compact JSON tuple
  `[definitionHash, selectedImageMapping, targetVmid]`, using the mapping struct's
  field order. YAML comments/formatting do not affect it; declared input changes do.

Public key contents are not repeated in the plan; their configuration is bound
by `definitionHash`. Steps describe future behavior: package installation,
snapshots, guest readiness and clone execution are not implemented yet.

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
`creationEnabled: false` and `vmidReserved: false`. Each status is `passed`,
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
