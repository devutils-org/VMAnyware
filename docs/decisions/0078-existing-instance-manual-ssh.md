# 0078 — Explicit manual SSH preparation for an existing association

Status: React and actual Linux native host trust, application-key login and terminal pass; Android/real-target acceptance remain open.

## Decision

Registered remote rows expose Prepare SSH. Selection opens the existing host-trust
component with an empty hostname and account, default editable port 22, and the
selected definition/node/VMID as context. It does not inspect a host, choose an
address from power/YAML, unlock a key, authenticate or open a shell. The operator
must supply a reachable LAN/VPN endpoint and independently verify its fingerprint
before choosing an account/identity. Association metadata does not attest that the
manually entered endpoint belongs to this VM; the UI explicitly says so.

Reuse the existing strict unknown/changed-key and fresh-inspection policy. Every
manual reselection mounts a fresh trust context, clearing account/inspection state
and cancelling pending challenges/results through existing cleanup. Native host
trust remains endpoint-scoped rather than being derived from the VM label.
No new native command, provider call, credential mechanism or dependency is added.

Manual entry is disabled while another mutation review is retained. A successful
workspace reload that omits, archives or rebinds the selected association clears
its manual SSH context; explicit disconnect/reconnect also clears it. A temporary
workspace read failure alone does not terminate a direct guest SSH context: failed
control-plane reads do not prove the independently selected endpoint changed.
Ready-guest creation/recreation handoffs retain their existing behavior.

## Evidence and next slice

Frontend build and focused Firefox simulated-IPC flow pass for blank/manual endpoint,
no automatic transport/authentication, invalid URL refusal, explicit unknown-key
consent and fresh verification, changed-key refusal, reselection/late response
cancellation, archival cleanup and reconnect. Existing handoff regression outcomes
are recorded in STATE.md. This is not actual SSH/network/guest or Android evidence.

Next prove manual host inspection/confirmation through the real native command and
an owned generated SSH endpoint, without Proxmox/real VM operations or personal key/
agent access. Preserve independent trust and no-auto-login guarantees; a later full
terminal check remains separate from this entry slice.


## Actual native preparation acceptance

The --manual-ssh owned Linux fixture now exercises actual remote React, verified
HTTPS/control-plane list and the production native SSH host commands against an
in-process generated loopback peer. Endpoint/account start empty. The operator
simulation supplies only the owned address/port, compares the handshake fingerprint
with independently injected fixture metadata, explicitly confirms unknown-key trust,
reinspects freshly and observes changed-key refusal after test-owned rotation.

The peer records zero public-key authentication attempts, channels and PTY/shell/data
events; probe connections must close. Saved public trust bytes are unchanged after
changed-key refusal. The example registers no identity/auth/session handlers or
agent manager and uses only a temporary trust path. Its capability queries now use
the production functions; Linux agent support remains false. No personal files or
agent socket are discovered. The generated peer/rotation/report are example-only.
Control-plane provider read/task/write counts remain zero. This proves native host
preparation/trust, not login, endpoint ownership, real VM or Android behavior.

Next: connect the existing-instance path to an owned actual SSH authentication/
terminal check, retaining strict trust and temporary application identity/agent
boundaries. Keep fresh artifact/platform and real infrastructure gates open.

## Actual native terminal acceptance

The same owned mode now continues through production application-key selection and
session commands. A fresh encrypted IdentityStore is created inside its temporary
directory; only its public key is authorized on the generated peer. AgentManager
uses an explicit nonexistent owned socket, never a personal agent. The UI requires
account, passphrase, connection consent and explicit identity selection before
opening the terminal. Real xterm input executes a shell command; stdout renders,
and `stty size` matches resized rows. Explicit terminal disconnect closes the peer
connection. The subsequent changed-key refusal still preserves stored trust bytes.
Peer evidence requires one channel and PTY/shell/resize events. Native build and
actual integration pass; log `.lab/native-deps/manual-ssh-terminal-check.log`.
This supersedes the earlier host-only mode assertions, retaining its trust checks.
It does not attest VM ownership, real provisioning or Android execution.
