# 0077 — Inspect the retained remote instance definition

Status: native read implemented; UI/integrated runtime acceptance remains open.

## Decision

Add `read_remote_instance_detail(connectionId, target, definitionName)` to the
local-main application capability. Reuse the registered target shape; validate
instance/scope/node/VMID and bounded selected definition name before transport.
Read only fixed GET `/api/v1/environments/{id}` through the active native HTTPS
profile. The existing server route reads stored association/definition data without
provider access or a write flag; no server API change or new dependency is needed.

The response must match selected ID/name/provider/scope/node/VMID. Revision is
positive, status is registered/deleted, and retained YAML is nonempty and at most
64 KiB of UTF-8 bytes. Its SHA-256 must equal the stored definition hash, using the
same exact canonical byte identity as server persistence. Forward only typed
public detail fields, preserving the original YAML rather than reserializing it.
Do not log or echo damaged YAML in errors. Common response/auth/size/429/disconnect
protections apply; no consent, mutation, plan or lifecycle state is installed.

The client checks byte integrity and selection metadata, not the YAML schema anew.
Schema validity/canonicalization remain the trusted server's responsibility. This
read does not prove that current VM resources match the saved declaration, that
an archived VM is absent now, or that the guest/IP/SSH identity is ready. A valid
hash from a malicious trusted server is not independent provenance or authorization.

## Evidence and next slice

Five focused Linux native generated-HTTPS tests pass: registered/archived exact
GET/YAML/revision and extra-field stripping, unrelated metadata/target/status/hash
and changed YAML refusal, empty/oversized UTF-8 byte bounds even with matching hash,
invalid input before transport, auth/marker/status/size failures without token/body
echo, and stale disconnected response rejection. All 76 production commands remain
restricted to local main by the manifest/grant drift check.

This is protocol/compilation evidence, not UI, actual native window, Android or real
infrastructure proof. Next expose explicit retained-definition inspection/refresh in
the remote workspace, with literal YAML, revision/hash and desired-versus-observed
wording. Do not rebuild artifacts until the usable UI/integration slice is complete.


## Read-only inspection UI

RemoteInstanceDetail now appears on registered and archived rows. Reads/refresh are
explicit and send only connection, selected target and definition name. It displays
revision, SHA-256, stored association state and literal YAML through React text
children, with wrapped/scroll-bounded code for small screens. No edit/submit/SSH
control follows from this display. The panel states that this is the saved desired
declaration, not observed VM resources/existence/readiness/identity.

A new read clears previous content. Errors, changed target/association context and
connection cleanup cannot leave stale definition results visible. Hide clears local
content without changing the server. The UI checks selection and bounded metadata;
exact YAML byte/hash verification remains native, without a second YAML parser or
client dependency. Frontend build and focused simulated-IPC evidence are recorded
in STATE.md. Next prove actual native/server stored-definition read against the
production route before Android artifact refresh.


## Actual native/server detail acceptance

The current real React/native HTTPS/server path now passes saved-detail inspection
for registered and archived associations in the second app of the lifecycle and
recreation recovery fixtures. Each actual server was restarted on the same database
with provider/catalog absent and all writes false. The driver explicitly reads,
refreshes and hides the exact association's YAML; visible text/hash/revision/name/
status match the original retained record. Native hash validation therefore executes
on the actual server bytes, not mocked IPC. A public association-ID data attribute
avoids ambiguous VMID-only selectors when recreation reuses a retired VMID.

All existing recovery GET-only, full-operation/identity/definition and provider
counter checks still pass. No provider read/write or changed journal follows from
inspection. Frontend and embedded example builds pass. Evidence is isolated Linux
clean app/server restart, not Android, real VM or a new deployment claim. Next
refresh Android artifacts for the completed client feature; server code is unchanged.
