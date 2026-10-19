# ADR 0026 — Fixed stopped-clone CPU and memory configuration

Date: 2026-10-18
Status: Transport and applied-value observation tested locally; standalone durable stage caller implemented; production caller pending.

The approved plan supplies CPU and memory; its clone request supplies exact target
node/VMID, name and generated instance/definition/plan description. A caller must
retain a valid observed configuration digest and commit a separate resource-stage
dispatch marker before using this transport. No production worker/API calls it yet.

Use the [upstream synchronous QEMU configuration API](https://github.com/proxmox/qemu-server/blob/master/src/PVE/API2/Qemu.pm)
(PUT config, null response) for the stopped clone. Send only digest, sockets=1,
cores=requested CPU, vcpus=requested CPU, cpulimit=0, memory=requested MiB and
balloon=0. Explicit topology and disabled limiting/ballooning prevent inherited
settings from reducing the requested resources. Disk expansion, network,
Cloud-Init and tags remain separate work; this transport is not a generic map.
The installed provider's supported fields and least-privilege permissions still
require approved real-target validation.

Before PUT, observe the exact clone's current digest/identity, non-template and
unlocked configuration, stopped power/QMP and absence of pending changes. Require
the approved digest and also send it to Proxmox to reject concurrent configuration
changes. These observations cannot lock out an external administrator or reserve
a provider target; the digest protects configuration changes, not an atomic power
transition. The complete operation has a 20-second deadline. The existing verified
TLS client disables redirects and retries. An ambiguous response cannot justify
another PUT; resource-stage recovery must use GET-only reconciliation.

Applied-value observation reads settings, independently validates the clone and
requires matching configuration revisions across those reads. Require exact
sockets, cores, vcpus, CPU limit, memory and balloon values, plus no pending changes.
The resulting evidence may carry a newer digest than the original dispatch input.
It establishes CPU/memory observation only, never guest readiness or whole-plan
success. Later durable completion must bind this evidence to its retained inputs.

Four targeted generated-HTTPS tests cover exact fixed form, target/revision/
lock/template/power/pending rejection before PUT, exact applied values and
consistent observation revisions, and one PUT with no replay after malformed
submission replies. Scoped library Clippy passes. No infrastructure was mutated,
no new dependency was added and no complete provisioning flow is claimed.

Durable stage execution/recovery is now covered by [ADR 0027](0027-durable-resource-stage.md); production provisioning remains unavailable.
