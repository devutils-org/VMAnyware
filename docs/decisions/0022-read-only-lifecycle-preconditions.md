# ADR 0022 — Observe lifecycle preconditions before preparing writes

Date: 2026-10-18
Status: Implemented with local HTTPS fixtures; real-host validation pending.

A local association and power observation do not prove that VMAnyware may mutate
the target. Add a read-only, endpoint-scoped lifecycle preflight. Require the
sole case-insensitive VMAnyware-prefixed tag to exactly equal
`vmanyware-<instance-id>` in canonical lowercase. No marker is installed by this
slice; setting tags remains a separate provider mutation. Missing, foreign,
duplicate or ambiguous markers fail this check.

The marker is an administrative declaration, not a secret, security credential
or independent ownership proof. A clone may retain tags, a VMID may be reused and
an endpoint may be replaced. Future provisioning must set fresh target markers;
future execution still needs concrete authorization, current identity checks,
serialization and durable action/task reconciliation. No write is enabled here,
even if every reported precondition passes.

Read current config (`current=1`), pending changes, power/QMP status, then current
config again under a 20-second overall deadline. Retain only the provider config
digest, marker match, template/lock flags and pending-change presence. Reject
missing/malformed metadata. Pending deletes 1 and 2 both block actions. Unknown
QMP state never passes a shutdown precondition. Compare digests and marker/lock/
template checks at both ends; expose failed/unknown conditions instead of silently
repairing configuration or selecting a different target.

Matching digests detect observed revision changes, not every possible race or
an atomic lock on a subsequent action. They are revision metadata, not ownership
credentials. A later executor must freshly recheck under its own explicit policy;
a preflight response never authorizes or reserves a mutation.

Keep the provider client read-only and use existing verified TLS/deadline/body
limits. No schema, crate or runtime dependency is added. Raw tags, descriptions,
lock text and configuration/pending values must not be exposed or logged. The
API reports sanitized checks with `executionEnabled: false` and retains no
ownership/ready status in the registered instance.
