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

Registered-instance snapshot metadata is available through the read-only
`GET /api/v1/environments/{id}/snapshots` route; the read-only `POST /api/v1/environments/{id}/snapshot-preflight` checks a
bounded disk-only creation request without submitting it. See [snapshot workflows](snapshots.md).

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

## Request admission limits

The production binary shares one request budget across all connections and routes,
including failed authentication: burst 120, refill 20 requests/second, at most 32
concurrent handlers until response headers. Excess requests return HTTP 429 with
`error.code=request_limit_exceeded` and `Retry-After: 1`, before the handler runs.
Wait before retrying. Preserve the original idempotency key/body for an uncertain
mutation; use saved-operation reads when the operation ID is known. A global limit
can also delay health checks. Forwarded IP headers do not grant separate quotas.
State resets on service restart; proxy connection/time/size limits remain necessary.
The current native remote client recognizes HTTP 429 even without the authenticated
response marker, since admission runs before authentication. It displays a fixed
wait/retry instruction without reflecting upstream bodies or arbitrary Retry-After
values. It does not automatically retry mutations or clear their retained request
key/plan/operation. This status is a capacity error, not authentication proof.
See [ADR 0071](decisions/0071-process-wide-request-admission.md).

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

### Native HTTPS connection trust

The native `connect_remote` command accepts an optional `caPem` containing one
public PEM CA certificate, at most 16 KiB. Omission/blank input uses platform
trust. An explicit CA limits the current profile to that CA only, with certificate
chain, validity and hostname verification still enabled; it does not install or
modify device roots. Obtain the public CA through a trusted channel. Private keys,
certificate bundles and malformed/oversized input are rejected. The endpoint is
still HTTPS-only, without a path, query, fragment or embedded credentials.

The certificate and bearer profile remain in memory. The mounted form retains
only the public CA until disconnect/unmount, while the bearer field is cleared
on submission. Disconnect or application process restart requires entering the
connection settings again. No new server endpoint or authentication bypass is
introduced. Android packages include explicit platform-verifier initialization
and its Cargo-matched Kotlin component; see ADR 0050 and the native Android
fixture for verified-HTTPS/read-only recovery evidence and remaining limits.


## Saved snapshot operation recovery

Read-only per-instance snapshot history and operation-context routes are available
at `/api/v1/environments/{id}/snapshot-operations` and
`/api/v1/environments/{id}/snapshot-operations/{operation_id}`. They work with
provider/write capability disabled and never enqueue or resume execution. See
[snapshot recovery](snapshots.md#read-only-server-operation-recovery) for the
historical metadata, isolation, bounds and error contract.


Restore has a separate read-only
`POST /api/v1/environments/{id}/snapshot-restore-preflight` route accepting only a
selected snapshot name. It reports the independent restore capability separately
from eligibility and does not enqueue rollback or power actions.
`POST /api/v1/environments/{id}/snapshots/{snapshotName}/restore` now accepts a
complete target/snapshot approval with explicit data-loss acknowledgement and one
idempotency key, returning a tracked operation. Both route and worker require
`VMANYWARE_RESTORES_ENABLED=true` (default false). See
[restore prerequisites](snapshots.md#read-only-restore-preflight) and
[confirmed restore HTTP behavior](snapshots.md#confirmed-restore-http-behavior).

## Saved restore operation recovery

`GET /api/v1/environments/{id}/restore-operations` returns the newest 100 restore
records for that instance. `GET /api/v1/environments/{id}/restore-operations/{operation_id}`
returns the recorded operation/target/request/fingerprint and submission marker.
These reads require no provider or write flag and never restore approval, claim
work or resubmit rollback. They validate the saved journal and normalize public
evidence. See [restore recovery](snapshots.md#read-only-saved-restore-operation-recovery)
for scope, status consistency, safe errors and historical/current-state limits.

## Read-only deletion preparation

`GET /api/v1/environments/{id}/delete-preflight` reports retained-target identity,
stopped/protection guards and API-visible live/snapshot volume candidates with
storage ownership checks. HA/replication dependencies and replication visibility are checked explicitly.
Fixed cleanup effects and candidate/preserved volume scope are now reviewed; all
checked prerequisites can pass, but execution remains disabled without consent or
a delete journal. It does not delete, change the association or
create an operation. See [deletion preparation](deletion.md).

Environment list/detail `status` is `registered` or `deleted`. Deleted associations
retain definition, YAML snapshot, provider reference and operation history; they
must not be used to address a replacement VM on the same VMID. New operations and
live target resolution refuse deleted identities. A replacement has a new instance
ID. This schema-14 retention contract does not enable deletion submission.

Deletion runtime capability uses independent `VMANYWARE_DELETIONS_ENABLED=true`
(default false; exact lowercase true/false only). The GET delete-preflight report
reflects this flag while prerequisite checks remain separate. The production
worker services retained deletion intents and can reconcile known tasks with the
flag off. HTTP deletion submission and client consent are not connected yet;
query parameters never enable dispatch or purge.

### Confirmed VM deletion

`POST /api/v1/environments/{id}/delete` journals an explicit deletion intent and
returns `202` plus `Location: /api/v1/operations/{operationId}`. It never contacts
Proxmox; the sequential worker independently revalidates consent before its single
provider DELETE. Require configured provider, matching provider scope and independent
`VMANYWARE_DELETIONS_ENABLED=true`. Production bearer authentication applies.

Send exactly one valid `Idempotency-Key`, JSON content type and at most 64 KiB.
Top-level approval fields are `instanceId`, `providerScopeId`, `definitionHash`,
`node`, `vmid`, `configDigest`, `plan`, `confirmed`, `acknowledgeDataLoss`.
The instance ID must equal the path. Build `plan` from the reviewed preflight's
`scopeHash`, `volumes`, `snapshots`, `preservedMedia`, `preservedOwnedVolumes`,
`effects`, without dropping or changing fields. Both consent flags must be true.
Unknown fields, unverified ownership, unsafe volume references or changed fixed
effects are rejected; purge and unreferenced-disk destruction are not options.

The same key with exactly the same full approval returns the original operation
and confirmation time, including after completion/association retirement. Changed
inputs or competing mutations return `409`. Retry does not renew dispatch expiry.
POST still requires current runtime capability/provider configuration; use the
returned GET operation location for reads without preparing another intent.
No definition or association is deleted during enqueue. Failed/uncertain work and
independent successful retirement are reported through the operation journal.

Safe errors: `400` invalid approval/key/path binding, `401` unauthorized, `403`
disabled, `404` missing instance, `409` scope/target/key/operation conflict, `413`
body too large, `415` non-JSON content type, `422` malformed JSON shape and `503`
missing provider/storage. Malformed JSON syntax returns `400`. Errors do not echo
supplied payloads or provider details. A queued response does not mean the VM was
deleted or that storage cleanup succeeded.

### Saved deletion recovery

- `GET /api/v1/environments/{id}/deletion-operations`: newest 100 deletion summaries
  for exactly that retained instance, ordered by creation time then ID descending.
- `GET /api/v1/environments/{id}/deletion-operations/{operationId}`: saved operation,
  target, `configDigest`, complete `plan`, `approvalHash` and `submissionStarted`.

These are authenticated reads, including for deleted associations. They need no
provider or write capability and never claim/re-enqueue/reconcile work or renew
consent. Summary entries contain ID, status, safe failure code, provider-task-known
flag, approval hash and creation time; large plans appear only in detail. No consent
flags, request key or raw provider UPID are returned. Saved scope/digest/evidence
describe the recorded operation, not current infrastructure or a new approval.

Missing instances/history or out-of-scope/wrong-kind operation IDs return `404`;
a known instance without deletion records returns an empty `200` history. Unknown
or contradictory saved evidence returns safe `503`, without partial history or
raw fields. Successful detail requires a retired association and matching full
approval/task/absence/definition evidence. Earlier failed records remain readable
after retirement. An unrelated damaged record stays `404` outside its instance.

## Retired-definition recreation preview

`POST /api/v1/environments/{id}/recreate-plan` accepts bounded JSON
`{"targetVmid":203}`. Browser Origin headers are refused; configured whole-API
bearer authentication applies. The source must be a deleted association.

Returns `sourceInstanceId`, `sourceDefinitionHash`, `sourceDefinitionRevision`,
`definitionYaml` and an ordinary provisioning `plan` from the exact saved YAML
and current template catalog. `requiresNewInstanceIdentity` is true;
`restoresPreviousDiskContents` and `recreationEnabled` are false. The embedded
plan retains false `liveVerified`, `vmidReserved` and `creationEnabled` flags.
A configured provider contributes only its endpoint scope hash, with no network
request. No provider is required for this preview; a template catalog is required.

Unknown/malformed source IDs return 404, active sources 409, invalid target/image/
disk plans 422 and missing catalog 503. Caller-supplied YAML/extra fields are
rejected. This route performs no mutation, reservation or enqueue. Later live
preflight, fresh confirmation and durable recreation submission remain pending.
See ADR 0065.

Recreation storage now supports fresh confirmed provisioning with immutable source
lineage (schema 15, ADR 0066). Confirmed submission is now exposed as documented below; the read-only
preview's `recreationEnabled` remains false. Existing
ordinary creation approvals and operation wire formats are unchanged.

### Confirmed recreation submission and saved detail

`POST /api/v1/environments/{id}/recreate` accepts at most 4096 bytes of JSON:

```json
{
  "sourceInstanceId": "0123456789abcdef0123456789abcdef",
  "approval": {
    "definitionName": "kernel-lab",
    "definitionHash": "<reviewed retained definition hash>",
    "providerScopeId": "<reviewed current endpoint scope hash>",
    "planHash": "<reviewed current catalog plan hash>",
    "targetNode": "pve-test",
    "targetVmid": 203,
    "sourceVmid": 9000,
    "storage": "local-lvm",
    "bridge": "vmbr0",
    "confirmed": true
  }
}
```

All fields are required, extra fields are refused and `sourceInstanceId` must
match the canonical source path ID. Exactly one `Idempotency-Key` is required;
Origin-bearing requests are refused. Configured whole-API bearer authentication
applies. `VMANYWARE_PROVISIONING_ENABLED=true`, a matching configured provider
scope, a current catalog and the supported minimal provisioning workflow are
required. The handler reads the retired source's saved YAML, verifies its hash and
rebuilds/matches the approved plan before atomic enqueue. No provider network
request is made by the handler. A current live preflight and later worker
revalidation are still necessary; enqueue is not readiness or VMID reservation.

Returns 202 and the ordinary `vm_provision` operation with a new instance ID.
`Location` points to
`/api/v1/environments/{sourceId}/recreation-operations/{operationId}`. Same-key
replay retains the same source, operation, identity, consent age and stages.
Changed source/approval or a conflicting active target is refused. An ordinary
creation key cannot be reused for recreation, or conversely. Runtime/scope/catalog
checks apply to HTTP retries too; changing configuration can refuse a retry even
when the original operation remains saved and readable.

The scoped GET returns `sourceInstanceId`, `operation`, the saved `plan`,
`restoresPreviousDiskContents:false` and `requiresNewInstanceIdentity:true`.
It validates the saved lineage/archive/target through the provisioning reader.
Unknown/wrong-source/non-recreation operations return 404. No approval/consent,
request key or raw provider task is returned. This read works without a configured
provider/catalog or enabled write flags, never claims work and leaves the journal
unchanged. Results describe historical recorded evidence, not a current VM read.
Source-scoped history discovery and live preflight are described below.

Generated-provider recreation acceptance now exercises this enqueue route through
the production worker/executor, including optional disk expansion, clean simulated
guest readiness, database reopen after clone, identical completed-request replay
without new provider traffic, and saved GET recovery after another reopen with
provider/catalog/write capabilities absent. The source association is a retired
fixture; this is not a real delete/recreate VM or a server-subprocess proof.

### Source-bound live recreation preflight

`POST /api/v1/environments/{id}/recreate-preflight` accepts the same bounded
`{"targetVmid":203}` body as the read-only preview. It performs the same source
retirement/archive-hash/current-catalog validation before observing the provider.
The caller cannot supply replacement YAML. A configured provider is required;
write execution may remain disabled. No operation or VMID reservation is created.

The response preserves the preview's source identity/hash/revision/YAML, target
plan and fresh-identity/no-old-disk semantics, and adds `preflight`, using the
ordinary provisioning report. Its `planHash` matches the returned plan. Four fixed
provider GETs observe template configuration, storage, network and target VMID
availability, within the existing overall deadline. Failed/denied reads produce
unknown checks without raw provider errors. Known occupied VMIDs fail the target
check. Template image revision, execution permissions and actual guest readiness
remain explicitly unknown; metadata reads cannot prove those gates.

`recreationEnabled`, embedded `plan.creationEnabled` and report `creationEnabled`
reflect current provisioning opt-in, provider scope and supported minimal workflow.
They are capability flags, not a claim that all checks passed or authorization to
skip fresh confirmation. `vmidReserved` remains false, and plan `liveVerified`
remains false. The later worker independently revalidates before dispatch. The
offline `recreate-plan` route still reports execution disabled and makes no provider
request. Whole-API authentication, Origin refusal and the 1024-byte body limit
apply to both read-only POST routes.


Saved recreation detail now fills `plan.providerScopeId` from the original durable
provisioning approval. It describes the recorded endpoint scope even without a
current provider configuration, allowing native source/plan/result matching.
Native retained prepare/confirm/read/clear commands are implemented and locally
HTTPS-tested; the explicit client recreation UI remains pending (ADR 0067).

### Source-scoped recreation history

`GET /api/v1/environments/{sourceId}/recreation-operations` returns
`{"sourceInstanceId":"…","operations":[{"id":"…","instanceId":"…","status":"queued","createdAt":"…"}]}`.
The retained source must be deleted. Its most recent 100 recreations are ordered
by creation time descending, then operation ID descending. Ordinary creation and
other-source jobs are excluded. A valid source with no recreations returns an
empty array; active/malformed/unknown sources return 404. Selected corrupted or
oversized journals return a safe 503 storage error, not partial/empty history.

Provider/catalog/write capabilities are unnecessary. The route does not claim,
renew, submit or reconcile work and makes no provider request. Whole-API bearer
authentication applies. Summary fields expose no key, consent, YAML or raw task.
Choose an operation explicitly before reading its existing scoped detail. Status
is recorded historical state, not an observation of the replacement VM today.
See ADR 0068. Native fixed history/detail GET commands now compile and pass
focused HTTPS tests; client history/restart integration remains pending.

## Saved lifecycle operation recovery

`GET /api/v1/environments/{id}/lifecycle-operations` returns `instanceId` and up to
100 recorded start/shutdown summaries, newest `createdAt` then ID first. Summaries
contain ID, action, status, failure, providerTaskKnown and createdAt. The instance
may be registered or archived. `GET .../lifecycle-operations/{operation_id}` returns
`operation`, historical `action`, historical `target` (instanceId/providerScopeId/
definitionHash/node/vmid), and `submissionStarted`.

These reads require no configured provider/catalog or write flag and install no
consent. They do not read live power/guest state, replay a mutation or return a
confirmed flag, request key or provider task identifier. Retain historical versus
current-state wording in clients. When authentication is configured it covers both
routes. Missing/invalid/unrelated IDs return 404 `lifecycle_operation_not_found`;
a matching damaged journal returns 503 `lifecycle_recovery_unavailable`, including
history containing a damaged entry. See [ADR 0075](decisions/0075-read-only-lifecycle-recovery.md).
