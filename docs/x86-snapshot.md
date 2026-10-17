# Disposable clone 202 snapshot/restore plan

Status: prepared 2026-10-18; read-only preflight passed. No snapshot, rollback,
guest restart or guest file mutation has been executed for this slice.
Execution requires explicit approval of the concrete operations below.

Naming note: lab resource identifiers here use current branding. External
resources were not renamed. Resolve actual names/hostname/snippet from VMID 202
and ignored local lab records; never replace TLS names or SSH trust blindly.

## Scope and acceptance

Use only node pve-lab, VM 202 (vmanyware-cloudinit-compat-202), and snapshot
`pl202-baseline-a`. Prove stopped-disk snapshot create/list and rollback by
observing guest file contents after a fresh boot. Provider task success alone
is insufficient. This is an administrative lab experiment, not product code.

No changes to template 9000, diagnostic clone 201, reader token/ACLs, management
network, snippets, packages or SSH keys. No deletion of snapshots, VMs or disks.
No live snapshot, RAM capture, guest-agent dependency or production operation.

## Read-only observations — 2026-10-18

- 9000, 201 and 202 are stopped. 202 retains freeze=1 and references absent
  runtime vmbr1; a start requires deliberate transport restoration.
- scsi0: local-lvm:vm-202-disk-0,size=12G; ide2:
  local-lvm:vm-202-cloudinit,media=cdrom,size=4M.
- local-lvm is active/enabled, non-shared LVM-thin (VG pve, thinpool data).
  Reported available bytes: 17,977,492,007. This is a point-in-time observation,
  not a reservation or proof of thin-pool metadata capacity.
- GET /nodes/pve-lab/qemu/202/feature?feature=snapshot returns hasFeature=1.
- GET /nodes/pve-lab/qemu/202/snapshot lists only current, not a named snapshot.
- cicustom still binds user=local:snippets/vmanyware-202-user.yaml. Its bytes
  are external to the guest disk: preserve and verify the existing snippet;
  do not claim a VM snapshot backs up that external file.
- Raw sanitized observations are ignored under
  .lab/pve/clone202/snapshot-preflight.json.

LVM-thin supports efficient snapshots but shared thin-pool data/metadata capacity
must remain available. Before mutation, inspect local installed tool/API behavior
and read-only `lvs` data_percent/metadata_percent for pve/data. Require both below
80%, at least 4 GiB reported available, no pending config/lock, exact volume
identity and absent snapshot-name collision. These are conservative lab gates,
not general capacity guarantees. Refuse unsupported/unknown results.

Sources: [official LVM-thin documentation](https://github.com/proxmox/pve-docs/blob/master/pve-storage-lvmthin.adoc)
and [official QEMU API implementation](https://github.com/proxmox/qemu-server/blob/master/src/PVE/API2/Qemu.pm).
Upstream master is context; installed 9.2.2 behavior/readback is authoritative.

## Concrete operations proposed for approval

1. Recheck the observations and gates above, endpoint CA/hostname and outer
   QEMU ownership. Keep 9000/201 stopped throughout. Use the existing generated
   lab administrative login; never renew tokens or widen ACLs for this test.
2. Recreate only the prior runtime vmbr1, 192.0.2.1/30; retain 202's existing
   192.0.2.2/30 with no gateway. Recreate the owned bounded relay on outer guest
   10.0.2.15:18222 and QEMU forward bound 127.0.0.1:18222, plus a loopback-only
   temporary helper/serial receiver on 18473 if needed. Refuse port/route/link
   collisions; no host routes, NAT, DHCP or management config changes.
3. Start only 202 frozen, attach its serial recorder, then continue CPUs. Use
   its existing dedicated identity and pinned known_hosts with strict checking,
   no personal SSH agent/config. An unexpected host key stops the test; do not
   accept a replacement key merely because it followed a restart.
4. Require SSH and Cloud-Init exit 0/done with no fatal/recoverable errors.
   Exclusively create /home/debian/vmanyware-snapshot-202 (0700) and state.txt
   (0600), refusing an existing path/symlink. Write exact bytes
   `vmanyware-202-state-A\n`, fsync file and directory, read back/hash and save
   evidence locally. Gracefully shut down 202; require task OK and stopped.
5. POST /nodes/pve-lab/qemu/202/snapshot with snapname=pl202-baseline-a,
   vmstate=0 and a lab description. Poll the returned task to stopped/OK,
   with a bounded deadline and no automatic replay. List snapshots and inspect
   the named snapshot config; require expected identity/disks and no RAM state.
6. Boot using the same frozen/continue procedure and strict existing host trust.
   Verify A survived. Replace only the owned state.txt with exact bytes
   `vmanyware-202-state-B\n`; fsync/read back/hash. Exclusively create a second
   owned file after-snapshot.txt containing `vmanyware-202-after-snapshot\n`;
   fsync and verify. Gracefully shut down; require stopped before rollback.
7. POST /nodes/pve-lab/qemu/202/snapshot/pl202-baseline-a/rollback with start=0
   (verify installed API supports explicit no-start). This deliberately discards
   the post-snapshot B state and second file on this disposable VM. Require
   task OK and stopped; never force/unlock/retry a failed or ambiguous rollback.
8. Start/continue explicitly again. Require the original pinned SSH host key,
   Cloud-Init clean status and intended Debian user/resources. Read actual file
   bytes/hash: state.txt must be A, not B; after-snapshot.txt must be absent.
   Record the new boot ID to demonstrate a fresh boot. Do not reset Cloud-Init
   cache: its cached success here does not constitute another first-boot proof.
9. Gracefully stop 202. Verify 9000/201/202 stopped; terminate only owned relay/
   recorder, remove only the added forward/runtime bridge, stop temporary helper
   receiver and log out the lab root console. Preserve the stopped VM, snapshot,
   disks, A marker, snippet and existing identity. No deletion is included.

Snapshot capture and rollback can restore VM configuration as well as disks.
Compare disk/network/cicustom/freeze settings before each boot and after rollback;
do not silently fix unexpected differences. Retain the external snippet unchanged.

## Failure and evidence rules

Record provider UPIDs/status separately from snapshot list/config and guest proof.
Keep logs under ignored .lab/pve/clone202/snapshot; redact auth and never record
private keys. Distinguish planned, attempted and verified stages in STATE.md.
A timeout is an unknown result: observe the existing task, do not replay writes.
If a snapshot or rollback fails, preserve evidence and stop further disk writes;
request a concrete recovery operation rather than force cleanup. Attempt graceful
shutdown/owned transport cleanup where possible, and report any running target
or retained access accurately. Never force-stop without separate approval.

Acceptance closes only after A -> B -> restored A and absent post-snapshot file,
strict host trust, successful provider tasks/list and final stopped/cleanup proof.
This does not prove live/RAM snapshots, app consistency, backup durability,
least-privilege snapshot permissions or product lifecycle integration.
