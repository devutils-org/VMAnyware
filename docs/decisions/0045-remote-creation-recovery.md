# 0045 — Read-only remote creation recovery

Status: native HTTPS and browser UI tested separately; native/mobile launch pending.

A disconnected client must be able to inspect an accepted creation without
recreating its original form, approval or idempotency key. Use the currently
explicitly authenticated HTTPS connection. Add only fixed GET commands for recent
operations, one validated operation ID and its retained instance association.
Existing verified TLS, bounded bodies, authentication marker and connection
invalidation rules apply. No arbitrary HTTP path/body is exposed.

Read at most the server's latest 100 operations, filter vm_provision and offer
manual saved-ID lookup for older records. Validate operation identity/kind and
instance association. A successful report additionally needs internally consistent
instance/scope, plan hash, clean Cloud-Init exit 0 and usable IPv4 evidence. Return
only typed fields; omit raw definitions, provider payloads and failure bodies.
Unlike the original prepared-create read, this cannot compare against an original
local plan that no longer exists. It validates retained server evidence instead.

The UI shows retained target and historical guest initialization explicitly.
Neither proves current power, ownership, SSH reachability or host identity. Reads
are manual; disconnect or component removal discards stale replies. Credentials,
YAML, approval and request keys are not persisted. Recovery never submits, retries,
cancels or automatically cleans up a VM. Unknown submissions can be searched in
recent history, but automatic same-key retry after a full restart is not provided.
One submitted intent per native connection remains the current create limit.

Evidence: three scoped native recovery tests cover fresh-session GET-only lookup,
association/result rejection, filtered history/invalid IDs and disconnect during
read. Two affected prepare/ready regression tests pass. Firefox/React simulated IPC
covers reconnect/remount, empty/list/manual recovery, target/result display, no
additional creation submissions and stale reply suppression. Production frontend
build, native library Clippy and changed-file formatting checks pass. This does
not establish actual new-command Tauri IPC, a deployed service, phone or real VM.
