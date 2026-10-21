# 0076 — Explicit scoped remote power observation

Status: native/React power read and actual Linux integration pass; Android/real-target acceptance remains open.

## Decision

Add `read_remote_instance_status(connectionId, target)` to the local-main command
ACL. Reuse the validated target shape already needed by remote lifecycle review:
instanceId, providerScopeId, node and vmid. A valid target performs only fixed GET
`/api/v1/environments/{id}/status` through the active HTTPS profile. It does not
accept an arbitrary URL, contact a provider directly, prepare consent or enqueue
an observation job/mutation. No current operation or lifecycle review is installed.

The existing server route resolves the registered instance and configured provider
scope. Native additionally checks the returned instance/scope/node/VMID against the
selected row. Power must be running or stopped; QMP may be absent or one of the
server's known normalized states, including paused/unknown. Responses are bounded
to 4096 bytes; authentication, TLS, redirects, request capacity and disconnect
rules use the shared remote reader. Extra response fields are not forwarded.

This is an observation at read time. It does not verify ownership markers, guest
initialization, guest IP, SSH reachability or host identity. In particular running
with paused or unknown QMP is not ready. Saved lifecycle results remain historical
and are not substitutes for this explicit read. A read cannot authorize a later
mutation; start/shutdown still needs its separate fresh preflight/confirmation.

## Evidence and next slice

Five focused Linux native generated-HTTPS tests pass for exact GET/no idempotency
header, running/stopped/paused/unknown semantics, target mismatch/unknown-state
refusal, invalid target before transport, auth/marker/status/size failures without
upstream body/token reflection, extra-field stripping and disconnected replies.
The 75-command production manifest/grant check passes, local main only.

These are native protocol tests, not a new UI, Android or real-provider run.
Next add explicit read/refresh controls and local observation-time/staleness wording
in the remote environment list. On errors/reconnect, do not leave an old power read
presented as current. No automatic SSH action follows from power alone. Refresh
Android artifacts only after the UI/integration feature is complete.


## Read/refresh interface

Remote registered rows now offer explicit Read power/Refresh power controls. The
result labels observed power/QMP and device-clock read completion time, warns it
may already be stale and states that ownership, guest/network/SSH identity remain
unverified. Archived rows expose no live-read control. Loading a workspace does
not trigger status reads.

Starting another read clears the previous result; errors and wrong-target replies
leave power unobserved. Connection/target cleanup and the shared mutation-review
lock invalidate pending and displayed observations. A late pre-action read cannot
reappear after that review is cancelled. No automatic SSH handoff or write follows
from this panel. UI reads do not replace fresh action preflight/consent.

Frontend build and focused Firefox simulated-IPC checks pass. Shared connection/
lifecycle row regression outcomes are recorded in STATE.md. Next run actual native
WebView/HTTPS/provider status integration before refreshing Android artifacts.


## Actual native observation integration

The existing --remote-lifecycle owned fixture now registers the production status
command and exercises explicit reads before/after both confirmed actions. Actual
React/WebKit/native HTTPS/server/provider returns stopped→running→stopped, and the
relay verifies exact selected instance/scope/node/VMID. Read-completion time and
staleness/no-readiness wording are present; action review clears previous power
evidence and disables reads until acknowledgement. Exactly the original one provider
start/one shutdown still occurs despite lost accepted replies and identical retries.
No additional mutation follows from observation. The shared clean-restart recovery
fixture checks that a newly mounted panel does not trigger automatic provider reads;
its completed outcome is recorded in STATE.md. This remains generated infrastructure,
not actual guest reachability, Android runtime or verified ownership.
