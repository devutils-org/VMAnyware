# 0085 — Recorded provisioning progress API

Status: implemented; targeted acceptance recorded in STATE.md.

## Decision

`GET /api/v1/operations/{id}/provisioning` exposes a bounded projection of the
retained sequential provisioning journal. It uses the existing authentication
boundary and one SQLite read transaction. It works without current provider,
catalog or write flags, does not contact Proxmox and does not repair or replay work.

The response binds operation, new instance, optional retired source, provider scope,
node/VMID and definition/plan hashes. Each ordered stage has its position, fixed
kind, recorded status and historical `submissionStarted` marker. The marker is
not proof of a successful write, current power, current guest readiness or SSH.
This remains a historical observation; existing operation/result validation owns
readiness and SSH handoff. Stage results, YAML, approval, request key, provider task,
guest PID, seed journal and guest/provider output are never included.

The existing strict retained-job decoder checks identity, source lineage, plan,
ordered stage kinds and task scope. The projection additionally rejects a stage
started after an incomplete predecessor and contradictory overall completion.
Missing/malformed/non-provisioning IDs return a fixed 404; unavailable or corrupt
storage returns a fixed 503, without details or journal mutation. No schema,
dependency, production write capability or native application command is added.

The client progress reader and four creation/recovery views are implemented in
ADR 0086. Refreshed delivery archives include this endpoint; the exact extracted
server also passes the joined declared native workflow. Actual Android runtime
and real guest acceptance remain separate gates.
