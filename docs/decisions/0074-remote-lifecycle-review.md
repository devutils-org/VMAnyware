# 0074 — Retained lifecycle review through the remote HTTPS profile

Status: native preparation/clear/confirmation/status implemented; React review/confirmation/status UI implemented; Linux integrated native UI passes; saved-operation recovery and Android/real-target acceptance remain open.

## Decision

Existing local lifecycle commands use the local loopback service. Mobile remote
workspaces must instead use the active native HTTPS profile. Add two application
commands, restricted to local main by the existing command ACL:

- `prepare_remote_lifecycle(connectionId, requestKey, target, action)`
- `clear_remote_lifecycle(connectionId, requestKey)`
- `confirm_remote_lifecycle(connectionId, requestKey, confirmed)`
- `read_remote_lifecycle_operation(connectionId, requestKey)`

Target contains instanceId, providerScopeId, node and vmid; action is start or
shutdown. The native layer validates bounded identifiers before any request, then
reads only fixed GET `/api/v1/environments/{id}` and GET
`/api/v1/environments/{id}/lifecycle-preflight`. No URL or arbitrary request is
accepted from the WebView. Detail must be a registered Proxmox instance matching
the selected target; its definition hash binds the subsequent report.

The report must match instance/scope/definition/marker/node/VMID, contain a valid
configuration digest and exactly the five known unique checks. Native validation
recomputes both aggregated preconditions from power/QMP/check states. Failed,
unknown and execution-disabled reports can be reviewed; none authorizes a write.
Untrusted server check messages are discarded rather than forwarded to the UI.

Retain the review and its monotonic preparation time in connection memory. Replacing
a review clears previous evidence before reading; overlapping preparation is
refused. Clear requires the same key and invalidates outstanding reads. Generation,
epoch and key checks discard stale responses and stop the second read when the
first completes after clear/disconnect. Disconnect clears all review state.
No intent, key or token is persisted; preparation/clear never submit a mutation.

## Evidence and remaining work

Five focused native generated-HTTPS tests cover the exact GET paths/absence of
idempotency header, target/report validation, duplicate checks, inconsistent
aggregates, blocked/unknown/disabled review, keyed clear, overlap and clear/disconnect
races. The shared remote capacity decoder regression also passes. The command
permission drift check now confirms 72 production commands restricted to local main.
These tests compile the native library on Linux; they are not UI, Android or real
provider evidence.

## Confirmed submission and status

The confirmation command requires explicit consent, enabled execution and passed
selected-action preconditions. It builds the approval only from retained validated
review fields, reusing the existing local lifecycle approval/operation validator
through crate-visible types. No WebView-supplied replacement approval is accepted.
A first submission requires a review younger than 120 monotonic seconds. After an
unknown result, explicit same-key/body retry remains possible without re-preparing
or changing inputs. New preparation and clearing nonterminal submitted work are
blocked. Disconnect discards native state but does not cancel server work.

Fixed start/shutdown POST requires 202. Once a validated operation ID is known,
confirmation and status use only GET `/api/v1/operations/{id}` with expected 200.
No idempotency header is attached to status reads. The shared bounded authenticated
response reader handles capacity/auth refusals without reflecting server bodies;
its generic transport error now says submitted action rather than creation.
Validation binds action, kind, instance, result scope/target/task/power and known
operation ID. Queued work cannot already have a known provider task. Uncertain
work and interrupted work with a known provider task remain retained; only safe
terminal evidence allows acknowledgement/clear.

Nine focused HTTPS tests pass across preparation and confirmation/status. Both
actions cover a malformed reply after the complete POST is received, explicit
identical-key/body retry, GET-only monitoring through both entrypoints, rejection
of unrelated results, stale/unconfirmed/blocked inputs, overlap and disconnect.
Three affected local lifecycle and two shared capacity tests pass as well.
No UI, actual native window, Android or real-provider acceptance is implied.

The remote workspace now selects start/graceful shutdown for registered instances,
reviews the retained target/action/power/checks, requires initially unchecked consent,
locks other actions, retries only the same unknown request and explicitly reads known
status. Power success never claims SSH readiness. A fixed native pre-submission
expiry/refusal returns the form to cancellable unsubmitted state; transport/server
uncertainty retains the key. Reconnect discards stale responses and consent.
Frontend build and focused Firefox simulated-IPC flow pass, plus affected parent
recreation/snapshot-create checks. This does not prove actual native UI dispatch.

Next: actual native generated-provider remote lifecycle integration and saved
operation recovery. Refresh Android
artifacts once that usable feature is complete. Existing APKs predate these commands.


## Actual remote native integration

The owned Linux --remote-lifecycle fixture now passes actual WebView/IPC/HTTPS/API/
worker/generated-provider integration for start and graceful shutdown. It loses
each action's first accepted API reply, verifies identical path/key/body retry and
GET status, and observes exactly two provider mutations total, one per action.
Persisted operations independently confirm target/scope/task and requested power;
the registered instance is unchanged. This is generated infrastructure, not a real
VM or Android runtime. The shared snapshot-restore proxy scenario is the affected
regression for the updated loss-injection branch. See STATE.md for its run result.
