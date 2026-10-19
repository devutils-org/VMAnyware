# ADR 0034 — Prioritize the first usable mobile journey

Date: 2026-10-18
Status: User-directed implementation priority; no mobile support claim.

The user asked when the app would let them create a VM and connect by SSH on a
phone, and challenged progress that did not deliver that journey. Make it the next
product milestone instead of waiting for snapshots, recreate/delete, full release
packaging or every platform. Preserve the full roadmap as the subsequent objective.

Deliver the smallest supported journey: connect the phone to the control plane
securely, select a prepared template/definition and resources, explicitly confirm
creation, follow actual provisioning/readiness and open a usable verified-host SSH
terminal to that VM. Validate on one chosen mobile platform first. A platform
preference is pending; independent backend integration can proceed meanwhile.

Current blockers to this journey are concrete: creation stages are not yet exposed
as a complete production workflow; the service is loopback-only without remote
client authentication; SSH authentication currently uses a macOS Unix agent. The
mobile application build, credential backend, terminal runtime and real journey
have not passed. Existing frontend or desktop/provider tests do not establish them.

ADR 0019's macOS-first priority is superseded for upcoming work. Its technical
warnings and evidence standards remain valid: select mobile key access/storage,
verify host trust, compile and run on the actual target and exercise terminal input,
resize/reconnect/cleanup. No desktop agent reuse or mobile feasibility is implied.
Secure remote access belongs to this milestone, not merely later release hardening.

Reuse verified stage code and keep checks scoped to each diff. Prioritize missing
user-visible integration over additional standalone helpers unless a concrete
invariant blocks that integration. Real VM or credential mutations still require
explicit approval of their target and action; local isolated fixtures stay safe.
