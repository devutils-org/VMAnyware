# ADR 0061 — One-shot deletion and read-only reconciliation

Date: 2026-10-20
Status: Accepted for standalone local implementation; runtime integration and real acceptance open.

## Decision

The fixed provider transport issues DELETE only to the retained node/VMID, with
`purge=0&destroy-unreferenced-disks=0` in its query and an empty body. It never
adds skiplock, a power action, storage deletion requests or caller-controlled
parameters. Existing verified TLS, bounded responses, redirect refusal and
transport retry prohibition apply. A returned task must match node, qmdestroy and
VMID. Explicit 401/403 refusals are definite; other errors or unverified replies
leave submission uncertain and cannot authorize another attempt.

The standalone executor receives an explicit dispatch capability. Before a new
write it checks retained provider/instance identity and proves current storage
permissions support later complete absence reads. Then it repeats the whole
read-only stopped/protection/ownership/pending/lock/snapshot/HA/replication/effects
review. Current digest and every public plan field must exactly match durable
approval. Passing prerequisites do not renew the 120-second approval lifetime.
Commit the once-only dispatch marker before DELETE, then retain a valid returned
task immediately. Neither task linkage nor dispatch markers may be replaced.

Known-task recovery bypasses the write path regardless of capability. Poll with
bounded reads, then use ADR 0060's independently created task/absence completion
proof for atomic retirement. Failed destroy tasks remain uncertain because cleanup
may be partial. Missing task outcome, VM/volume absence, provider or matching scope
also retains uncertainty and mutation exclusion. Backoff applies; later evidence
may complete the original task without a new DELETE. Unknown submission without
a verified task remains non-reclaimable and needs explicit future reconciliation.

## Provider contract and limitation

The upstream [VM destroy API](https://github.com/proxmox/qemu-server/blob/master/src/PVE/API2/Qemu.pm)
accepts the fixed deletion options and returns qmdestroy; it repeats running,
protection and HA/replication checks under its configuration lock. It has no digest
parameter for atomic comparison with our approval. The [HTTP parameter parser](https://github.com/proxmox/pve-http-server/blob/master/src/PVE/APIServer/AnyEvent.pm)
extracts DELETE parameters from the query, rather than parsing a form body.

Our review and provider deletion are separate requests. Another administrator or
external actor can change target configuration/ownership between them. The local
mutation journal prevents competing VMAnyware operations, not external changes.
Do not claim atomic provider consent enforcement or real-backend acceptance from
these fixtures. This constraint must be reviewed with the concrete disposable
real target before enabling an infrastructure test. No generic override is added.

## Evidence and integration boundary

Eight generated-HTTPS tests cover exact single DELETE/empty body, independent
retirement, 18 before-dispatch refusal variants, dropped/wrong task replies,
restart with writes off, partial cleanup, task/read failures, later successful
GET-only reconciliation, missing/changed provider, 401/403/500 and relative
redirect refusal. The affected public-preflight fixture passes after extracting
its unchanged route builder for this executor's reviews. Server library compilation
passes. No unrelated suites, real infrastructure or credential files are involved.

No runtime flag, worker wiring, HTTP submission or native/client deletion controls
are added here. API preflight execution remains false. Next integrate a separately
default-off runtime capability and fair worker, with process-level evidence.
