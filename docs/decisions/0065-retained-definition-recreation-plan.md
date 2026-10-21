# 0065 — Plan recreation from a retired instance

Status: accepted for read-only planning; confirmed submission remains pending.

Recreation provisions a fresh VM from a retained definition. It does not restore
previous disk contents or reuse deletion consent, an old instance identity or an
old provider task. An active instance cannot be used as a recreation source.

`POST /api/v1/environments/{id}/recreate-plan` accepts only `targetVmid`. It reads
the deleted association's exact saved YAML and verifies its recorded definition
hash, rather than accepting replacement YAML or looking up the latest definition
by name. The current explicit template catalog produces the ordinary deterministic
provisioning plan. A catalog revision can change template/storage/network mapping;
those inputs must be reviewed again before any later submission.

The response binds source instance, definition hash/revision and saved YAML to the
plan, explicitly requires a new instance identity and denies previous-disk
restoration. It does not contact the provider, reserve a VMID, create an operation,
change the source association or enable recreation/creation. The retired VMID may
be proposed, but availability must be independently checked before dispatch.
Changing provider scope is a new target choice, not proof of old VM ownership.

The next slice must bind source lineage to fresh provisioning consent and durable
idempotency, preserve the source record and allocate a distinct new identity.
Existing confirmed creation execution and recovery should be reused where their
contracts apply. Planning alone does not satisfy roadmap recreation acceptance.

## Source-bound live review

`recreate-preflight` now reuses the exact same source/archive validation and
catalog plan builder before the ordinary read-only provider preflight. It returns
the source-bound plan plus matching-hash checks, never caller YAML or a reservation.
Configured execution capability is separate from check status. An occupied target
fails its check even when execution support is enabled; denied reads are unknown.
The offline preview remains unchanged. This prepares native retained review;
it does not replace fresh confirmation or worker revalidation.

Three generated-HTTPS cases verify four GETs only, both runtime-flag states,
source/plan identity, no journal/association changes, busy target and redacted denied
reads. HTTP cases verify active/missing source, input/Origin refusal and provider
configuration requirements. Real-provider and native/client evidence remain open.
