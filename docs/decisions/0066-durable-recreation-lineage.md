# 0066 — Durable recreation lineage on the provisioning journal

Status: accepted for storage and confirmed HTTP enqueue/scoped detail;
native/client submission remains pending.

Recreation uses the existing `vm_provision` operation and ordered stage executor.
Schema 15 adds `recreation_sources`, an immutable operation-to-retired-instance
reference. No new provider transport or second execution pipeline is introduced.

`Store::enqueue_recreation` requires a canonical source instance ID, a new explicit
provisioning approval and a request key. Within one immediate transaction it reads
the deleted source's saved YAML/name/hash/revision, rebuilds and verifies the
approved catalog plan, allocates a new registered instance, saves the normal
provisioning intent/stages and records source lineage. Failure rolls everything
back. The prior VMID can be reused because retirement releases only the active
association constraint; normal executor preflight still checks real availability.

The ordinary creation path reads the current definition record as before. The
recreation path reads only the source archive. No consent, request key, instance
ID or provider task is inherited from deletion. Definition data and source history
remain intact. The normal approval hash keeps its existing format; source lineage
is independently bound to the request key by the immutable source relation. Same-
key replay must match both the ordinary approval hash and the exact source (or
absence of source). Ordinary creation and recreation cannot replay each other's
keys, nor can two identical-definition source instances share a replayed key.

Database insert guards require a deleted source distinct from the target, ordinary
provisioning kind and exact saved definition fields. Update/delete guards retain
lineage. Job loading independently revalidates the source/archive/target relation,
including after restart, before the existing worker can act. `Job::recreated_from`
provides the recorded source ID for future scoped API/recovery presentation.

This is storage preparation, not HTTP acceptance or real VM recreation. Next:
confirmed API submission with runtime opt-in, current provider/catalog binding,
minimum workflow checks and scoped saved-operation recovery; then native/client
review and integration evidence. Real provider operations still need concrete
explicit approval.


The confirmed HTTP route now applies the existing provisioning runtime flag,
current endpoint scope/catalog, exact retired archive and minimal workflow checks
before enqueue. Its body explicitly binds source ID and fresh ordinary approval.
The returned scoped Location supports GET-only saved detail after restart without
provider/catalog/write capabilities. Four focused HTTP tests cover queue/retry,
source isolation, restart, runtime/configuration/consent refusal, auth/body limits
and unsupported template-user mapping. This is handler/journal evidence; actual
recreation worker/provider integration and native/client acceptance remain open.

## Generated-provider worker evidence

The recreation API now runs through the production worker/executor against an
owned generated localhost HTTPS provider in `proxmox::tests::create_flow`. Both
8 GiB (no expansion) and 40 GiB (one expansion) scenarios pass. The fixture starts
with a retired source association; it does not execute deletion itself.

Assertions bind the clone marker to the new instance ID, preserve exact source
YAML/hash/revision and the definition record, and require all provisioning stages
plus clean synthetic Cloud-Init/guest-IP evidence before success. Reopening SQLite
after the clone preserves progress. Each expected provider mutation occurs once;
repeating the HTTP request after completion returns the same operation and leaves
the worker idle without provider traffic. Saved source-scoped GET recovery after
another reopen works with no provider/catalog/write capability and returns the
unchanged succeeded journal/source with no provider request.

This runs actual router, worker and HTTPS transport in the Rust test process.
It does not prove fresh server subprocess behavior, real guest state, native/mobile
interaction or real infrastructure recreation. The affected ordinary creation
scenario shares the helper and is rerun with the same two disk-size variants.
