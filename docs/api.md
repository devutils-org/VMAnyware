# Local control plane API

Status: compiled and exercised on macOS arm64 with Rust 1.99.1. Configuration
and HTTP contract tests pass; a live health request returns HTTP 200.

The service binds only `127.0.0.1`. `POCKETLAB_PORT` sets the port (default 8080,
range 1–65535). No host override, authentication or CORS permission exists yet.
SQLite definitions and an optional read-only Proxmox provider are available.
The native client connects through Rust to this
local service; credentials are never sent to the WebView.

Definition validation/create/list/read routes are documented in
[the definition API and schema](definitions.md). They operate without Proxmox.
Asynchronous inventory requests and persisted statuses are described in
[durable operations](operations.md).
Image catalog and offline preview routes are documented in
[provisioning plans](provisioning-plan.md).

## Read-only inventory

`GET /api/v1/provider/inventory` reads the configured Proxmox provider and returns:

```json
{
  "provider": "proxmox",
  "version": "9.0.1",
  "nodes": [{"name": "pve-1", "status": "online"}],
  "templates": [{"node": "pve-1", "vmid": 9000, "name": "debian"}]
}
```

This is a response-shape example, not evidence of a real host. Entries are
sorted and restricted to what the token can see. Offline nodes are shown, but
their templates are not queried. The entire collection has a 20-second deadline;
partial failures return an error rather than presenting incomplete inventory as
success. The supported maximum is 100 visible nodes.

Without runtime configuration, return HTTP 503 with `provider_not_configured`.
Invalid/partial configuration refuses server startup. Provider HTTP 401/403 map
to HTTP 502 with `provider_unauthorized`/`provider_forbidden`. Timeouts map to
HTTP 504 with `provider_timeout`; other failures map to HTTP 502 with
`provider_unavailable`. All use the fixed safe error envelope below.

Configure runtime variables as described in [Proxmox setup](proxmox-setup.md).
`make server` does not contact Proxmox until an inventory request arrives.
In the native application, enter the local server port and click **Connect**.
The current connection prototype supports this computer only; remote control
planes require authentication and TLS configuration in a later slice.

## Liveness

`GET /api/v1/health` returns `200 OK`, `Content-Type: application/json`:

```json
{"status":"ok"}
```

This means the HTTP process responds. It does not assert Proxmox connectivity,
database availability, guest readiness or completion of an operation. The
response contains no configuration, credentials, host inventory or versions.

## Errors

Unimplemented routes return HTTP 404:

```json
{"error":{"code":"not_found","message":"Route not found."}}
```

Unsupported methods on a known route return HTTP 405 with code
`method_not_allowed` and message `Method not allowed.` in the same envelope.
Requests and query parameters are never echoed into these responses.

## Run and verify

```sh
make server
```

In another terminal:

```sh
curl --fail-with-body http://127.0.0.1:8080/api/v1/health
make check-server
```

Ctrl-C requests graceful shutdown. Production service lifecycle and deployment
remain outside this initial slice. `make check-server` builds/tests only the
server package, without requiring native Tauri libraries or frontend assets;
Cargo still resolves the shared workspace dependency lock.

Implementation references: [Axum routing and JSON](https://docs.rs/axum/latest/axum/),
[Axum serving](https://docs.rs/axum/latest/axum/fn.serve.html).
