# macOS SSH-agent authentication

The native application now offers an authentication-only check after SSH host
verification. The separate host-inspection action still never opens an agent or
authenticates. Both the native path and isolated experiment are tested against
disposable macOS OpenSSH agents, not the user's agent.

## Native workflow

1. Inspect the endpoint; independently verify and explicitly trust an unknown
   fingerprint. Inspect again to complete a fresh verified handshake.
2. Under **Connect with an SSH agent**, enter the login name and allow access to
   the configured agent. Click **List agent identities**.
3. Choose one displayed SHA-256 fingerprint, then **Check selected identity**.
   Success closes the connection without opening a terminal. Retry requires a
   new identity listing; a rejected key never falls back to another identity.

The application reads SSH_AUTH_SOCK only after the explicit listing action and
a completed persisted-trust handshake. A GUI launch may not inherit your shell's
agent configuration; start `make dev` from a shell with a configured agent if
needed. The socket must be an absolute path. No personal SSH config, key file or
known-host file is imported, and the control plane is not involved.

Rust binds the listed keys, socket, endpoint and username to a two-minute,
one-use window selection. Authentication repeats host verification and checks
that the selected key remains in the same agent. Ed25519 software keys are the
tested baseline. Only algorithms and fingerprints reach the WebView. There is
no generic signing API or agent forwarding.

New listings invalidate earlier selections. Edits/unmount cancel a known
selection; window destruction clears its state. Cancellation discards late
results and prevents signing that has not yet started. It cannot revoke a
signature already requested from an agent. Pending I/O remains bounded by a
ten-second overall deadline; owned SSH sockets close on every exit. Limits are
eight active operations/eight owners and 128 identities per bounded agent reply.
The UI countdown is advisory; Rust enforces expiry.

No agent access occurs on application startup or merely inspecting a host. A
manual walkthrough with a deliberately selected real agent/guest is pending.

## Fixture boundary

The opt-in macOS tests start `/usr/bin/ssh-agent` in foreground mode on an
explicit socket inside a fresh private directory under `/private/tmp`. The child
has a cleared environment. `-a` selects only that socket; `-U` disables cleanup
of personal agent directories. The fixture never discovers `SSH_AUTH_SOCK`, uses
`connect_env`, invokes `ssh-add`, reads a private-key file or contacts an existing
agent. It never uses `ssh-agent -k`; cleanup terminates and reaps only its owned
child, then removes its temporary directory/socket.

Generated Ed25519 fixture keys are loaded into the disposable agent through its
local protocol, entirely in memory. Provisioning therefore transmits generated
private-key material to that disposable process; it is not written to disk or
logged. The authentication helper receives only a trusted server public key,
explicit socket path and selected public-key fingerprint. It has no private-key
parameter. Private fixture keys are dropped before the successful client flow.

`make check-ssh` runs the existing trust/PTY checks and the additional agent tests
on macOS. Other platforms do not compile the agent-specific module, and no agent
support result is claimed for them. Native default tests and these opt-in tests require the system agent
and its `-U` flag; unsupported/missing tools fail rather than fabricate success.
The installed companion client reports OpenSSH 10.3p1 / LibreSSL 3.3.6. The local
agent does not implement the version-query flag, so an agent version was not
independently reported.

## Authentication policy exercised

Complete a verified SSH handshake before connecting to the agent, listing its
public identities or requesting a signature. An unknown or changed server key
must produce zero client-path agent accesses and zero authentication attempts.
The agent fixture may already exist for the test; those counters measure the
client authentication path, not fixture provisioning.

Select exactly one plain public key matching the explicitly chosen SHA-256
fingerprint. An empty agent, missing/ambiguous selection or unsupported identity
must not trigger a signature or remote authentication. A rejected selected key
must not fall back to another available identity. Certificates and hardware-key
support remain outside the proven baseline.

Delegate only the selected key's SSH authentication signature to the agent.
Agent removal of that key, a disconnected transport or refusal to sign must
return a fixed safe failure. No generic signing endpoint or agent forwarding is
exposed. The bounded helper owns its SSH transport, so failures and timeout drop
the connection even if the SSH library detaches an internal session task.

## Evidence and remaining work

The tests exercise two authenticated connections, PTY/shell request acknowledgments
and protocol data exchange, plus unauthorized-key rejection, missing identities,
host-key rejection, signing failures and an unresponsive-agent deadline. The
agent-authenticated fixture echoes protocol data; the separate real-shell test
proves native terminal resize and foreground Ctrl-C. Do not conflate those two
fixtures with an integrated graphical terminal.

All network peers and agents are disposable and local. This does not establish
interoperability with a real guest/OpenSSH server, the user's agent, Keychain,
third-party or hardware agents, interactive approval/passphrase dialogs, Windows,
iOS or Android. Native default checks now also exercise the product selection/authentication
path with generated keys, temporary trust stores and disposable agents. No new
application dependency was needed. `STATE.md` records actual checks/results.

The [bounded native PTY transport](ssh-sessions.md) now reuses this authentication
path and has local protocol-fixture evidence. The [graphical terminal](ssh-terminal.md)
is also wired, with separate simulated-IPC WebKit evidence. Integrated native
and real guest walkthroughs remain separate acceptance gates.

Primary references: [OpenSSH agent manual](https://man.openbsd.org/ssh-agent),
[russh agent client](https://docs.rs/russh/0.64.1/russh/keys/agent/client/struct.AgentClient.html),
[pinned agent-client source](https://docs.rs/crate/russh/0.64.1/source/src/keys/agent/client.rs),
[russh authentication API](https://docs.rs/russh/0.64.1/russh/client/struct.Handle.html).
