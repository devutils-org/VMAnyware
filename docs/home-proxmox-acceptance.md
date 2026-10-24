# Home Proxmox acceptance plan

Status: prepared 2026-10-25; read-only connectivity proven by user-run probe.
No VM, network, storage or credential modification is authorized by this plan.
The complete product specification remains VMAnyware.md.

## Target and boundaries

- Endpoint: https://192.168.1.36:8006, node debian-btw, Proxmox 9.2.21.
- TLS: public cluster CA configured on the Mac; certificate includes the IP.
- Dedicated token successfully reads version and node status. Its exact effective
  ACLs and expiry still need verification; no administrator credential is needed.
- Existing VM 100 and all its volumes remain outside the test scope.
- Proposed template 9000 and disposable instances 9001–9010 require confirmation
  of availability and reservation before any concrete mutation approval.
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
