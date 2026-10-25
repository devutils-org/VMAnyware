# 0092 — Restore storage check observes current snapshot support

Status: accepted (2026-10-25)

## Context

Restore preflight queried
`GET /nodes/{node}/qemu/{vmid}/feature?feature=snapshot&snapname={name}` and
required `hasFeature=1`. On the home Proxmox 9.2.21 host with LVM-thin
(`local-lvm`), the user-run read-only queries for VM 9001 returned
`hasFeature:1` without `snapname` and `hasFeature:0` with
`snapname=accept-base`. Proxmox evaluates the feature for the named state:
LVM-thin can snapshot the current state but not a snapshot. The check
therefore blocked every restore on the default Proxmox storage although
rollback is supported.

## Decision

The `restore_snapshot_storage` check observes snapshot support of the current
VM state (`feature=snapshot` without `snapname`), still between the two exact
saved-configuration reads. Other restore checks are unchanged: stopped power,
existing complete disk-only snapshot, marker identity, stable saved-config hash
and unchanged live target. The unused named-state helper is removed.

## Consequences

Proxmox API offers no read-only rollback-possibility query. Storage-specific
rollback refusals (for example ZFS rollback past a newer snapshot) surface as
provider task failure handled by the existing executor, not as preflight
proof. Real evidence: after the fix, restore of `accept-base` on 9001 succeeded
and a guest file written after the snapshot was absent after restart.
