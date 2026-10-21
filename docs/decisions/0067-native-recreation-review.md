# 0067 — Native retained recreation review

Status: accepted for native prepare/confirm/status/clear. Client integration remains pending.

The native bridge exposes `prepare_remote_recreation(connectionId, requestKey,
sourceInstanceId, targetVmid)` and `clear_remote_recreation(connectionId,
requestKey)`. Preparation reads the exact source via fixed GET, then calls only
its fixed `recreate-preflight` route with targetVmid. It never saves a replacement
definition, creates an association or submits an approval.

The source must have the selected canonical ID, deleted status, bounded saved
YAML, nonzero revision and valid hash. The returned review must preserve the exact
source ID/YAML/hash/revision, the ordinary supported creation plan's definition
identity and requested target VMID, matching preflight plan hash, ten unique known
check codes and recognized statuses. It must deny old-disk restoration and require
a new identity. Runtime capability flags must agree and reservation must be false.
Disabled execution and failed/unknown checks remain visible for later UI review;
they do not grant consent. Server check messages/unknown fields are not forwarded.

The normal creation plan validator and bounded authenticated response reader are
reused through module-scoped helpers, without changing ordinary creation behavior.
Current authenticated HTTPS profile, bearer marker, response-size bounds and
verified TLS remain required. Paths and request bodies cannot be supplied by UI.

A per-review epoch prevents a cleared/replaced response from reinstalling state.
Only one read preparation runs at a time. Clearing requires its retained key;
disconnect resets all review state and generation checks discard late replies.
No token, review or consent is persisted. Cancel/disconnect does not mutate server
work; preparation has submitted none.

Five focused HTTPS tests cover fixed paths/body/auth, retained/clear state, changed
source/plan/semantics/check sets, disabled/failed/unknown presentation, active source,
invalid input, missing bearer marker and disconnect while reading. Native Linux
compilation/linking is evidence only, not an actual WebView/client recreation run.

Next: reconstruct fresh confirmation only from this retained review, require its
eligible checks and fresh consent, keep identical key/body after unknown replies,
and validate a distinct new instance plus source-scoped GET operation status.


## Retained confirmation and status

`confirm_remote_recreation` accepts only connection ID, retained key and explicit
confirmation. It reconstructs source ID and the entire provisioning approval from
the retained review. The first seven metadata checks must pass, no check may be
failed, execution capability must be enabled and the first submit must occur
within 120 seconds of preparation. Unknown image/permissions/guest checks remain
explicit limitations of metadata preflight, independently revalidated by the
server worker where supported.

Before HTTP submission the native state is marked submitted/busy. A lost,
malformed or refused response keeps the same key/body and prevents clearing or
replacement. Only explicit same-key retry remains possible; the original review
expiry does not forbid this idempotent lookup/retry. Known operations cannot POST
again. Native accepts only a canonical new instance ID distinct from the retired
source, normal provisioning kind and valid task/guest readiness evidence for a
succeeded operation. The existing ordinary creation evidence validator is reused.

`read_remote_recreation_operation` uses only the retained source-scoped GET path.
Saved source/no-old-disk/fresh-identity semantics, the complete native plan inputs
(excluding current capability) and both operation/instance IDs must remain equal.
A terminal result may be acknowledged with clear; queued/running/uncertain or
unknown submissions remain retained. Busy submitted requests cannot be cleared.
Pending unsubmitted review reads can still be cancelled; epochs discard replies.
Disconnect clears local state without cancelling server work.

The server saved-detail response now supplies the provider scope from its durable
approval, rather than a scope-less rebuilt catalog plan or current configuration.
It still works with no provider configured. Focused API and production worker
journeys verify this historical scope and unchanged restart behavior.

Twelve native focused tests cover preparation plus confirmation/read/clear,
identical retry, expiry/capability/check gates, false success/source-ID reuse,
changed saved plan/operation/instance, concurrent submit refusal and disconnect.
These compile/link and exercise generated HTTPS, not actual WebView/mobile
interaction or real VM recreation. Next: explicit React source/target review,
confirmation/status and existing SSH handoff for the new instance.

## Explicit client review and confirmation

Deleted remote rows now expose a recreation selector. It only opens a form; the
user must choose a target VMID and explicitly request review. The UI displays the
retained YAML/hash/revision, current template/resources/storage/bridge/user and
public-key count, with preflight status and no-reservation caveats. It explains
that fresh provisioning does not recover previous disk contents or snapshots.

Confirmation starts unchecked and is disabled unless the known checks and runtime
capability pass. Source/target/hash/semantic mismatch is rejected before review is
installed. Cancel and reconnect discard consent. Active review/submission prevents
mutation-target replacement and starting ordinary creation. A response without a
verified operation retains the same request and offers explicit same-key retry;
a known operation has GET refresh, without another submit control.

Success requires the same operation and a distinct new instance, matching plan/
provider scope and clean guest/IP readiness evidence. Only then can the new guest
be handed to the existing SSH host-trust setup; no host key is accepted implicitly.
Unknown/uncertain states remain retained, and terminal results can be acknowledged.
Unchanged generation guards discard late prepare/status replies after disconnect.

Frontend build and the focused Firefox recreation scenario pass with simulated
IPC. The affected deletion and restore scenarios are selected because they share
mutation selectors and locking. Actual native WebView/server integration, saved-
source history discovery and real VM recreation remain open.

## Actual Linux native integration

The isolated lifecycle example now registers the production recreation commands
and embeds a real UI driver. `--recreation` uses an authenticated server subprocess,
production worker and the existing generated Android-provider protocol peer via an
owned Node adapter. Native source/target review, cancel/re-review, fresh consent,
lost accepted response with identical API retry, scoped status and new-guest SSH
trust handoff pass. Independent journal/instance/provider assertions require the
retired source/definition/lineage retained, a distinct ready instance, every stage
succeeded and exactly six expected mutations, with no duplicate clone/seed/start.

An initial fixture expected the demo SSH public key's comment; the production
server correctly strips comments before Cloud-Init. The adapter now expects that
canonical public key. The adapter's report pipe also closes explicitly so cleanup
finishes. No production behavior or trust check was weakened. This is actual
Linux WebView/IPC/process/local-TLS evidence, not real infrastructure or SSH proof.
See the lifecycle fixture README for reproduction and remaining platform gates.
