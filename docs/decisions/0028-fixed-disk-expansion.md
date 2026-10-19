# ADR 0028 — Fixed primary-volume expansion

Date: 2026-10-18
Status: Transport and configured-size observation tested locally; durable storage/executor implemented locally; production caller pending.

The plan chooses only its catalog-mapped scsi0, virtio0 or sata0 device, requested
integer GiB and template source GiB. A request also requires the exact previously
observed volume identifier and configuration revision. Volume syntax is bounded,
on the configured destination storage, without empty/path traversal components or
form-field injection. This does not establish provider volume ownership: the
future executor must observe the exact stopped clone, retain its volume/revision
and commit a separate stage marker before dispatch.

The [upstream QEMU resize API](https://github.com/proxmox/qemu-server/blob/master/src/PVE/API2/Qemu.pm)
uses PUT resize and returns a task whose kind is resize and whose VMID is the
TARGET. Send only disk, absolute size (for example 40G) and digest. Do not send
relative increments, skiplock or arbitrary storage/volume overrides. Reject
non-expansion requests, including equal source/target sizes; the deterministic
plan already omits expand_disk for those cases.

Before PUT, read the selected drive and independently observe the exact clone and
applied CPU/memory settings, stopped power and absence of pending changes. Require
matching configuration digests across those reads, the retained dispatch revision,
the exact retained volume and the original catalog source size. Ambiguous fields,
duplicate keys, CD-ROM media, missing sizes, fractional syntax and integer overflow
fail closed. Integer byte/K/M/G/T sizes are supported. The provider also receives
the digest to reject concurrent configuration changes under its configuration
lock. This is not an atomic lock against external administrators/power changes.

Validate the returned task against target node, resize kind and canonical target
VMID; reject source-bound tasks, other actions, foreign nodes and aliases. The
existing verified-TLS client disables retries and redirects. One dispatch has a
20-second deadline; lost/malformed replies never trigger another PUT, despite the
absolute size reducing the risk of an accidental increment. Durable uncertainty, retained task linkage and restart guards are now implemented
in storage and a standalone executor is locally tested.

After task success, an independent GET-only observation requires the same exact
volume/device and requested size, current CPU/memory, stopped clone identity,
consistent revisions and no pending changes. It reports configured provider size,
not independently measured backend allocation, guest partition/filesystem growth
or readiness. Actual installed-provider behavior and guest growth remain real-lab
gates. No production worker/API dispatches this transport yet.

Four targeted generated-HTTPS tests pass: exact form and task binding; pre-write
volume/size/revision/resource/identity rejection; applied size observation and no
replay after malformed or genuinely dropped replies; and all catalog devices,
including a file-backed volume and an unrelated disk that must never be selected.
Scoped library Clippy passes. No dependency, migration, native/client changes or
real VM mutation are part of this slice.

See [ADR 0029](0029-durable-disk-stage.md) for schema-8 disk journal guarantees.
