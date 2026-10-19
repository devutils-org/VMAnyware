# Lifecycle execution implementation boundary

Fixed provider transport, durable storage and a lifecycle executor are implemented.
Confirmed HTTP enqueue and typed native action/read commands are implemented;
native confirmation controls and bounded monitoring are implemented with simulated-IPC browser evidence. The
production worker disables new writes unless VMANYWARE_LIFECYCLE_ENABLED is
explicitly true. Local tests exercise this capability; no real VM was changed.
Lifecycle preflight reports configured execution availability, defaulting to false;
it never authorizes or confirms an action.
No real infrastructure operation is evidenced by the local tests below.

## Fixed provider requests

The concrete client has one typed lifecycle transport accepting a validated
`VmReference` and `Action::Start` or `Action::Shutdown`. Callers must first check
capability, confirmed target/revision and fresh preconditions, then commit the
store dispatch marker. The transport itself is not an authorization boundary.
The executor enforces these steps; its default service caller disables writes.

Start sends `POST /api2/json/nodes/{node}/qemu/{vmid}/status/start` with an empty
form body. Shutdown sends the corresponding `/status/shutdown` request with
exactly `timeout=60&forceStop=0`. There is no force-stop, skiplock, user-supplied
parameter map or arbitrary path. Both endpoints return a task reference; task
acceptance does not prove action completion or guest readiness.

The shared client explicitly disables transport retries for all requests,
refuses redirects/proxies and verifies TLS certificate/hostname. The existing
15-second request timeout and 2 MiB response limit also apply to POST replies.
A reply must contain a string UPID matching the requested node, `qmstart` or
`qmshutdown` worker kind and canonical decimal VMID. Invalid or missing identity
is an unconfirmed submission, never permission to repeat the POST. Raw UPIDs and
provider bodies are not included in public operation responses or safe errors.

Only explicit HTTP 401/403 responses represent definitive authorization
rejection. Timeout, connection failure, non-success response, malformed body,
wrong task identity or response-size failure can occur after acceptance; the
future worker must retain the durable uncertain outcome rather than retry.

[Upstream QEMU API source](https://github.com/proxmox/qemu-server/blob/master/src/PVE/API2/Qemu.pm)
was checked on 2026-10-18 for fixed POST paths, form parameters, `qmstart` and
`qmshutdown` task kinds and `VM.PowerMgmt` permission checks. This is an upstream
contract observation, not verification of installed-provider compatibility or
least-privilege ACLs on a real target.

## Local evidence and remaining integration

Generated localhost HTTPS fixtures check exact methods, paths, authorization,
content type and bodies for both actions. Negative cases reject wrong node,
action and VMID (including leading-zero aliases), null/object task replies,
untrusted certificates, permission failures, redirects, timeout and oversized
responses. Fixtures observe no second request after invalid/error replies.
The explicit retry policy additionally covers client-internal retry behavior;
these HTTP/1.1 fixtures do not simulate HTTP/2 protocol retry signals.

The store tests separately cover committed dispatch guards, persisted private
task identity, restart recovery, known-task reconciliation claims and unknown-task
exclusion.

## Executor evidence

The executor claims one lifecycle job, rejects disabled new writes, checks current
provider scope and saved target, rereads lifecycle preconditions and compares the
confirmed digest. It commits the dispatch guard (including approval expiry check)
before POST, persists the returned task before polling, and checks power separately.
Task polling uses 250 ms intervals and a 20-second overall deadline. A failed task
is terminal; unconfirmed submission, task or resulting power remains uncertain.
No SSH or guest readiness is claimed.

Six HTTPS/SQLite executor tests cover enabled start and graceful shutdown,
default-disabled/scope/revision/expiry/foreign-marker zero-POST rejection, task
failure, invalid submission reply and power mismatch. An async worker is aborted
after the fixture receives POST, then after task linkage/first task GET; SQLite is
reopened and explicit startup recovery runs. Neither case replays POST. Known
tasks get only GET diagnosis even with writes disabled; unknown tasks retain their
exclusion. A separate seeded post-link crash boundary reconciles successfully via
two GETs. These are local worker/connection restart tests, not process kill or real
provider tests. A generated-HTTPS real binary lifecycle flow is now tested below.

The service worker processes at most one read and then one lifecycle job per
round, sequentially. Neither queue can indefinitely starve the other; a bounded
provider request still delays subsequent claims. Idle rounds wait 250 ms.

`VMANYWARE_LIFECYCLE_ENABLED` defaults to false. Only exact lowercase `true` and
`false` values are accepted; empty, whitespace-padded, numeric and other values
stop startup with a fixed diagnostic before database creation. Explicit true
allows the executor to process previously confirmed, still-valid queued intent;
it does not create intent or relax scope/revision/marker/expiry checks. Known
tasks reconcile by GET with either setting. The confirmed HTTP routes use the same runtime capability as the worker.
Preflight reports this capability without granting approval; the renderer and
native read command accept both boolean values and expose no action control.

Tests verify both queues complete in one round under both capability settings.
Real cleared-environment binary tests verify absent/false versus true reaches the
worker using preseeded SQLite intent and no configured provider; no dispatch marker
is acquired. Invalid-value subprocess tests verify no database or lock is created.
Those capability-only subprocess tests do not exercise provider POST or lifecycle
restart recovery; the separate end-to-end test below does.
HTTP enqueue and in-process HTTPS worker integration now pass. Next: HTTPS-backed
native explicit confirmation/monitoring UI after the binary evidence below.
See [ADR 0023](decisions/0023-durable-lifecycle-execution.md).


## Confirmed HTTP enqueue evidence

Two local HTTP/TLS tests verify default-disabled and missing-key/origin/provider
rejection, full target/action binding, unknown/oversized fields, same-key identity,
changed-input/cross-kind conflicts and active-action exclusion. Enqueue makes no
provider request. A real executor round then completes the accepted start against
the generated HTTPS peer with exactly one POST. Enabled preflight explicitly
reports capability true. Defaults still report false. Native preflight contracts
and renderer validation now accept either boolean; this is not a confirmation or
automatic mutation trigger. No native lifecycle write command is added.


## Real binary lifecycle evidence

A cleared-environment server subprocess test uses generated localhost HTTPS,
a fake token and a temporary database/CA. It saves a definition, registers an
association, explicitly reads a passing capability-enabled preflight and enqueues
a fully confirmed start. The normal case persists successful task and running
power evidence; repeating the key returns the same terminal operation.

Two further cases hold one provider reply: the POST response before the server
receives the UPID, and the first task-status GET after SQLite linkage. SIGTERM
exits successfully; the database retains a dispatch marker in both cases and a
task reference only in the second. On actual binary restart, unknown submission
remains uncertain with same-key identity and fresh-key conflict. A 600 ms window
(more than two idle worker intervals) observes no provider request. Known tasks
reconcile successfully by GET with new writes explicitly disabled. SIGINT exits
successfully after both restart cases. Every scenario observes exactly one POST;
the registered association remains identical and logs omit token/raw UPID.

This proves local application process exit/restart with simulated provider state.
It does not test SIGKILL/power-loss fsync durability, real Proxmox, token ACLs,
guest readiness or native application controls. Shutdown transport/executor is
covered separately by the in-process TLS tests; this subprocess scenario starts
a fixture VM only. No real infrastructure is accessed.


## Native command boundary

`begin_lifecycle_action(port, approval, requestKey)` accepts only the full typed
confirmation and a bounded safe key. Rust rejects invalid target IDs, scope/hash/
digest format, node, VMID, unconfirmed input and unknown action/body fields before
transport. It sends a single fixed start/shutdown POST to IPv4 loopback with the
original key and exact JSON. The native HTTP client explicitly disables retries
and redirects; it never accepts an arbitrary service URL or provider credential.

`read_lifecycle_operation(port, id, approval)` resumes only through GET using the
original target context. Neither write capability nor a renewed confirmation is
needed for this read; the prior typed approval supplies identity, not permission
to dispatch again. Every response validates operation ID/kind/instance and state/
result/failure consistency. Success also requires retained task identity, successful
task status, matching scope/action/node/VMID and desired power. Unknown failure
text is rejected rather than passed to the renderer. No guest readiness is inferred.

Three Linux native HTTP tests verify exact request paths/body/key, both actions,
GET-only uncertain/success resumption, mismatched identity/evidence, pre-transport
input rejection and fixed safe lifecycle errors. The signed-package temporary
prefix compiles the registered Tauri commands and runs these tests. No native
WebView/GUI lifecycle action is claimed: React confirmation/monitoring is verified
separately through the browser fixture below. Transport loss must preserve the original key; it
cannot authorize creating another request automatically.

## Client monitoring controller

The standalone TypeScript lifecycle controller is implemented and tested but is
connected to React controls with the browser evidence below. `confirmedLifecycle` builds a fresh request
only after an explicit caller confirmation; it validates report identity, enabled
capability, all common checks and action-specific power/QMP preconditions. It
binds the exact saved instance/scope/node/VMID/definition/digest, port and key.
It makes no network call by itself.

`monitorLifecycle` requires successful persistence of that complete request before
transport. An unknown operation ID uses the original confirmed body/key; a known
ID uses only GET. Acknowledged IDs persist even after the view becomes inactive,
while stale results cannot update the view. Success requires bound task/power
identity; unknown failure text and inconsistent results are rejected.

Polling pauses after 60 seconds measured with a monotonic clock, plus at most one
bounded in-flight native request (25-second timeout). Leaving the view stops
subsequent reads without cancelling the durable server action. An uncertain
result returns distinctly and retains its original context/ID; explicit monitoring
resumption uses GET. Transport loss retains the same request, never silently
changes its key, and cannot trigger a new action automatically. Terminal records
also remain available for explicit UI review; this controller never clears intent.

Seven Node tests cover confirmation blockers, persistence-before-dispatch,
request binding, loss and same-key resume, known-ID/uncertain GET resumption,
deadline/leave behavior, late acknowledgements and invalid evidence. The client
build type-checks this module. TypeScript permits explicit `.ts` import extensions
under its existing no-emit mode so the same sources run directly in Node tests;
no runtime dependency or lock changed. React persistence, explicit confirmation
controls and their browser walkthrough are now covered below.


## React confirmation and resume controls

Each registered instance displays lifecycle review controls only after an enabled
passing preflight; unavailable actions remain disabled. Review shows the action,
local service port, saved instance/name, node/VMID, provider scope, definition hash
and configuration digest. A separate initially unchecked confirmation is required
before dispatch. Cancellation creates no request. Graceful shutdown is named
explicitly and never offers force-stop. Changing the report invalidates review.

Pending context persists under a port/instance-scoped localStorage key before the
native command runs. Missing/invalid/unavailable storage blocks new confirmation.
A saved request blocks fresh actions; reloading/remounting exposes explicit resume.
Before saving, the component checks that an existing request has the same key and
approval. This is a local collision guard, not a cross-window transactional lock;
server uniqueness/confirmation/preconditions remain authoritative.

Unknown operation IDs explicitly describe that resume retries the original
confirmed request/key; known IDs use read-only monitoring. Outcomes render fixed
local text. Uncertain results expose no acknowledgement/new-action path, while
succeeded/failed/interrupted results require explicit acknowledgement to clear
the local pending record. Acknowledgement also invalidates the preflight so the
next action needs a fresh check. Server history is never deleted. Invalid saved
state remains blocked for manual review rather than being silently discarded.

The disposable Firefox fixture now exercises review/checkbox/cancel, exact target
details, start success, persisted lost-reply same-key resume after switching ports,
uncertain GET-only reconciliation, late ack retention without stale display,
acknowledgement/fresh-check requirements and graceful shutdown/stopped power. All
commands are simulated IPC; no native WebView or provider is invoked by that
walkthrough. Isolated Linux native IPC/GUI integration is now covered below.


## Real Linux WebView/IPC evidence

The opt-in [native lifecycle harness](../experiments/lifecycle-native/README.md)
now passes with real built React assets, Tauri/WebKitGTK and production Rust
commands. It starts an actual Axum process against a generated HTTPS peer, drives
review/cancel/unchecked consent/confirm for start, checks task plus running power,
acknowledges, rechecks and confirms graceful shutdown to stopped power. Exactly
one start and one shutdown POST are observed; association and redacted-log
assertions pass. There is no invoke replacement.

The run uses owned Xvfb/Mesa display, generated X11 auth, isolated XDG/D-Bus, hidden
existing home directories and read-only verified dependency overlays. No system
package, sandbox/trust bypass, personal profile or real VM is involved. The
fixture-specific example/identifier is distinct from normal packaged startup.
This proves isolated Linux native lifecycle UI/IPC, not real Proxmox, guest
readiness, hardware rendering or other desktop/mobile platforms.
