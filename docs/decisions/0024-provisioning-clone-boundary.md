# ADR 0024 — Fixed full clone and retained provisioning identity

Date: 2026-10-18
Status: Clone transport/target observation implemented; durable clone executor implemented locally; production caller pending.

The existing deterministic plan supplies the source template, same-node target,
storage, definition name/hash and plan hash. A validated instance ID adds the
association identity. The fixed transport sends only newid, name, storage, full=1
and a generated description binding instance/definition/plan. It supports no linked
clone, cross-node transfer, arbitrary form map, pool or snapshot selection.

The [upstream QEMU API](https://github.com/proxmox/qemu-server/blob/master/src/PVE/API2/Qemu.pm)
was checked for the clone POST and its `qmclone` task identity. This task uses the
source VMID, not the destination VMID. Validation therefore binds its node, worker
kind and canonical source VMID; destination identity must remain in durable inputs
and be checked separately after task success. Installed-provider behavior and
least-privilege ACLs still need approved real-target evidence.

The clone API does not offer a tags argument. Set an exact generated description
at creation instead of claiming a later tag write already succeeded. After task
success, read the current target configuration and separately observe power:
require the exact description/name, valid digest, non-template, absent lock and
stopped power with absent/stopped QMP. These reads share a 20-second deadline.
This administrative marker is not independent ownership or authorization proof.
Do not infer resources, Cloud-Init, packages or guest readiness from clone success.

Before wiring a caller, persist the complete approved plan, canonical definition,
provider scope, generated association and idempotency binding. Reserve the target
locally against both existing associations and other provisioning work; Proxmox
availability observation is not a reservation. Recheck the current catalog/plan,
source and target preconditions before committing a clone dispatch marker. Once
marked, send at most one POST. Persist a validated task before polling. Unknown
submission cannot be retried under another key or silently cleaned up; known tasks
reconcile by GET. A clone may leave resources even when its task fails: retain the
association and a visible failure requiring explicit inspection/recovery.

Later configuration stages must verify this exact target and current revision
before changing resources/tags/Cloud-Init. Each side effect needs its own durable
stage boundary and appropriate uncertain recovery; never replay the entire clone
workflow after an interrupted later stage. Provisioning stays disabled by default
and requires explicit full-plan confirmation. Real mutations remain opt-in.

The dynamic clone form uses form_urlencoded, already present transitively through
url. It is now a direct server dependency for correctly escaping the generated
multiline description; Cargo.lock adds only that dependency edge, with no newly
resolved crate/version. This avoids a hand-written URL encoder or generic request
passthrough. HTTPS tests check exact fixed inputs, source-task identity, malformed/
foreign replies, safe error/size/timeout bounds and independent target observation.
No HTTP route, provisioning worker or actual infrastructure mutation is added by
this transport slice.
