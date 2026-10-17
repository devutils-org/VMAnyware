# ADR 0011 — Prefer explicit agent-backed authentication for the desktop prototype

Date: 2026-10-17
Status: Local feasibility implementation; native authentication UI pending.

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
