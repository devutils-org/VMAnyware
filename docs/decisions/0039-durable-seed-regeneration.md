# 0039 — Durable seed regeneration before start

Status: standalone executor/journal tested locally; production start/API pending.

Regeneration is the first substep of the existing start_vm stage. Keep saved plan
steps and hashes stable. Stage results remain reserved for completed stages;
therefore migration 10 adds a small provisioning_seed_regeneration table holding
bounded typed expected/completed reports keyed by operation, without raw seed data.

Claim only after validated clone/resources/optional-disk/Cloud-Init success, with
an undispatched queued/interrupted start stage and no result. Validate the prior
settings result's instance, scope, plan, resources, disk, network, guest user, keys,
stopped state and revision. Reuse existing source validation rather than treating
an arbitrary succeeded flag as proof. The executor requires current capability,
provider scope and matching catalog before preflight or dispatch.

Commit the expected compatible report and start_vm dispatch marker atomically
before the one regeneration PUT. The marker covers the seed substep, not a power
request. Matching safe postflight evidence is stored in the separate journal;
start_vm and the whole operation remain running, with no start task and no ready
claim. A new start executor must use regenerated_seed() as a necessary gate and
perform fresh state checks; it also needs its own durable power-dispatch substep.

Startup recovery preserves journal evidence. Before-marker interruption can be
claimed again. After-marker interruption or ambiguous transport remains uncertain
and cannot be reclaimed for another regeneration, even if previews look correct.
Successful evidence survives restart and exposes the gate; no regeneration call
is needed or permitted again. Failure before marker is terminal without mutation.
Bad/corrupt journals fail closed during reads and claim transactions roll back.

No task identifier is fabricated for this synchronous operation. Neither generated
hashes nor the journal prove attached ISO bytes or guest execution. This supersedes
ADR 0038's absence of a durable caller, but the production worker/API, power-start
journal, strict readiness and mobile runtime remain open.

Evidence: two focused storage tests cover both optional disk shapes, approval and
proof rejection, success and restart before/after markers. A generated-HTTPS test
covers six executor cases including lost response, incompatible seed and disabled/
unconfigured inputs, then restarts with no extra requests. Migration 9->10 retains
settings without inventing a regeneration result; affected older migration checks
also pass. No real provider/VM/credential changes.
