# Local environment instance associations

An instance registration connects a saved definition to an explicit proposed
Proxmox node/VMID. It changes only the local SQLite database. It does not discover,
create, start, stop, adopt or verify ownership of a VM. `status: registered` is
neither an observed provider power state nor guest readiness.

## Register and read

First save a definition using the [definition API](definitions.md), then send:

```http
POST /api/v1/environments
Content-Type: application/json

{
  "definitionName": "kernel-lab",
  "definitionHash": "<64 lowercase hexadecimal characters from the saved definition>",
  "node": "pve-test",
  "vmid": 202
}
```

The hash placeholder must be replaced with the exact saved definition hash.
The definition name follows the YAML name rules; node/VMID follow
[VM status target validation](vm-status.md). Unknown/duplicate fields, invalid
JSON/types, or invalid targets are rejected. The request body limit is 64 KiB.
A browser `Origin` header is forbidden. These controls do not replace the future
control-plane authentication required before remote exposure.

Registration requires configured Proxmox to derive the target's endpoint scope;
it makes **no network request**, so an unreachable endpoint does not prevent local
registration. The caller cannot choose the scope, status, YAML or revision.
The saved definition name/hash must match. Registration returns HTTP 201 and
`Location: /api/v1/environments/{id}` after a successful transaction.

The response contains:

- `id`: generated 32-character hexadecimal instance ID.
- `definitionName`, `definitionHash`, `definitionRevision`: saved definition identity.
- `definitionYaml`: exact retained canonical definition, including public SSH keys.
- `providerReference`: `provider: proxmox`, opaque `scopeId`, `node` and `vmid`.
- `status: registered` and `createdAt`.

A future definition edit cannot change this retained snapshot. The definition
itself remains reusable, and several instances may reference it.

`GET /api/v1/environments` returns the newest 100 summaries ordered by creation
time descending, then ID descending. Summaries omit `definitionYaml`; empty
storage returns `[]`. `GET /api/v1/environments/{id}` returns the full retained
record or HTTP 404 `instance_not_found`. Older records remain addressable by ID;
pagination, deletion and retention are not implemented. Reads require neither
provider configuration nor connectivity and never update observed VM state.

## Scope and conflicts

`scopeId` is SHA-256 of the configured canonical HTTPS origin. Credentials,
CA bytes and endpoint URLs are not copied into instance rows or responses.
A token rotation keeps scope; changing hostname/port changes it. The scope hash
is public metadata, not authentication. Endpoint aliases are not recognized as
the same cluster, and a reused hostname does not prove infrastructure continuity.

Only one association may exist per (scope, VMID), even on different nodes.
Duplicate registration returns HTTP 409 `instance_exists`; there is no silent
replacement or idempotent overwrite. If a response is lost, inspect the list
before retrying. A moved node requires future explicit reconciliation.

A missing definition returns HTTP 404 `definition_not_found`; a different saved
hash returns HTTP 409 `definition_changed`. Invalid semantic inputs return HTTP
400 `invalid_instance_request`; schema/type errors return HTTP 422 with the same
code. Invalid JSON is HTTP 400, wrong content type HTTP 415 and an oversized body
HTTP 413. No provider configuration returns HTTP 503 `provider_not_configured`;
storage failures return HTTP 503 `storage_unavailable`. Errors contain fixed
messages without parser/database diagnostics or request-controlled values.

## Storage and verification

`migrations/0003_instances.sql` transactionally advances schema 2 to 3 while
retaining definitions and operations. The table uses a definition-name foreign
key and copies YAML/hash/revision in the same write transaction. Registration
validates the retained definition's schema/hash before copying it. Existing
process locking and database busy timeout continue to apply.

Local tests cover clean/v2 migration, database reopening, immutable snapshots,
concurrent duplicate registration through separate connections, distinct endpoint
scopes, input rejection, bounded deterministic list results, detail reads without
provider configuration, and zero provider connections during HTTP registration.
Subprocess log checks cover static route patterns without names/IDs/body data.
No real infrastructure or native-client behavior is asserted.

Before start/shutdown is added, current scope, VM identity/ownership and lifecycle
state must be verified independently, and action/task linkage must survive
interruption. See [ADR 0020](decisions/0020-local-instance-registration.md).

## Observe an association

`GET /api/v1/environments/{id}/status` reads its saved target after checking that
the configured provider scope still matches. It returns `instanceId`,
`providerScopeId` and `observation` (the node/VMID/status/qmpStatus shape in
vm-status.md). A missing instance is HTTP 404; absent configuration is HTTP 503;
changed scope is HTTP 409 `provider_scope_changed` before any provider request.
Invalid persisted target data fails with HTTP 503 `storage_unavailable`.
Provider errors use the safe VM-status error mapping. Registration is unchanged.
Caller query parameters cannot select a different target.

For asynchronous monitoring use `POST /api/v1/environments/{id}/refresh` with an
`Idempotency-Key` header using the same rules as inventory refresh. It returns
HTTP 202/Location after persisting an `instance_observe` operation with `instanceId`.
It does not wait for Proxmox and may enqueue while provider configuration is
absent; the worker then records `provider_not_configured`. Poll the standard
operation detail route for its result. Success contains the same scoped
observation shape as the synchronous route and is not guest readiness.

Request keys are bound to operation kind and instance. A reused key for another
kind/target, or another active observation for the same instance, returns HTTP
409 `operation_conflict`. Distinct instances may queue independent reads. Missing
or invalid keys are HTTP 400 `invalid_request_key`; Origin headers are HTTP 403.
A missing instance is HTTP 404. No failed enqueue creates work.

Changing the configured endpoint before execution produces a persisted
`provider_scope_changed` failure without provider access. Invalid/missing saved
references fail as `instance_reference_invalid`. Running reads interrupted by
service shutdown become `interrupted` on exclusive restart; queued reads remain
queued. Retrying the same key returns the original terminal record, while a new
key explicitly requests a new read. No operation is silently replayed.

Migration 4 adds the operation/instance foreign key while retaining instance
snapshots and all prior inventory operations. See [ADR 0021](decisions/0021-durable-instance-observation.md).

## Native client preparation

**Load registered instances** explicitly reads local summaries. **Observe VM**
starts or resumes an instance-bound operation, polls until terminal state and
shows provider power/QMP state separately from unproven readiness/ownership.
Monitoring pauses after approximately a minute; it does not cancel backend work.
A non-secret request key is cached per service port and instance when browser
storage is available. It is cleared only after a confirmed terminal result;
transport failures, paused monitoring and leaving the view retain it for resume.
Without browser storage, resume caching is best-effort and the operation remains
readable by ID/history. Changing port hides old results and stops polling the
old view; late results/errors are ignored. Polling is never automatic on startup.

Result kind, operation ID, instance identity, provider scope and node/VMID are
checked before display. A failed read clears the earlier observation. The client
does not register/adopt/start/stop a VM; registration remains available through
the local API. The history includes observation kind and instance ID.

The frontend build, six observation-controller tests and disposable Linux
Firefox simulated-IPC walkthrough pass. Native command HTTP tests now compile and pass on Linux using a temporary
signed-package WebKitGTK prefix; see [native validation](native-linux-prefix.md).
Use `make check-instances-browser` for the explicit browser fixture. This is not
an integrated native/real-provider or macOS WebKit result.

## Lifecycle preconditions

The [read-only lifecycle preflight](lifecycle-preflight.md) separately checks
the proposed instance marker and provider configuration. Registration does not
install a marker. A matching public tag is an administrative declaration, not
independent ownership proof; preflight never enables a write or changes instance
status.
