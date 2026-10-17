# Prepared Debian template on the disposable x86 lab

Status: approved and executed on 2026-10-18; template 9000 is stopped and
visible through the scoped reader token. This is the concrete next step of [the template guide](debian-template.md),
confined to [the approved outer lab](x86-lab.md). It does not start a guest.

## Verified preparation

- Target: only `pve-lab` inside `vmanyware-pve-lab`, Proxmox 9.2.2.
- Read-only API observation: no cluster VM/container resources; requested VMID
  9000 available at observation time. This does not reserve the ID.
- `local-lvm`: enabled/active LVM-thin storage allowing images; 21,302,870,016
  bytes available at observation time (about 19.8 GiB).
- `vmbr0`: active bridge over `nic0`, outer management address 10.0.2.15/24.
- Official image downloaded on the development host under ignored `.lab/pve/images/`:
  `https://cloud.debian.org/images/cloud/trixie/20261015-2632/debian-13-genericcloud-amd64-20261015-2632.qcow2`.
- SHA-512 matches the manifest fetched over verified HTTPS from the same dated
  directory:
  `7f1d06b629fa01241ec5817735777d0554554ff8c8d9dc27f9eac65ba3c3a6f859deb6079ebc9c65795d1955abd00a77aa3ff9cb84ae6307dfbbd8b2e398dab2`.
- `qemu-img info` confirms qcow2, no backing file and 3,221,225,472-byte virtual
  capacity (3 GiB). Build metadata JSON parses. No image was booted or mounted.
  Matching HTTPS/checksum is not a detached-signature verification.
- Installed Proxmox's public `qm` manual was fetched with the explicit lab CA
  and hostname verification; reviewed import-from, Cloud-Init and template syntax.

## Approved concrete operations

| Setting | Proposed operation |
| --- | --- |
| VMID/name | Create new 9000, `vmanyware-debian-13-20261015` |
| Compute | 2 cores, 2048 MiB; VM remains stopped and never boots |
| Disk | Import the verified 3 GiB image to `local-lvm`, expand scsi0 to 8 GiB |
| Boot/console | virtio-scsi-pci, boot scsi0, serial socket console |
| Cloud-Init | Add local-lvm Cloud-Init drive, user debian, NoCloud, ciupgrade=0 |
| Network | virtio NIC attached to existing vmbr0; dormant DHCP declaration |
| Conversion | Convert only this stopped VM to a template |
| Role | New `VMAnywareTemplateReader`, containing only `VM.Audit` |
| ACLs | Grant that role only on `/vms/9000`, propagate=0, to the existing reader user and inventory token |
| Catalog | Local ignored mapping for debian-13 to pve-lab/9000 and the recorded revision |

No password/key, token regeneration/expiry change, host-network/firewall change,
VM start, clone, snapshot, restore or deletion is included. If preparation fails,
record existing VM/disks and stop; do not automatically remove or recreate them.

The role name was absent at observation time. Recheck VMID, role, storage and
bridge immediately before creation; refuse existing objects rather than update
someone else's target. Apply no ACL outside `/vms/9000` and no global role.

## Reviewed execution sequence (executed after approval)

Run commands on the owned Proxmox console, after approval. Check each result;
do not paste/replay the whole document following a partial failure. Download
into a new directory; do not overwrite an earlier preparation.

```sh
set -euo pipefail
IMAGE_BUILD=20261015-2632
IMAGE_FILE="debian-13-genericcloud-amd64-${IMAGE_BUILD}.qcow2"
IMAGE_BASE="https://cloud.debian.org/images/cloud/trixie/${IMAGE_BUILD}"
IMAGE_DIR="/var/lib/vz/vmanyware-images/debian-13-${IMAGE_BUILD}"
test ! -e "$IMAGE_DIR"
mkdir -p "$IMAGE_DIR"
cd "$IMAGE_DIR"
curl --fail --show-error --location --proto '=https' --proto-redir '=https' \
  --output "$IMAGE_FILE" "$IMAGE_BASE/$IMAGE_FILE"
curl --fail --show-error --location --proto '=https' --proto-redir '=https' \
  --output SHA512SUMS "$IMAGE_BASE/SHA512SUMS"
sha512sum --check --ignore-missing SHA512SUMS
qemu-img info "$IMAGE_FILE"
pvesh get /cluster/nextid --vmid 9000
```

Require the exact image's `OK`, the pinned hash above, qcow2/no backing file/3 GiB,
and the unused ID. Reobserve storage and bridge before these mutations:

```sh
qm create 9000 --name vmanyware-debian-13-20261015 \
  --memory 2048 --cores 2 --ostype l26 \
  --scsihw virtio-scsi-pci --net0 virtio,bridge=vmbr0
qm set 9000 --scsi0 "local-lvm:0,import-from=$IMAGE_DIR/$IMAGE_FILE"
qm disk resize 9000 scsi0 8G
qm set 9000 --ide2 local-lvm:cloudinit
qm set 9000 --boot order=scsi0 --serial0 socket --vga serial0
qm set 9000 --ciuser debian --citype nocloud --ciupgrade 0 --ipconfig0 ip=dhcp
qm config 9000 --current 1
qm status 9000
```

Review configuration and stopped status before converting. Then recheck the
role does not already exist before creating it and granting the two scoped ACLs:

```sh
qm template 9000
qm config 9000 --current 1
pveum role add VMAnywareTemplateReader --privs VM.Audit
pveum acl modify /vms/9000 --users vmanyware-reader@pve \
  --roles VMAnywareTemplateReader --propagate 0
pveum acl modify /vms/9000 --tokens 'vmanyware-reader@pve!inventory' \
  --roles VMAnywareTemplateReader --propagate 0
```

The token identifier is public metadata; never put its secret in CLI arguments.
Existing account/token expiry remains 2026-10-18 21:22:17 UTC. Expiry/replacement
is a separate credential operation, not an automatic part of template creation.

## Acceptance after approval and execution

Require stopped status, template=1, scsi0=8G, expected controller/boot/Cloud-Init
and no configured guest password, SSH key, hook or extra disk. Run the opt-in
Rust inventory probe using only the existing reader token; require template
9000 on pve-lab. Read its current configuration with that token and confirm
VM.Audit is limited to `/vms/9000`. Inspecting a known template can close that
specific feasibility item; it does not establish cloning/readiness/snapshots.

The prepared local catalog is `.lab/pve/templates.yaml` with revision
`debian-13-amd64-20261015-2632-r1`. It is an operator label linked here to the
verified image hash, not a digest calculated from a running template.

## Network boundary for the later clone

The template remains unbooted. Its dormant DHCP setting follows the current
catalog/preview contract; it is not a claim that an inner guest can get a lease
or SSH connectivity behind this outer QEMU NAT. Do not start a clone with this
assumption. The next guest slice must approve an explicit isolated addressing,
DHCP/routing and host-to-guest SSH plan. No existing management bridge alteration
or broader LAN exposure is authorized by this template proposal.

## Actual execution evidence — 2026-10-18

The user approved creation/conversion and the scoped ACL changes. Rechecked
unused ID, storage, bridge and absent role before execution. Downloaded the
pinned image again inside pve-lab over verified HTTPS and required the pinned
SHA-512, qcow2 format, no backing file and 3 GiB size before importing.
Commands were staged through the owned console, with script hashes compared
before execution; the temporary loopback server exposed only the public scripts
and was stopped afterward. The administrative console was logged out.

The first stage created/configured 9000 and stopped for a verified-TLS API
readback before conversion. The second stage converted only that VM and created
the role/grants. Final observed properties:

- `template=1`, status `stopped`, 2 cores, memory 2048 MiB. No start was issued.
- `scsi0=local-lvm:base-9000-disk-0,size=8G`.
- `ide2=local-lvm:vm-9000-cloudinit,media=cdrom`.
- virtio-scsi-pci, boot order scsi0, serial0 socket and serial VGA.
- Cloud-Init debian/NoCloud, ciupgrade=0, ip=dhcp, virtio NIC on vmbr0.
- No configured guest password, SSH keys, hook, passthrough, custom QEMU args
  or extra/unused disk.

Administrative readback confirms role `VMAnywareTemplateReader` contains only
VM.Audit and exactly two direct nonpropagating grants on `/vms/9000`, one for
the reader user and one for the inventory token. Token separation and expiry
are unchanged. The reader's own GET `/access/permissions` returns only
`{"/vms/9000":{"VM.Audit":0}}` (0 denotes nonpropagating permission).

The same reader can list exactly one visible stopped template and GET its
current configuration with HTTP 200, using explicit CA/hostname verification.
The existing Rust probe exits zero and reports:

```text
Proxmox version 9.2.2 (release 9.2)
Node pve-lab: online
Template 9000: vmanyware-debian-13-20261015
```

No clone, nested guest execution, guest Cloud-Init completion, readiness,
snapshot or restore has been proven. Next: propose the isolated network and
concrete clone target before any clone/start operation.

The subsequent [clone/network proposal](x86-clone.md) selects VMID 201 and a
private static-address bridge for first-boot SSH testing. It is prepared only;
creation/start/network/identity operations still need explicit approval.
