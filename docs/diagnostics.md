# Local service diagnostics and shutdown

The service writes one JSON object per diagnostic line to stderr. It uses the
existing JSON serializer; no logging runtime dependency or user-configurable
verbose HTTP logging is added. Do not redirect debug output from HTTP clients
or add raw errors/configuration to these events.

`request_complete` contains only:

- `requestId`: a generated process-local numeric identifier, also returned as
  `x-request-id`; incoming request IDs are ignored.
- `method`: one standard allowlisted HTTP method or `OTHER`.
- `route`: an allowlisted static route template, such as
  `/api/v1/definitions/{name}`, or `unmatched`.
- `status`: HTTP status code.
- `durationMs`: elapsed handler time in milliseconds.

Bodies, headers, raw paths, query strings, definition names, operation IDs,
credentials, provider response bodies and database paths are not recorded.
Request IDs restart when the process restarts and are diagnostic references,
not authentication tokens. A completion event measures response construction;
it does not prove a client received the full body or that a guest is ready.
Requests aborted before response construction may have no completion event.

Lifecycle events are `server_listening` (numeric loopback port),
`server_drain_timed_out` (no fields), `server_stopped`,
and `server_failed` (a fixed application-owned stage code). A failure exits with
code 1 without printing the underlying error or user-supplied configuration.
Useful failure codes include:

| Reason | Inspect locally |
| --- | --- |
| `invalid_port_configuration` | `VMANYWARE_PORT` range/encoding |
| `invalid_provider_configuration` | Explicit provider variables and trusted CA |
| `invalid_template_configuration` | Explicit catalog path and documented schema |
| `invalid_database_configuration` | Database setting is nonempty and valid |
| `database_lock_unavailable` | Another process owns the same database, or path permissions |
| `database_unavailable` | Database access and supported schema |
| `listener_unavailable` | Selected local port is free |
| `operation_recovery_failed` | Database accessibility during recovery |
| `shutdown_signal_unavailable` | OS signal registration |
| `operation_worker_stopped_unexpectedly` | Worker/database health; restart and inspect recovered job status |
| `server_stopped_unexpectedly` | Local HTTP listener/runtime failure |

On Unix, SIGINT and SIGTERM are registered before readiness is announced and
both request Axum graceful shutdown. Active requests have up to five seconds to
drain. If an unfinished request prevents completion, the service emits the fixed
`server_drain_timed_out` event, aborts its inventory worker and returns from the
server runtime; remaining async connections close during runtime teardown. An
idle service exits immediately. The service releases its database lock on exit. Running records
remain durable and become interrupted on the next exclusive startup; queued
work remains queued. There is no provider-task cancellation or VM cleanup.
A requested shutdown exits zero even if the drain deadline expires;
`server_stopped` means the process is stopping, not that every request completed.
This bounds HTTP draining, not arbitrary blocking OS/database I/O. Windows Ctrl-C
handling is prepared in source but has not been compiled or run on Windows.

Actual subprocess tests on macOS verify redaction, generated IDs, unknown routes
and methods, SIGTERM/SIGINT exit status zero and restart with the same database.
An additional subprocess test keeps a request body unfinished through SIGTERM,
observes the five-second timeout event/exit, verifies socket closure and restarts
with the same database/port. Idle SIGINT must still exit in under two seconds.
Tests use an empty process environment and temporary database, with no provider.

Implementation references: [Axum middleware](https://docs.rs/axum/latest/axum/middleware/fn.from_fn.html),
[Tokio Unix signals](https://docs.rs/tokio/latest/tokio/signal/unix/fn.signal.html).
