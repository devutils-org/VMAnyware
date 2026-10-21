# 0081 — Admit declared packages and final snapshots through the API

Status: API/worker integration passes owned HTTPS tests; native UI and real infrastructure remain open.

## Decision

Creation and retained-definition recreation now accept validated Debian package
declarations and snapshotAfterProvisioning. Admission uses the same declaration
validator as the fixed package transport, preventing definitions which would stop
the worker before dispatch. The image-default user attestation remains required.
A requested final snapshot additionally requires the existing independent snapshot
write flag; otherwise plan/preflight reports creationEnabled=false and submission
returns 403 snapshot_disabled without reserving an instance or enqueuing work.
Invalid package declarations return 422 package_declarations_unsupported. Creation
and lifecycle gates retain their existing independent meaning.

Definition/plan hashes bind the exact requested packages and optional snapshot.
The API still performs no provider request during enqueue and returns the durable
operation immediately. The existing sequential package/final-snapshot workers must
complete all declared stages before success. Known task/PID observations retain
the earlier no-replay rules; API admission does not grant a recovery mutation.

## Initial instance marker

The complete journey exposed a real mismatch: clone ownership was stored in its
exact description, while lifecycle/snapshot preflight requires a sole matching
vmanyware-<instance-id> tag. Resource configuration now sets that exact fresh tag
in its existing stopped-clone/digest-bound PUT, and observation verifies it alongside
CPU/memory. No extra mutation, relaxed identity check or opportunistic tag repair
is added. The initial tag replaces inherited template tags on the new clone;
existing VMs are not retroactively edited. Older uncertain resource dispatches
without this tag remain unconfirmed and cannot be replayed automatically.

Proxmox documents VM tags and their configuration permission in its
[official tag documentation](https://github.com/proxmox/pve-docs/blob/master/pve-gui.adoc).
Real least-privilege ACL acceptance for this added field remains open.

## Evidence and limits

Bearer-protected Axum API/actual worker/generated HTTPS provider journeys pass for
create and recreate, each with package-only, snapshot-only and combined declarations,
and both equal/expanded disks: twelve declared workflows. They verify all stages,
scoped package proof, snapshot metadata, mutation counts, restart, same-key retry and
retired-source preservation/read-only saved recovery. No real guest executes apt
in these fixtures; the provider returns controlled PID/exit evidence.

Six create API checks, seven recreation API checks, seven resource transport/
observation/executor checks and the two existing no-feature create/recreate journeys
pass. Missing/foreign tags are rejected by resource observation. The generated
Android/Linux provider fixture now stores/checks the actual tag sent by the server.
Native create plan acceptance/form/result display still restrict these declarations;
that client slice and exact-artifact delivery/execution come next. Current packaged
server/APKs predate this source activation. No real VM mutation was performed.
