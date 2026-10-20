# ADR 0058: Review retained deletion target and API-visible volume scope first

Date: 2026-10-20. Status: accepted; read-only target/volume review, dispatch absent.

Deletion is distinct from forgetting an association or recreating from a definition.
The initial GET preflight uses only the retained instance scope/node/VMID. It keeps
the stored definition/association unchanged and performs no power or delete call.
It reviews live identity/template/lock/pending/digest, stopped power, protection,
complete snapshot work and saved ownership markers before exposing candidate scope.

Read live and saved snapshot configurations twice; normalize active/unused/EFI/
TPM/Cloud-Init and RAM-state volume references, deduplicate and retain source labels.
Require safe storage volume IDs and authoritative filtered image inventory owner
VMID, rather than infer ownership from a filename. Unknown/foreign/missing image
ownership, malformed/ambiguous references and raw host paths block the review.
Recognized ISO CD-ROM references are reported separately as preserved media.
Canonical config/snapshot/normalized-scope hashing binds the two reads; raw configs,
provider inventory extras, guest settings and host paths are never returned.

The preflight remains incomplete while additional provider cleanup effects are
not fully bound; its overall result stays unknown unless a checked condition fails.
Execution is always disabled, independently of every existing write flag. Before
any future delete approval/dispatch, review additional provider effects and cleanup scope after the management checks
described below. The API-visible candidates are not
proof of every backend side effect or guaranteed volume removal. Provider cleanup
can warn without removing a volume; task success alone must not prove deletion.
Retain definitions and journals for future recreation; no purge, skiplock or
unreferenced-disk destruction is granted here.

Bounds: 20 seconds overall, 64 snapshots, 256 candidate images, 1024 inventory
entries per referenced storage, existing 2 MiB provider response bound. Permission
failures are explicit safe errors, not empty owned-storage reports. All new tests
use generated HTTPS/API fixtures and no real VM mutations.

Primary contracts: [destroy API](https://github.com/proxmox/qemu-server/blob/master/src/PVE/API2/Qemu.pm),
[volume-owner storage listing](https://github.com/proxmox/pve-storage/blob/master/src/PVE/API2/Storage/Content.pm),
[destroy implementation and cleanup warnings](https://github.com/proxmox/qemu-server/blob/master/src/PVE/QemuServer.pm).


The next read-only slice checks cluster HA/replication membership twice around the
volume review, binds target membership into the scope hash and exposes only target
haManaged/replicationJobs. Configured HA or replication blocks deletion even when
disabled or removal-pending. Membership changes block as well. HA index requires
Sys.Audit on /; replication index filters by VM.Audit on the target. Probe effective
permissions at the fixed target path before and after each replication read and
require a valid VM.Audit key. Its value is the propagation flag (0/false is not a
denial). Never assume an empty filtered list proves absence without that visibility.
No permission mutation or user/token override is sent. Reject malformed/duplicate/
oversized lists and inconsistent replication job/guest IDs. Accept canonical
numeric or string guest IDs; retain only normalized target job IDs.

Nine targeted deletion checks pass, including five affected existing preflight
checks and four new management checks. Full positive target/volume review now uses
23 GETs. Additional provider-effect scope stays unknown and execution off; passing
management checks does not complete destructive authorization or cleanup proof.

Primary read contracts: [HA resources](https://github.com/proxmox/pve-ha-manager/blob/master/src/PVE/API2/HA/Resources.pm),
[replication config visibility](https://github.com/proxmox/pve-manager/blob/master/PVE/API2/ReplicationConfig.pm),
[effective token permissions](https://github.com/proxmox/pve-access-control/blob/master/src/PVE/API2/AccessControl.pm).


The effect-scope slice now parses recorded fleecing references from current
special sections, proves their owner VMID, reports observed unreferenced owned
images separately and classifies candidate effects. Fixed version-1 side effects
include configuration/snapshot metadata, VM ACL/firewall, IPAM and recorded
fleecing cleanup. Purge jobs and bulk unreferenced-disk destruction stay false.
Unknown special sections, malformed/foreign fleecing and snapshot-only disk
references with no guaranteed destroy path block review. Non-Cloud-Init CD-ROM
media is preserved. Observed preservation is confined to inspected inventories;
it does not claim global orphan discovery. Canonical scope v2 plus the fixed
effect contract binds this evidence. Fully passing checks now yield passed read
preconditions, while execution remains disabled and consent/journal absent.

The earlier positive fixture's snapshot-only old disk is now an explicit refusal
case; the supported positive snapshot references the same disk as live config.
This corrects the candidate classification against upstream destroy semantics,
rather than claiming that every saved-config disk will be removed. Ten targeted
checks pass, including aggregate candidate/preservation bounds. No real mutation
or new provider transport occurs.

Primary effect contracts: [current config retains special sections](https://github.com/proxmox/pve-guest-common/blob/master/src/PVE/AbstractConfig.pm),
[recorded fleecing cleanup](https://github.com/proxmox/qemu-server/blob/master/src/PVE/QemuConfig.pm),
[CD-ROM and Cloud-Init classification](https://github.com/proxmox/qemu-server/blob/master/src/PVE/QemuServer/Drive.pm).
