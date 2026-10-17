# ADR 0011 — Prefer explicit agent-backed authentication for the desktop prototype

Date: 2026-10-17
Status: Native macOS authentication check implemented and fixture-tested; manual
walkthrough and real-agent/guest interoperability pending.

Keep the next authentication experiment outside the product until a real agent
can complete SSH authentication with a selected public identity. Use a disposable
OpenSSH process, generated in-memory keys, an explicit private socket and the
existing verified-host-key fixture. No private-key file, personal configuration
or existing agent is needed for this evidence.

The desktop direction is agent-backed signing, with exact public-key fingerprint
selection after host verification. Do not iterate through all identities or fall
back to another key after rejection. Expose no generic sign or forwarding API.
Keep the client private key in the agent; the native client constructs and sends
only the SSH authentication request for the selected endpoint/user/session.

This avoids inventing application private-key file storage or forwarding keys
to the control plane. It does not make every local agent safe or usable: socket
selection, identity-list limits, cancellation, interactive agent approval and
real-agent interoperability still need native integration tests. Access to the
user's configured agent must require a deliberate application action, never
startup or the existing host-inspection flow.

macOS software Ed25519 keys are the proven experimental baseline. Do not infer
Keychain integration, hardware-key behavior, Windows named-pipe support or a
mobile credential strategy from it. Preserve those as separate platform gates
before committing to a cross-platform authentication architecture.

The native bridge now keeps a two-minute, one-use selection in Rust, bound to
its window, endpoint, login name, socket snapshot and listed public keys. A new
listing invalidates the previous selection. Authentication repeats the persisted
host-trust handshake, re-lists the same agent and signs only for the selected key.
Window destruction/cancellation invalidates pending results and prevents a later
signing request; a signature already handed to an agent cannot be revoked.
Operations have a ten-second overall deadline and owned SSH socket shutdown.
There are at most eight concurrent operations and eight window owners, with a
128-identity limit in addition to the library's 256 KiB agent-frame limit.

Resolve SSH_AUTH_SOCK only after a deliberate list action and trusted handshake;
never send its path, comments, private keys or signing payloads to the WebView.
A checkbox grants access for the check, and no identity is preselected. Native
tests inject fresh disposable sockets rather than inspecting the environment.
The existing macOS dependencies suffice; no generic credential framework or
terminal transport was added.
