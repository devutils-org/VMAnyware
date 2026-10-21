# ADR 0063 — Retained native deletion consent and full-plan result binding

Date: 2026-10-21
Status: Accepted for native command preparation/compilation and local HTTPS tests.

## Decision

Native preparation reads the retained registered instance and its fixed GET
preflight on the current authenticated HTTPS connection. It binds instance,
definition, provider scope and node/VMID; validates the complete bounded cleanup
plan, fixed effects and exact prerequisite set; then retains one review and
request key in memory. Unknown effects, unsafe/overlapping volume IDs, incomplete
passed reports and deleted targets cannot become eligible consent.

Confirmation accepts only the retained key plus independent confirmation and
permanent-data-loss acknowledgement. It cannot take a replacement target or plan
from IPC. A first submission needs an eligible review prepared within 120 monotonic
seconds. Mark submission in memory before sending. A lost/unverified response
keeps the original key/body and blocks replacement/clearing; an explicit same-key
retry remains possible without silently constructing a new consent. A known
operation ID only follows its fixed GET path, even when write capability is off.
Failed destroy tasks and incomplete cleanup remain uncertain and cannot be cleared
as terminal failure. Disconnect clears local consent, not server work. Generation
and preparation revision checks suppress outdated replies.

Success additionally requires the exact instance/scope/node/VMID, known provider
task, all absence/retirement/definition-retention evidence and the full approval
hash. Add a direct portable `sha2 = 0.10` dependency for this working use case; the
version already exists in Cargo.lock. No new resolved package/version is introduced.
Typed approval/plan/effect field order follows the server's versioned SHA-256
serialization contract. The same checked-in JSON and expected hash are tested in
native and server code, covering live/snapshot/fleecing/preserved scope. The example
contains only synthetic IDs and no credentials.

## Evidence and limits

Fourteen focused native tests pass, including shared hash/JSON, dual consent,
fixed paths/full body, unknown same-key retry, real socket loss, stale preparation,
disconnect during submit/read, scope/ownership/effect/power guards, expiry,
auth/status/response bounds, exact result evidence and GET-only partial cleanup.
Server contract test passes. Five affected workspace/restore HTTPS tests pass after
native state/handler/dependency changes. The actual Linux native crate compiles
and links using the existing signed dependency prefix; no GUI or mobile launch
or real deletion is established by these tests.

Next: explicit client review/dual-consent/status controls, then scoped read-only
history/recovery and actual native end-to-end evidence. Keep real mutations gated
by approval of the concrete disposable target/action.

## Client review integration — 2026-10-21

The remote React client now exposes explicit registered-row selection followed by
native preparation, complete public deletion/preservation scope and fixed effects,
and two unchecked consent controls. Unknown submissions retain their key/body;
known IDs only refresh, partial failures cannot be cleared and verified retirement
removes old-identity actions. Acknowledging the result does not reactivate the row.
Mutation selectors share a lock; disconnect/reconnect suppresses stale replies and
never restores consent automatically. The renderer consumes native-validated hash
evidence rather than implementing another approval serialization/hash algorithm.

Frontend build, a dedicated Firefox simulated-IPC deletion scenario and affected
snapshot-create/restore scenarios pass. One parallel restore timeout was followed
by a passing exact sequential rerun; its cause remains unconfirmed. No native GUI
launch, saved-history recovery or real infrastructure acceptance is claimed.
