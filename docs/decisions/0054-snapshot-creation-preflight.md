# ADR 0054: Disk-only snapshot creation prerequisites

Date: 2026-10-20. Status: accepted for the initial snapshot creation path.

## Decision

The first creation path uses disk-only snapshots. Memory inclusion is fixed false;
it is not a caller-provided provider option. Creation requests contain a name and
optional description. Names use the Proxmox configuration-ID grammar (initial
ASCII letter, then ASCII letters/digits/underscore/hyphen), length 2–40, excluding
the exact reserved identifier `current`. Descriptions are bounded to 4 KiB UTF-8;
control characters other than newline/tab are rejected.

A read-only creation preflight binds this request to the retained instance,
provider scope, node, VMID, definition hash and configuration digest. It checks
existing identity/lifecycle guards, provider-reported snapshot feature support,
exact name availability and absence of incomplete snapshot state. It repeats
pending/identity reads after the snapshot reads to reject a changed target.
The complete read sequence has a 20-second deadline. No snapshot is reserved,
operation enqueued, or provider mutation dispatched by preflight.

## Consequences

The report exposes the independent runtime creation opt-in as `executionEnabled`;
that capability does not override failed prerequisites. A passed report is neither consent nor a guarantee against
later races. The future worker must revalidate before committing its durable
no-replay dispatch guard; Proxmox remains the final authority on uniqueness.
Approval must include the exact reviewed request and complete target. A lost
submission response cannot be retried as a new provider POST.

Disk-only snapshots are not independent backups and this preflight does not
establish guest/application consistency, quiescing, free-space sufficiency,
VM.Snapshot write permission or successful recovery. RAM capture, restore,
deletion and automatic post-provision snapshots remain outside this path.

## Evidence

Targeted generated-HTTPS and actual route tests cover bounded request parsing,
retained target binding, feature support, duplicate names, incomplete snapshot
work, final configuration changes and GET-only reads. No real VM operation was
performed. Upstream contracts:
[feature and snapshot API](https://github.com/proxmox/qemu-server/blob/master/src/PVE/API2/Qemu.pm),
[snapshot name schema](https://github.com/proxmox/pve-common/blob/master/src/PVE/JSONSchema.pm).
