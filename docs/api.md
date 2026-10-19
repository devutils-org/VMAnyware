# Local control plane API

Status: compiled and exercised on macOS arm64 with Rust 1.99.1. Configuration
and HTTP contract tests pass; a live health request returns HTTP 200.

The service binds only `127.0.0.1`. `VMANYWARE_PORT` sets the port (default 8080,
range 1–65535). No host override or CORS permission is enabled. Optional
VMANYWARE_ACCESS_TOKEN authentication protects the entire API when configured.
SQLite definitions and an optional Proxmox provider are available. Read-only
provider observations and explicitly confirmed lifecycle enqueue routes coexist. The lifecycle worker defaults to disabled; its validated
VMANYWARE_LIFECYCLE_ENABLED opt-in and current integration limits are documented
in [lifecycle execution](lifecycle-execution.md).
The native client connects through Rust to this
local service. Proxmox credentials are never sent to the WebView. A control-plane
access token entered in the new remote form is cleared from the field after
submission and retained only in native session memory.

Definition validation/create/list/read routes are documented in
[the definition API and schema](definitions.md). They operate without Proxmox.
Asynchronous inventory requests and persisted statuses are described in
[durable operations](operations.md).
Image catalog, offline preview and read-only preflight routes are documented in
[provisioning plans](provisioning-plan.md).

## Control-plane access token

When VMANYWARE_ACCESS_TOKEN is set, every route, including health and fallbacks,
requires one Authorization header with the Bearer scheme and the exact token.
Missing/invalid/duplicate headers receive HTTP 401 with code unauthorized and a
Bearer challenge. Accepted requests return X-VMAnyware-Authenticated: bearer;
the remote connection check requires this marker rather than accepting an
unauthenticated health response. Tokens contain 32–256 ASCII letters/digits/hyphens/underscores;
use a high-entropy random value, separate from all Proxmox credentials. Empty or
invalid configured values fail startup without echoing input. No local secret file
is discovered or written. Token Debug/error/log output is redacted.

An absent token preserves local development only. Binding stays loopback; this
feature does not expose a remote listener or add TLS. Any future remote proxy must
require the configured token and verified HTTPS, preserve the Authorization header
and avoid logging it. Do not put tokens in URLs. The native client now has a separate HTTPS/token connection check. Fixed remote catalog/environment reads and confirmed creation use the retained
connection. Other VM controls still use local mode. Authentication does
not replace explicit target/action confirmation for writes.

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
`make server` does not contact Proxmox until an inventory or preflight request arrives.
In the native application, enter the local server port and click **Connect**.
Local VM controls still target this computer. The new remote mode checks a
verified-HTTPS endpoint with an in-memory bearer token; it hides local controls
to avoid addressing the wrong service. **Load remote workspace** reads the fixed
`GET /api/v1/images` and `GET /api/v1/environments` paths with the retained
connection ID. Lists have independent errors and stale results are discarded on
disconnect. Configured templates are not live-verified; registered environments
are explicitly labelled as having unchecked live state. Confirmed minimum creation is now available in the remote form; other VM controls
and actual remote deployment remain pending. A self-signed/untrusted certificate is rejected; no UI option
disables certificate verification.

## Environment associations

`POST /api/v1/environments` registers a local saved-definition/Proxmox-target
association; it performs no provider request or VM mutation.
`GET /api/v1/environments` and `GET /api/v1/environments/{id}` read the retained
records without provider configuration. See [instance registration](instances.md)
for scope, validation, snapshot and conflict semantics.
`GET /api/v1/environments/{id}/lifecycle-preflight` reads sanitized marker,
lock, template, pending-config and revision checks. It never enables execution;
see [lifecycle preflight](lifecycle-preflight.md).

## VM status

`GET /api/v1/provider/nodes/{node}/vms/{vmid}/status` reads one explicitly
addressed VM and returns sanitized power/QMP observations. It performs no
mutation and does not establish guest readiness or application ownership.
See [VM status contract and verification](vm-status.md).

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

SIGINT/Ctrl-C and Unix SIGTERM request graceful shutdown. Request and lifecycle
events are safe JSON on stderr; see [diagnostics](diagnostics.md) for fields,
fixed startup error codes and recovery behavior. Production service lifecycle and deployment
remain outside this initial slice. `make check-server` builds/tests only the
server package, without requiring native Tauri libraries or frontend assets;
Cargo still resolves the shared workspace dependency lock.

Implementation references: [Axum routing and JSON](https://docs.rs/axum/latest/axum/),
[Axum serving](https://docs.rs/axum/latest/axum/fn.serve.html).

## Confirmed lifecycle requests

`POST /api/v1/environments/{id}/start` and `/shutdown` enqueue fixed lifecycle
operations. They default to HTTP 403 (`lifecycle_disabled`); the server must
explicitly set `VMANYWARE_LIFECYCLE_ENABLED=true`. Browser Origin headers are
rejected and an `Idempotency-Key` is required, as for inventory operations.
Bodies are limited to 4096 bytes and reject unknown fields.

```json
{
  "action": "start",
  "instanceId": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "providerScopeId": "<64 lowercase hexadecimal characters>",
  "node": "pve-lab",
  "vmid": 202,
  "definitionHash": "<64 lowercase hexadecimal characters>",
  "configDigest": "<40 lowercase hexadecimal characters>",
  "confirmed": true
}
```

This is a shape example, not an approved real target. For shutdown, both the path
and action must say `shutdown`; force-stop is unsupported. The body must match the
path, saved association and configured provider scope. `configDigest` comes from
an explicit lifecycle preflight; a stale revision fails in the worker before POST.
The capability flag indicates availability, not authorization or passing checks.
Every action still requires complete explicit confirmation and fresh preconditions.

The handler performs no provider request. It persists queued intent before HTTP
202 with `Location: /api/v1/operations/{id}`. The worker then performs the bounded
checks and dispatch described in [lifecycle execution](lifecycle-execution.md).
Poll the operation detail route; task acceptance and power are separate evidence,
and neither proves guest readiness. An uncertain result never permits automatic
resubmission, even with a new key. A retained task may reconcile through GET only.

Repeating the same key and exact body returns the same operation, including a
terminal/uncertain outcome, without extending approval expiry. Different input,
cross-kind keys or conflicting active/uncertain action returns HTTP 409. A disabled
capability or changed current provider configuration may reject enqueue retries;
GET operation detail/history remain available to retrieve existing outcomes.
Missing instances return 404; invalid confirmation returns 400, malformed/unknown
JSON fields 422, oversized body 413, missing provider/storage 503. Rejections never
include raw provider bodies, credentials or task references.

Generated HTTPS tests exercise local HTTP enqueue -> actual worker -> provider
POST -> persisted task/power result, plus rejection/idempotency cases. A separate generated-HTTPS real server subprocess start/restart test now proves
local process recovery without repeating submission; real Proxmox remains unverified.
Typed native action/read commands compile and pass local HTTP tests. React
confirmation, pending-intent persistence and bounded monitoring pass Node/browser
simulated-IPC tests. An isolated Linux WebKit/Tauri/actual-Axum/generated-HTTPS harness also passes;
normal packaged launch, real Proxmox and other platforms remain separate. See [native command boundary](lifecycle-execution.md).


## Remote create form

The native remote form now offers definition/resources/public-key input, save and
preview, then explicit complete-target confirmation and operation status refresh.
Its fixed native commands retain the prepared plan and one request key, reconstruct
Approval from native state and reuse that same key after an uncertain response.
New preparation is blocked after submission until disconnect; there is no automatic
retry. Disconnection does not cancel work. Success requires scoped clean guest
proof and does not establish SSH access.

This first form retains context in session memory only. Record the request key and
operation ID before closing; the GUI cannot yet resume a submitted operation after
full app restart/reconnect. The server retains its operation independently and its
GET detail route remains available. Native HTTPS tests and browser simulated-IPC
checks pass separately; actual native command launch/mobile and a deployed remote
control plane remain unverified. See [ADR 0044](decisions/0044-remote-create-confirmation.md).
