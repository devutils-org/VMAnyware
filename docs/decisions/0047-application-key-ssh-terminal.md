# 0047 — Authenticate and open PTY with the encrypted application key

Date: 2026-10-18
Status: Linux native protocol/real-shell and browser tests pass; mobile unproven.

Use ADR 0046's app-owned Ed25519 identity with the existing strict host manager,
window-bound single-use selection and bounded PTY actor. Enable this source on
Unix rather than assuming a macOS Unix agent exists on mobile. The existing
AgentManager name is retained to avoid renaming callers; a concrete internal source
variant distinguishes the configured agent from an application private key.

An explicit list_app_ssh_identity call validates endpoint/username and completes a
persisted-trust handshake before accessing the encrypted key. Decryption runs on a
blocking worker with a zeroizing password, serialized by the same store mutex as
create/unlock. A ten-second caller deadline and owner/generation checks reject late
results. Blocking KDF computation already started may finish after cancellation;
its result cannot install a stale selection. A known/unknown/changed host outcome
is never overridden by authentication consent.

Return only the selected public algorithm/fingerprint and a native-generated
120-second selection ID. Bind the retained key to window, endpoint and username.
Explicit cancellation, replacement, window destruction or expiration invalidate it;
a weak-owner timer also removes an idle expired application selection. Authentication
consumes its ticket exactly once, requires the selected fingerprint and performs a
fresh persisted-trust handshake before signing. No fallback key, password or agent
is attempted for an application selection. Active protocol operations remain bounded
by the existing authentication/session deadline and owned-socket shutdown.

Authentication-only checks close without opening channels. Terminal starts reuse
the existing actor: positive PTY/shell acknowledgments, bounded input/output, owner
checks, resize, explicit close and destruction cleanup. Existing macOS-agent source
remains available in the UI on macOS. Compiling its shared Unix implementation on
Linux does not establish current macOS/mobile runtime behavior.

The UI defaults to the application source, requires account and explicit consent,
clears the passphrase before IPC, and then requires choosing the displayed public
identity. The same selection can check authentication or open a terminal once.
Late unlock/start replies are cancelled/closed after endpoint edits or unmount.
Changing the source/account or revoking consent invalidates the local context.
No software-key/host-key/backend trust is inferred from provisioning's saved IP.

Evidence: three focused native tests use a generated localhost SSH server and
app identities in disposable directories. They prove unknown/changed-host refusal,
wrong passphrase/fingerprint/window, consumed/cancelled/expired selections, server
rejection, authentication without channels and actual /bin/sh output, stty resize,
Ctrl-C/status 130, close and fresh reconnect. Five host-manager and five trust-store
tests plus two PTY-policy tests pass after enabling the Unix source on Linux.
Frontend build and native library/test Clippy pass. Real Firefox/React with simulated
IPC covers consent/unlock/failure, cleared secret fields, explicit identity choice,
authentication/terminal controls, Ctrl-C/disconnect and stale unlock/start cleanup.

No new-command native GUI/IPC, real guest, phone build, OS keystore, keyboard or
suspend/resume proof follows. The existing macOS WebKit fixture was adapted to
choose the agent explicitly but cannot be run on this Linux host. No real VM,
credential or infrastructure was accessed or changed. Next: native mobile build/
launch and the create-to-SSH journey, with platform storage/lifecycle checks and
an explicitly approved disposable real target for final infrastructure evidence.
