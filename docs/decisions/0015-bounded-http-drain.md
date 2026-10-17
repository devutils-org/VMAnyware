# ADR 0015 — Bound local HTTP shutdown draining

Date: 2026-10-17
Status: Implemented and verified with actual server subprocesses.

Graceful shutdown alone can wait indefinitely for an unfinished HTTP request
body. Register signals before readiness as before, then allow at most five
seconds for Axum requests to drain after a signal. An idle server exits without
waiting for that timer. Do not introduce a configurable timer or dependency for
this local prototype.

On deadline, emit only the fixed server_drain_timed_out event, abort the owned
inventory worker and return; runtime teardown closes remaining async connections.
Requested shutdown still exits zero and emits server_stopped. Neither event
claims that an incomplete request succeeded. Durable running operations recover
as interrupted on the next exclusive startup; no provider mutation or automatic
operation replay is added.

Subprocess evidence covers SIGTERM with a stalled body, safe event fields,
socket closure, database-lock/port reuse and fast idle SIGINT. This is an HTTP
drain bound, not a guarantee against arbitrary blocking OS/database I/O. See
docs/diagnostics.md. Windows code remains source-prepared, not tested.
