# 0068 — Source-scoped read-only recreation discovery

Status: accepted for server discovery and native read-only recovery;
client/restart integration remains pending.

The source-scoped saved detail from ADR 0066 is supplemented by
`GET /api/v1/environments/{sourceId}/recreation-operations`. A retained source must
exist and be deleted. Active, unknown and malformed source IDs return 404, while a
valid retired source with no recreations returns an explicit empty list.

The immutable schema-15 source relation selects only this source's most recent
100 operations, ordered by recorded creation time descending and operation ID
descending for ties. Ordinary provisioning and other-source recreations are not
included. Entries contain operation ID, new instance ID, recorded status and
creation time; the response binds `sourceInstanceId`. Request keys, consent,
provider task handles, archived YAML and detailed results are not returned.

Before parsing each selected job, the reader bounds persisted approval, plan,
catalog, target-definition and result payloads, including per-stage results. The
existing provisioning reader independently validates approval/plan/archive/source
lineage. A damaged selected record fails the whole read with a safe storage error;
it is not omitted or presented as empty history. Detailed historical evidence uses
the existing scoped GET route after explicit operation selection.

Neither endpoint requires a provider, template catalog or write flag. They do not
claim queued work, renew confirmation, dispatch, reserve a target or contact a
provider. Reopening the database and reading history/detail leaves the operation,
source and stage-dispatch state unchanged. Authentication remains whole-API bearer
protection. Recorded success is historical; it does not observe a current guest
or imply that the replacement instance still exists or is ready for SSH now.

Focused tests cover source isolation/empty history, restart without provider or
writes, unchanged journal/source, exact newest-100 ordering over 103 source jobs
with unrelated ordinary provisioning present, active/unknown source refusal,
authentication and corrupt/oversized journal rejection. Native fixed GET readers,
explicit client history/manual-ID lookup and actual app/server restart recovery
remain the next integration work; real infrastructure gates remain open.

## Native read-only recovery

The production bridge now registers `read_remote_recreation_history(connectionId,
sourceInstanceId)` and `recover_remote_recreation(connectionId, sourceInstanceId,
operationId)`. Both use only fixed GET paths on the current authenticated verified
HTTPS profile. History validates the source ID, at most 100 unique operation/new-
instance IDs, known statuses, bounded timestamps and descending time/ID ordering.
Empty history is preserved, while invalid data/auth/read failures remain errors.

Detail recovery first GETs and validates the exact retired source archive, then
GETs its scoped saved operation. The plan must bind the archived definition name/
hash and valid target/template/resources; the operation must match the selected ID
and a distinct new instance. Fresh-identity/no-old-disk semantics are mandatory.
Successful historical results use the existing readiness/scope/plan validator.
Disabled execution is acceptable: these are saved reads, never a capability check
or a new allocation. Other saved states remain visible without becoming consent.

No prepared review, request key, operation tracking state or consent is installed.
Unknown server fields, including hypothetical raw key/consent fields, are not
forwarded. Generation checks discard late source/history replies after disconnect.
The archived-source validator and saved wire type are shared with retained native
submission without changing its behavior. No YAML parsing, template query, provider
request, arbitrary URL or new runtime dependency is introduced.

Eight focused native HTTPS cases cover fixed GET/auth paths, no installed state,
nonterminal/failed records, changed source/plan/instance and false success refusal,
history bounds/ordering/duplicates/statuses, empty versus error, active source,
invalid IDs, auth/error redaction and disconnect. Twelve affected retained workflow
checks pass too. This is native compile/link and generated-HTTPS command evidence,
not a client history panel or actual WebView/server restart recovery. Next: explicit
React history/manual-ID lookup with historical/current-state wording.

## Client saved-source panel

Retired remote rows now expose explicit saved-recreation history and a copied
operation-ID form. The panel uses only the native history/detail readers, without
preparing a review, installing consent or generating a new request key. It validates
source/definition/operation/new-instance and matching success evidence before
presenting saved details. Failed/uncertain states remain recorded observations,
and wrong-source/false-success data is refused.

The UI states that current VM existence, guest readiness/address and SSH identity
have not been checked. Recorded initialization never exposes a direct SSH action;
the operator must inspect the current instance separately. The saved plan explicitly
describes fresh provisioning rather than recovered old disk/snapshot contents.

ID edits, disconnect and component replacement invalidate pending replies. Empty
history and read errors remain distinct; reconnect starts without restored ID/result
or consent. Focused Firefox simulated-IPC recovery plus affected recreation/deletion
history scenarios are selected alongside the frontend build. This source change is
not embedded in the previously delivered Android APK snapshot; a later APK rebuild
and exact-artifact runtime run are still required.

## Actual native restart recovery

The owned Linux fixture now completes recreation once and restarts both the native
app and actual authenticated server on the saved database. All write capabilities,
provider configuration and runtime catalog are absent. A fresh native UI explicitly
reads source history, a selected operation, copied-ID lookup and refresh through
the production GET commands. It restores no token/ID/consent and offers no direct
SSH from historical readiness. No application IPC is simulated.

The proxy sees GET only during recovery. The live generated provider's complete
request/error snapshots are identical before and after, and the saved operation,
retired source and new target records are unchanged. Exactly six original provider
mutations remain. Source setup is a retired fixture and the guest/provider are
synthetic; real VM deletion/recreation and current guest state are not claimed.
This proves clean app/server restart recovery on the isolated Linux bench. Fresh
Android exact-artifact runtime remains pending explicit emulator-target approval.
