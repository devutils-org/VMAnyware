# ADR 0062 — A local provider beside Proxmox

Date: 2026-10-21
Status: Proposed. Scope change requested by the user; the hypervisor choice and
the first local target await the user's approval. No code yet.

## Context

`VMAnyware.md` §9 keeps additional hypervisors out of the MVP "unless the user
explicitly changes the scope". The user has: VMAnyware is to be usable without
a Proxmox host, with environments provisioned on the machine it runs on, in
addition to Proxmox.

§3 also says not to introduce a provider abstraction before the Proxmox
implementation shows which interface is needed. It now does: the server talks
to Proxmox for inventory, a clone from a template, CPU/memory/disk settings,
Cloud-Init, power, status, guest-agent readiness, snapshots, restore and
deletion (ADRs 0024–0061). Proxmox types stay in `server/src/proxmox*`, but
the executors call that module directly; there is no seam yet.

On the development machine, QEMU and `/dev/kvm` are present; libvirt is not
installed. distroproof, a sibling project, already boots cloud images on QEMU
directly (overlay disks, Cloud-Init NoCloud seed, QMP), as an ordinary user.

## Decision (proposed)

1. **A narrow seam, cut from what exists.** One `Provider` trait in the
   server with the operations of the first local slice only: list base
   images, create an instance from an image and a definition (CPU, memory,
   disk, Cloud-Init user and key), start, stop, status, guest readiness and
   delete. The Proxmox module implements it by moving existing calls, not by
   changing them; snapshots, restore and Proxmox's approval/journal machinery
   stay where they are until a second provider needs them.
2. **Direct QEMU, no new daemon (recommended).** A `local` provider runs
   `qemu-system-*` with KVM as the server's user: a qcow2 overlay on a
   verified cloud image as the "template clone", a NoCloud seed for Cloud-Init,
   a QMP socket per instance for power and status, user-mode networking
   (passt or slirp) with one forwarded SSH port. The server supervises the
   processes and records each instance's files under its data directory, so
   instances survive a server restart (re-attached through QMP) but not a host
   reboot unless started again. distroproof's approach is borrowed; its code is
   not modified or depended on.
   - *Alternative: libvirt (`qemu:///session` or `qemu:///system`).*
     Persistent domains, autostart, snapshots and bridged networks for free,
     at the cost of a daemon the user installs and, for system mode, a group
     membership. Worth it if local instances must outlive reboots or be
     reachable from other machines.
3. **The same safety rules as Proxmox.** Every create, start, stop and delete
   on the local provider is a VM mutation: confirmed by the user, bounded,
   idempotent by key, recorded before it acts, as ADRs 0022–0025, 0044–0045
   and 0059–0061 require for Proxmox. Local resources live only under the
   server's data directory; the provider never touches a disk, network or
   process it did not create.
4. **Definitions stay provider-neutral.** A definition names a base image and
   resources; where it runs (`proxmox:<node>` or `local`) is chosen when it is
   provisioned, not stored in the definition.

## First slice, and its acceptance

Scope: the seam with its two implementations for create, start, stop, status
and delete; the local provider behind `VMANYWARE_LOCAL_ENABLED` (default off);
the client lists `local` beside the Proxmox nodes.

Accepted when, on a machine with KVM and no Proxmox: a Debian 13 cloud image
verified by checksum becomes an instance from a saved definition, boots,
reports guest readiness, accepts the definition's SSH key on its forwarded
port, stops, restarts and is deleted with every file it created gone; the
server restarts in between and re-attaches; existing Proxmox tests still pass
unchanged; default checks need neither KVM nor Proxmox.

## Open for the user

- Direct QEMU (recommended) or libvirt.
- The first concrete local target: this machine, the data directory, and the
  image to download (AGENTS.md: no VM mutation without approval of the target
  and the operation).
