# ADR 0059: Retain destructive deletion scope in the shared journal

Date: 2026-10-20. Status: storage implemented; absence proof and execution pending.

A deletion approval binds instance/provider/definition/node/VMID, current digest,
the reviewed scope hash and the complete normalized public plan. Retain candidate
volumes/effects/source references, snapshot names, preserved media/observed images
and fixed versioned backend effects: reconciliation must remain possible after
the original VM can no longer be read. Both confirmation and explicit current-data
loss acknowledgement are required. Unknown fields, unsafe/unverified/ambiguous
candidates, altered fixed effects, overlapping/unsorted collections and oversized
approval JSON are rejected. Maximum serialized approval is 64 KiB. A real positive
preflight plan passes this contract. The executor must later compare the entire
fresh plan as well as its hash/digest, never trust a hash with substituted contents.

Schema 13 adds vm_delete to shared operation/mutation exclusion. Its migration
retains existing rows and provisioning children with transactional FK validation
and enforcement restoration, following ADR 0055. Active/uncertain provisioning
blocks deletion through the common index; failed provisioning does not trigger the
non-delete lifecycle blocker, allowing future explicitly approved cleanup after
ownership preflight. The failed record is retained. No association/definition is
removed or retired in this storage slice.

Idempotency hashes use a separate vmanyware-delete-v1 domain and include all
confirmed inputs. Same-key retries preserve identity and confirmation age. Exact
approval, running claim and nonnegative age at most 120 seconds are required
before the irreversible dispatch marker. Only qmdestroy on the bound node/VMID
can be retained, never replaced. Unknown marked submissions are not reclaimed;
known tasks can be reclaimed for reconciliation with backoff, never dispatched
again. Exclusive startup recovery preserves uncertain marked work and interrupts
unmarked running work.

Unlike task failure in a reversible workflow, a failed destroy task can leave
partial deletion. Keep it uncertain and retain mutation exclusion. Definite
post-marker failure is allowed only for explicit unauthorized/forbidden refusal
before a task was retained. An uncertain no-task record requires submission-
unconfirmed evidence; task/state failures require a known task. There is no public
success or association-retirement transition yet: add it with independently typed
VM/volume absence proof and durable retained definition/history handling. Generic
read-operation completion cannot bypass the deletion journal.

Eight new storage tests and 67 affected existing storage/migration/preflight checks
pass. No provider DELETE, HTTP enqueue, runtime flag, worker or client controls are
connected. Real cleanup, independent absence, recreation and full roadmap gates
remain open.
