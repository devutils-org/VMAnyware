# 0071 — Bound HTTP admission for the single-operator service

Status: accepted for source implementation; deployment acceptance remains open.

## Decision

The production service applies one in-memory token bucket before authentication,
covering health, unknown routes and failed credentials as well as accepted calls.
It admits an initial burst of 120 requests and replenishes 20 requests per second
using monotonic time. A semaphore permits at most 32 admitted handlers awaiting
responses. Excess work receives HTTP 429, the fixed safe error code
`request_limit_exceeded`, and `Retry-After: 1`; it is not queued and does not enter
a route handler. No request body, credential or forwarding header is retained.

These limits suit one operator and normal operation polling. They are process-wide,
not per-IP or per-token: the service has one operator identity and sits behind a
loopback HTTPS proxy. Using forwarded addresses as trusted client identities would
allow spoofing without a separately established proxy trust model. Fixed-size state
avoids an attacker-controlled map and requires no dependency or configuration knobs.
Router constructors remain usable in isolated component tests; the production
entrypoint explicitly wraps authentication in admission protection.

## Limits and verification

The bucket resets on process restart. Limits are not a network firewall, a full
HTTP connection/body-read timeout policy, a distributed quota or protection against
an attacker occupying the HTTP listener before middleware runs. The concurrency
permit ends when the handler returns response headers; it does not bound streaming
response duration. The HTTPS reverse proxy still needs deployment-specific limits.
A global quota can be exhausted by unauthenticated callers and affect the operator.
Do not expose the loopback listener directly.

Deterministic token-clock tests cover exhaustion, fractional refill and burst cap.
Router tests prove concurrency refusal before handlers, permit release, shared
state across cloned routers, and authentication/fallback ordering. Existing auth
contract tests pass as the affected regression checks. An actual production-entrypoint
debug process on temporary SQLite also rejects an unauthenticated burst with 429
and recovers to 401 after refill, without any provider configuration. No real-provider or platform
acceptance is implied. Clients should wait before explicit retry; a network error
around a mutation still requires the existing identical idempotency key or saved
operation read, never a new unreviewed mutation.
