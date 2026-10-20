# ADR 0056: Bound read-only restore selection before destructive execution

Date: 2026-10-20. Status: accepted for the first restore path; execution pending.

Restore is a distinct destructive operation, not a snapshot creation retry.
The first path requires a VM already stopped and an explicitly disk-only named
snapshot. It will not shut down or start the VM automatically. The eventual
approval must independently acknowledge data loss and bind the complete instance,
provider scope, definition, live configuration and selected snapshot fingerprint.
Creation's runtime flag/consent does not grant restore capability.

The provider snapshot-config GET returns saved configuration, without a dedicated
snapshot revision digest. Compute a domain-separated SHA-256 over its API-visible
canonical JSON and exact name, sorting object keys recursively and preserving
array order. Never return or log that configuration. This fingerprint is a
comparison token, not a hash of disk contents or proof of restore viability;
masked API fields remain masked. Independently compare normalized list metadata
before/after as well.

Preflight verifies existing live identity/template/lock/pending/configuration
guards, stopped power before/after, selected existence, complete snapshot work,
explicit disk-only metadata, matching saved instance marker and non-template/
unlocked saved configuration, stable snapshot metadata/fingerprint, provider-
reported snapshot feature for that saved configuration, and final live identity.
All reads fit a 20-second overall deadline. No rollback, automatic power action,
reservation, approval record or operation is created. Execution reports false.

RAM snapshots are deliberately unsupported initially because the provider may
start them during rollback independently of a false start option. Snapshots from
before this instance's matching marker are also unsupported by this path. These
limits remain explicit; they do not establish broad restore acceptance.
A passed report cannot remove races with external actors, verify guest/application
consistency, prove independent backup, grant VM.Snapshot.Rollback permission or
verify restored guest contents. Real restore acceptance must verify changed guest
state on an approved disposable target.

Upstream contracts: [snapshot config and rollback API](https://github.com/proxmox/qemu-server/blob/master/src/PVE/API2/Qemu.pm).
Evidence: six targeted generated-HTTPS/API restore tests plus six affected existing
identity/name tests pass. Durable destructive approval, conflict exclusion,
no-replay rollback task journal/executor and native/client controls remain next.
