# ADR 0010 — Native macOS host-key inspection before authentication

Date: 2026-10-17
Status: Implemented; local verification recorded in STATE.md. Manual native
walkthrough and real-host interoperability remain pending.

The isolated SSH/PTY and persisted-trust fixtures established a concrete desktop
use case. Move the proven trust implementation into the native client and reuse
that source from the isolated experiment. Avoid two independent trust policies
or a generic provider/credential abstraction.

Introduce the pinned russh 0.64.1/ring candidate only for a macOS host-inspection
prototype. This promotes the handshake/trust path, not the full experimental
shell fixture or a production/mobile library decision. Non-macOS commands return
an explicit unsupported result; no cross-platform SSH parity is claimed.

Expose native inspect, confirm and cancel commands. Keep observed keys and their
endpoint binding in Rust. Return only display metadata and a random opaque
confirmation identifier. Bind pending challenges to their originating window,
expire them after two minutes, bound outstanding owners and invalidate previous
observations when a new inspection starts. Confirmation consumes a challenge;
old, cancelled, expired or wrong-window identifiers cannot approve trust.
Destroying a window also clears its owner state before its label can be reused.

Reject unknown/changed keys before authentication. A known key is reported as
trusted only after the full SSH handshake completes successfully, then the
connection is closed without authentication or channels. Unknown keys require
an independently verified SHA-256 fingerprint and explicit first-use action.
Never offer changed-key overwrite through that action.

Store public host trust under a dedicated client application-data child directory,
initialized lazily on the first explicit inspection. Do not import personal SSH
configuration, credentials or OpenSSH known-host files. Run synchronous storage
operations off the async executor and retain the bounded exclusive-lock and
atomic persistence behavior tested in ADR 0009.

The UI hides stale results/errors and clears verification after edits, superseded
requests or cancellation. Its countdown is advisory; Rust owns expiry and exact
key/endpoint/window validation. There is no authentication, private-key handling,
remote shell, guest readiness or terminal command in this slice. Test with local
generated SSH identities and temporary trust paths before real-host use.
