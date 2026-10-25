# Home Proxmox acceptance plan

Status: updated 2026-10-25; user-run template conversion, scoped token reads and
seven-check application metadata preflight proven. Real application creation pending.
No VM, network, storage or credential modification is authorized by this plan.
The complete product specification remains VMAnyware.md.

## Target and boundaries

- Endpoint: https://192.168.1.36:8006, node debian-btw, Proxmox 9.2.21.
- TLS: public cluster CA configured on the Mac; certificate includes the IP.
- Dedicated token successfully reads version and node status. Its exact effective
  ACLs and expiry still need verification; no administrator credential is needed.
- Existing VM 100 and all its volumes remain outside the test scope.
- User approved reservation of template 9000 and disposable instances 9001–9010.
  Template 9000 is now converted/stopped; latest preflight reports 9001 free.
  No clone/start operation is authorized merely by this reservation.
- Proposed storage local-lvm and bridge vmbr0 require live capability checks.
- Do not reuse the previous nested lab's credentials, approvals or image claims.

## Prerequisites before a create test

1. Confirm the disposable IDs and preservation boundary with the user; recheck
   cluster-wide availability immediately before allocation.
2. Establish DHCP on the intended test network. Current production Cloud-Init
   transport sets ipconfig0=ip=dhcp and validates a DHCP seed. The observed bridge
   10.10.10.1/24 alone supplies no DHCP evidence. Do not substitute a manual static
   clone and claim that application provisioning passed. First inspect existing
   DHCP service/listeners without dumping configuration that may contain secrets.
3. Establish guest Internet/DNS and Mac-to-guest SSH reachability. The observed
   MASQUERADE configuration is not evidence of working forwarding, firewall or
   routing. Any new DHCP service, route, relay or firewall rule needs a concrete
   separately reviewed target/change plan; preserve host management access.
4. Select a fixed official Debian 13 amd64 genericcloud build and verify its
   checksum and metadata. Prepare a separate unbooted Cloud-Init template using
   docs/debian-template.md, with the QEMU guest agent installed and enabled in
   the image and enabled in Proxmox. The production readiness executor requires
   guest-agent network/exec support; a Cloud-Init drive alone is insufficient.
5. Inspect that exact image's default-user settings before using the optional
   imageDefaultUser attestation. Verify generated seed semantics. Do not install
   a per-clone static snippet that bypasses the production configuration path.
6. Record a catalog revision, image hash, storage content/capability and resource
   limits. Start with 2 vCPUs, 2048 MiB RAM and a 12 GiB disk if image size permits.
7. Derive write ACLs from the actual fixed API calls and installed Proxmox schema.
   Verify both user and privilege-separated token grants. Scope them to the
   approved template, disposable targets, storage and required network resources.
   Read-only success does not establish the minimum provisioning role.

## Execution and acceptance evidence

Run through the actual application API/worker and client, retaining operation IDs,
artifact versions and sanitized results. Manual qm commands prove preparation only.
Each mutation stage requires explicit approval of its exact targets and operations.

| Stage | Required evidence |
| --- | --- |
| Inventory/preflight | Exact template/revision, available target, permitted storage/bridge, verified TLS and scoped token |
| Create | Clone task completed, CPU/RAM/disk and seed match saved definition, clean Cloud-Init exit 0, guest-agent IP proof, operation ready |
| SSH | Independently verified guest host key, application identity authentication, real PTY, resize, Ctrl-C and reconnect |
| Lifecycle | Graceful shutdown/start task completion and separately observed power state |
| Snapshot | Supported local-lvm configuration, named snapshot task completion and list observation; documented consistency limits |
| Restore | Write an owned guest marker, snapshot, change marker, shut down, explicitly approve data loss, restore, boot and verify original marker |
| Delete | Explicit volume/data-loss scope, completed task and independent VM/owned-volume absence; saved definition retained |
| Recreate | Fresh unused target, same retained definition, new instance identity, clean readiness and fresh SSH trust verification |
| Recovery | Restart control plane with retained operations, no duplicate provider mutations, inspect saved status; record uncertain outcomes honestly |
| Failures | Safe auth/TLS/unavailability handling; never send destructive negative tests to VM 100 or broaden ACLs to hide failures |

Preserve sanitized evidence and update roadmap rows only for the behavior actually
proven. Snapshots are not independent backups. Do not automatically destroy an
uncertain VM or retry an unconfirmed mutation. Cleanup targets and volumes require
explicit approval; retain the template unless its removal is separately approved.

## Current evidence and next action

User-run proxmox_probe reports version 9.2.21/release 9.2 and node debian-btw online.
No real home-server create, readiness, SSH, snapshot, restore, delete or recreation
has run. Next: confirm IDs, then inspect DHCP and guest routing prerequisites before
preparing the exact template/network/ACL operation proposal.

## API permission review before issuing write grants

Reviewed against the public Proxmox API schema on 2026-10-25 and current adapter
calls. The public schema is not pinned to the installed 9.2.21 server: confirm
its installed API schema before applying ACLs. These are requirements to review,
not a verified minimum role or authorization to grant them.

| Production call | Schema privilege or additional condition |
| --- | --- |
| Read VM config / snapshot list | VM.Audit on the exact VM |
| Full clone | VM.Clone on source, VM.Allocate on new target, Datastore.AllocateSpace on used storage and SDN.Use on used bridge/vnet |
| Set config | Parameter-specific config checks in addition to the route's any-config-privilege entry check; derive the role from the exact submitted CPU/memory/tags/Cloud-Init/network fields |
| Resize disk | VM.Config.Disk on target; check storage allocation requirements |
| Agent network interfaces | VM.GuestAgent.Audit or VM.GuestAgent.Unrestricted on target |
| Agent exec and exec-status | VM.GuestAgent.Unrestricted on target; required by current readiness and package execution |
| Create snapshot | VM.Snapshot on target |
| Rollback snapshot | VM.Snapshot or VM.Snapshot.Rollback on target according to this schema; retain application data-loss consent regardless |
| Destroy target | VM.Allocate on target; additional volume/storage checks still apply |
| Read own task status | Authenticated task owner; otherwise Sys.Audit on the node |
| Read cluster HA resources | Sys.Audit on / in the public schema |
| Read replication | Authenticated read filtered by VM.Audit on each VM |
| Prove volume absence after deletion | Current adapter additionally requires Datastore.Allocate on each affected storage to avoid filtered inventory being treated as absence |

Two consequential boundaries need operator review. Guest readiness requires a
privilege capable of arbitrary guest execution even though this application sends
fixed commands. Deletion preflight currently reads cluster HA resources, whose
public schema requires root-path Sys.Audit; deletion completion requires broader
storage visibility than allocation alone. Do not silently grant these privileges
on shared resources or promise a VM-only token for the complete workflow. Consider
a dedicated test storage when isolation from existing VM volumes is required.
VM 100 must never receive target mutation grants.

Source: [official API schema](https://pve.proxmox.com/pve-docs/api-viewer/apidoc.js).
Adapter evidence: server/src/proxmox/readiness.rs, deletion_management.rs,
deletion_absence.rs, clone.rs and snapshot.rs. No write ACL was created or tested.

## Concrete template preparation proposal — 2026-10-25

User approved and executed this preparation/import/conversion. Commands below
record the completed procedure; do not replay them. Verified source directory:
/var/lib/vz/vmanyware-images/debian-13-20261021-2638. User reports SHA512SUMS OK
for the official qcow2 and JSON metadata, qcow2 virtual size 3 GiB, corrupt=false.
Preserve the original image and create only a new prepared-agent.qcow2 copy.
The tool boots its own temporary libguestfs appliance; it does not boot the
prepared Debian guest. Network access is for the agent package installation.
Do not change host packages, credentials, boot configuration, VM 100 or DHCP.

Proposed commands, run sequentially on the approved host and stop on failure:

```sh
cd /var/lib/vz/vmanyware-images/debian-13-20261021-2638
# Refuse any existing prepared image, including symlinks.
test ! -e prepared-agent.qcow2 && test ! -L prepared-agent.qcow2
sha512sum --check --ignore-missing SHA512SUMS
cp --reflink=auto --sparse=always --no-clobber \
  debian-13-genericcloud-amd64-20261021-2638.qcow2 prepared-agent.qcow2
LIBGUESTFS_BACKEND=direct virt-customize --format qcow2 \
  -a prepared-agent.qcow2 --install qemu-guest-agent \
  --run-command 'systemctl enable qemu-guest-agent' \
  --run-command 'dpkg-query --show qemu-guest-agent cloud-init openssh-server' \
  --run-command 'systemctl is-enabled qemu-guest-agent'
qemu-img check prepared-agent.qcow2
sha512sum prepared-agent.qcow2
pvesh get /cluster/nextid --vmid 9000
qm create 9000 --name vmanyware-debian13-20261021 \
  --memory 2048 --cores 2 --ostype l26 --agent enabled=1 \
  --scsihw virtio-scsi-pci --net0 virtio,bridge=vmbr0
qm set 9000 --scsi0 "local-lvm:0,import-from=$(pwd)/prepared-agent.qcow2"
qm disk resize 9000 scsi0 8G
qm set 9000 --ide2 local-lvm:cloudinit
qm set 9000 --boot order=scsi0 --serial0 socket --vga serial0
qm set 9000 --citype nocloud --ciupgrade 0 --ipconfig0 ip=dhcp
qm config 9000 --current 1
qm status 9000
```

Review stopped status, exact scsi0 size, Cloud-Init drive, bridge and agent setting
before the separately gated final command `qm template 9000`. Approval may cover
conversion after that review, but never starting the source template. Do not
replay a partially successful import or destroy resources automatically.
Image-default-user attestation still requires inspection of the exact prepared
image's default-user configuration/locked-account metadata and absence of inherited
authorized keys before confirmed application creation. No SSH key or password is
added by this proposal. No test clone/start/snapshot/delete is included.

## First application create: scoped ACL proposal

Prepared after user-reported source-template conversion and image inspection.
User explicitly approved these credential/ACL changes in the subsequent reply.
User-returned effective token tables match all four grants. Applied to both
vmanyware@pve and privilege-separated token vmanyware@pve!dev-readonly, with
nonpropagating grants at these exact paths only:

| Role | Path | Privileges |
| --- | --- | --- |
| VMAnywareSource | /vms/9000 | VM.Audit VM.Clone |
| VMAnywareCreate | /vms/9001 | VM.Audit VM.Allocate VM.Config.CPU VM.Config.Memory VM.Config.Disk VM.Config.Network VM.Config.Cloudinit VM.Config.Options VM.PowerMgmt VM.GuestAgent.Unrestricted |
| VMAnywareStorage | /storage/local-lvm | Datastore.Audit Datastore.AllocateSpace |
| VMAnywareBridge | /sdn/zones/localnetwork/vmbr0 | SDN.Use |

Create new roles only; refuse collisions rather than altering existing roles.
Do not grant Administrator, root-path permissions or mutation rights on VM 100.
VM.Allocate inherently permits removal of the target; a deletion operation still
requires its own concrete approval. Guest-agent unrestricted permission inherently
permits arbitrary guest commands; application readiness submits fixed commands.
Storage allocation is a shared-storage capability, not a per-volume quota.
These are source-derived candidate grants, not live-verified minimum ACL proof.

After grants, use token-authenticated read-only preflight and effective privilege
observation. The proposed first app operation is full clone 9000 to 9001, 2 CPUs,
2048 MiB, 12 GiB scsi0 on local-lvm, DHCP vmbr0, exact test public SSH key, start
and guest readiness. This operation and the public-key identity must be selected
and approved before execution; reservation is not creation approval.
