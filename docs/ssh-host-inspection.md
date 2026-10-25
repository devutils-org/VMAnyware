# Native SSH host-key inspection

This macOS prototype inspects a server host key and manages explicit first-use
trust. It does not log in, load client credentials, open channels or run remote
commands. The local control plane and Proxmox configuration are not involved.
A native application build is required; browser preview cannot use these commands.
The implementation is gated to macOS; other native platforms are unsupported
and have not been compiled or tested for this feature.

## User flow

Enter the intended DNS host or IP address and SSH port (default 22), then click
**Inspect host key**. This is the only action that initiates a network probe.
Use a disposable endpoint that you have selected for testing.

The result displays the canonical host, port, key algorithm and SHA-256
fingerprint. It has one of three states:

| State | Meaning | Available action |
| --- | --- | --- |
| Unknown | No key is pinned for this exact endpoint | Compare fingerprint independently, then explicitly confirm or cancel |
| Trusted | The actual key matches persisted trust and a verified SSH handshake completed | Review; no authentication or terminal is started |
| Changed | Another key is pinned for this endpoint | Stop and investigate; no overwrite/reset action |

For an unknown key, obtain the fingerprint from a trusted console or another
established channel. Inspecting the same untrusted network twice does not
independently verify the host. Confirm the verification checkbox and click the
explicit trust button only after comparing the fingerprint. A successful
confirmation saves the observed key; it does not establish a new SSH session.
Inspect again to verify a fresh handshake against that key.

Editing host/port, starting a new inspection or cancelling clears the displayed
result and verification choice. Responses that arrive for superseded inputs or
request generations are hidden. A two-minute confirmation countdown disables
expired approval; the native side remains authoritative for expiry. A changed
key must not be approved as another first use. Legitimate rotations, such as a
recreated VM on the same address, use the explicit replacement flow (ADR 0093). Once explicit
confirmation has consumed its challenge, later edits or navigation do not undo
that approved write; cancellation discards pending approvals only.

## Native boundary

The native commands are:

- `ssh_host_support`: report whether this platform implements the prototype.
- `inspect_ssh_host({host, port})`: return display metadata and status, plus a
  confirmation ID only for an unknown key.
- `confirm_ssh_host({challengeId, fingerprint})`: consume an exact observation
  bound to its originating window and persist its approved key.
- `cancel_ssh_host({challengeId})`: discard a pending observation.

The WebView never supplies the public key to store. Rust retains the observed
key, canonical endpoint and window identity, with a random opaque identifier,
two-minute lifetime and bounded outstanding state. A new inspection invalidates
that owner's older pending observation; out-of-order completions cannot replace
a newer result. Destroying the native window discards its pending state, so
reusing a window label cannot revive an old confirmation. Replayed, cancelled, expired and wrong-window confirmations
fail. Concurrent persistence rereads current records, so a stale challenge cannot
overwrite a different key accepted by another process.

Endpoint validation precedes probing. DNS names are ASCII/lowercase; trailing
dots, URLs, userinfo, paths, invalid labels and port zero are rejected. IP
addresses normalize without merging DNS aliases or distinct ports. SSH
certificates are unsupported. Probes have an overall deadline, including DNS
and SSH connection work. Unknown and changed keys are rejected during key
exchange, before any authentication. A known-key result requires the handshake
to complete; simply receiving a familiar public key is insufficient.

## Local persistence and verification limits

The manager uses a dedicated `ssh-host-trust-v1` child of the native client's
application-local data directory. It creates that store lazily, without probing
or reading personal SSH files at application startup. Only public host keys and
endpoint metadata are stored. The [persistent trust policy](ssh-host-trust.md)
describes corruption handling, bounded locking, atomic updates and filesystem
assumptions. Missing/corrupt existing stores fail closed instead of resetting
trust. Errors use fixed application text without raw transport/configuration.

Automated verification uses generated localhost SSH identities and temporary
injected storage paths. Native compilation and command tests do not demonstrate
a manual walkthrough, interoperability with an actual guest or OpenSSH, mobile
support, platform private-key storage or an integrated terminal. See `STATE.md`
for the exact evidence from this slice and the next task.
