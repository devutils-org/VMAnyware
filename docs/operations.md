# Durable inventory operations

The first asynchronous operation is **inventory_refresh**. It performs only the
three provider reads documented in `proxmox-setup.md`. No arbitrary job payload,
shell command or VM mutation is accepted.

## API

`POST /api/v1/provider/inventory/refresh` requires an `Idempotency-Key` header
of 1–64 ASCII letters/digits/dots/hyphens/underscores. Browser `Origin` headers
are rejected, and CORS is not enabled. The server commits a queued operation
before responding with HTTP 202 and `Location: /api/v1/operations/{id}`.

```json
{
  "id": "0123456789abcdef0123456789abcdef",
  "kind": "inventory_refresh",
  "status": "queued",
  "result": null,
  "failure": null,
  "createdAt": "2026-10-16T12:00:00.000Z",
  "updatedAt": "2026-10-16T12:00:00.000Z"
}
```

Repeating the same key returns the original operation, including its final
status after restart. A new key represents a new attempt. Different keys cannot
queue overlapping inventory refreshes: an active queued/running operation causes
HTTP 409 (`operation_conflict`). No second operation is created by the rejected
request. Keys are currently scoped to this one operation kind; future mutation
operations must also bind idempotency to their target and input hash.

`GET /api/v1/operations/{id}` returns the persisted operation or HTTP 404
(`operation_not_found`). Operation IDs are 32 hexadecimal characters. Successful
results use the inventory JSON shape in `api.md`. Errors never contain raw
provider response bodies, credentials or transport diagnostics.

The synchronous `GET /api/v1/provider/inventory` remains available for existing
clients. The native application uses queued operations and polls their status.

## States

| State | Meaning |
| --- | --- |
| `queued` | Persisted but not yet claimed by the worker |
| `running` | Claimed atomically; provider collection in progress |
| `succeeded` | Typed inventory result persisted |
| `failed` | A fixed safe failure code persisted; no result |
| `interrupted` | Service stopped before confirming the outcome; no success claim |

Normal transitions are queued → running → succeeded/failed. Recovery changes
running → interrupted. Terminal operations cannot be overwritten or replayed.
Queued reads can be claimed after restart; interrupted operations require a new
request key to retry. Success means inventory was collected, not that a VM is
running or SSH-ready.

Failure codes are `provider_not_configured`, `provider_unauthorized`,
`provider_forbidden`, `provider_timeout`, `provider_unavailable` and
`process_interrupted`. The latter explicitly means the prior outcome is unknown.
VM operation reconciliation will require provider task IDs/ownership checks;
this inventory worker is not a general mutation/retry framework.

## Worker, migration and process ownership

The binary runs one inventory worker with a 250 ms idle interval. Claims use a
single SQLite compare-and-set; results are committed before polling reports a
terminal state. Database work runs outside the async executor. Provider collection
keeps its existing 20-second overall deadline. Worker storage failures stop the
service so operations cannot silently remain orphaned while HTTP stays healthy.

`0002_operations.sql` upgrades SQLite schema version 1 to 2 transactionally and
preserves saved definitions. A partial unique index enforces one active refresh.
On exclusive binary startup, running records become interrupted before the
worker starts. Opening a database connection alone never performs recovery.

The service holds an OS lock on `<database>.lock` for its lifetime. A second
service using that same database path refuses startup before migrations/recovery.
The lock file is intentionally retained: stale files do not imply stale locks,
and deleting a held lock would defeat mutual exclusion. Use one canonical local
database path; symlink aliases/network filesystems and multi-instance deployments
are not supported. In-memory databases have no shared file to lock.

Operation retention/pagination and cancellation are not implemented. Records are
retained until a future explicit retention policy is added.

## Native application

**Connect** starts an inventory refresh and displays its persisted status. It
retains the request key per local service port in localStorage when available.
After a connection interruption or app reload, **Connect** resumes the same
request. The cached key is cleared after a terminal result, so another click
starts a new refresh. Only a non-secret request key is cached; no credentials.
If browser storage is unavailable, caching is best-effort and the server record
remains addressable by ID.

Monitoring pauses after roughly a minute; this does not cancel the operation or
declare failure. Connect again to resume. Changing ports resets the displayed
result; the cache for each port remains independent.

## Verification

Tests cover persisted terminal results, migration from schema 1, duplicate keys
across database connections, atomic claims, active conflicts, interruption
recovery, native HTTP headers/polling, TLS-backed worker success and missing
provider failures. Default checks never contact real Proxmox infrastructure.
