# 0044 — Remote native create confirmation and status

Status: native HTTPS and browser UI tested separately; native/mobile launch pending.

Add fixed prepare_remote_create, confirm_remote_create and read_remote_create
commands to the retained HTTPS connection. No arbitrary HTTP IPC, endpoint, path
or request body is exposed. Prepare validates a bounded server plan/capability,
saves the definition and verifies its name/hash, including a same-hash conflict
lookup. Definition names cannot alter the fixed read path. All responses require
verified HTTPS, bounded bodies and the accepted-auth marker; errors never echo
server bodies or tokens. Existing no-redirect/retry/proxy policy applies.

Retain the plan/request key in native session memory. Confirmation accepts only
that key, connection generation and an explicit true boolean; build the complete
Approval from the retained plan, not frontend target fields. Record submission
before awaiting HTTP, serialize concurrent submission and reuse the same key/body
when the response is uncertain. Block preparing another intent after submission
on this connection. Disconnect invalidates local context; it does not cancel a
server operation. Late replies cannot restore the old session.

Operation reads target only the retained validated ID and require the returned ID
and kind to match. Successful reports must contain matching scope/plan, clean
Cloud-Init exit 0 and a usable IPv4; missing/false readiness cannot display success.
Only safe result fields reach the UI. No raw stdout, credentials or generic provider
payload is returned.

The form collects definition/image/VMID/resources/public keys, with empty packages
and no snapshot. Save/preview performs no VM creation. Review displays server,
node/VMID, template/version, storage, bridge, resources, guest user and key count.
A separate checkbox enables the create button. Uncertain submission offers a
same-key retry. Manual status refresh distinguishes guest initialization from SSH
reachability/host-key/authentication proof. Tokens are never persisted.

Limitations: one submitted intent per native connection; pending context is memory
only. ADR 0045 now adds read-only GUI inspection after restart/reconnect, without
restoring approval or automatically retrying submission. No automatic polling or
remote SSH control is claimed.

Evidence: six focused native cases cover HTTPS plan/save/same-hash conflict,
confirmation-only fixed POST and approval/key binding, safe failures, success
validation, exact operation reads and disconnect during submission. The affected
connection-generation case passes. Real Firefox/React with simulated IPC covers
form review, explicit confirmation, lost-response same-key retry, status, local
control isolation and stale results after disconnect. Production frontend build,
native library Clippy and formatting/diff checks pass. These do not prove an actual
new-command Tauri IPC launch, deployed control plane, VM, SSH or mobile runtime.
