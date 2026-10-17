# Isolated desktop SSH/PTY feasibility experiment

This is an opt-in Unix test crate with its own workspace and lockfile. It does
not add the shell fixture dependencies to the application/server or start a
persistent service. The macOS native host-inspection prototype now uses the same
SSH candidate and shares the trust source at
`apps/client/src-tauri/src/ssh/trust.rs`; this experiment imports that file. It binds an ephemeral IPv4 loopback port, generates Ed25519 host and
client keys in memory, and closes its fixture on completion. It never reads
personal SSH configuration, credentials or OpenSSH known-host files. Only the
experiment's temporary managed public-key records are persisted/read. No private
key is written to disk or logged.

From the repository root:

```sh
CARGO_TARGET_DIR=/tmp/vmanyware-ssh-target cargo fmt --manifest-path experiments/ssh/Cargo.toml -- --check
CARGO_TARGET_DIR=/tmp/vmanyware-ssh-target cargo clippy --manifest-path experiments/ssh/Cargo.toml --locked --all-targets -- -D warnings
CARGO_TARGET_DIR=/tmp/vmanyware-ssh-target cargo test --manifest-path experiments/ssh/Cargo.toml --locked
```

`make check-ssh` is the opt-in repository convenience target. Default application
checks do not run this experiment. First use downloads the isolated dependency
set. `russh` is pinned to 0.64.1 with default features disabled and `ring`
enabled; its current SSH-key dependency includes a prerelease. This is a
feasibility candidate, not a production library adoption decision.

## Proven by the fixture

- Exact expected public-key comparison during key exchange. Unknown and changed
  keys produce `UnknownKey` before any server authentication callback runs.
  Certificates are rejected; there is no accept-all or implicit first-use trust.
- Signed public-key authentication rejects a generated unauthorized client key
  and accepts the generated fixture account/key.
- SSH session, PTY and shell requests, positive acknowledgments, binary data
  including NUL/non-UTF-8/Ctrl-C bytes, window changes and channel close.
- Two fresh connections to the protocol fixture, each independently authenticated.
- Two fresh connections to a real local `/bin/sh -i` under a native PTY. The
  shell runs only fixed test commands in a newly created temporary directory
  with a cleared environment and no login/profile setup. Actual output is
  observed, `stty size` changes from 24x80 to 43x132, and a Ctrl-C byte interrupts
  the confirmed foreground `sleep 5` process, yielding status 130 within two
  seconds instead of waiting for its normal completion.

The native fixture owns shell/foreground process groups, child process and
reader thread. Closing or dropping it terminates those owned processes, reaps
its shell and joins the nonblocking reader. SSH output forwarding has a deadline;
reads have output bounds and stage-specific timeouts. No OS `sshd` is started
or reconfigured. Fixture commands are fixed source literals, not user input.

## Verification and limits

Verified on 2026-10-16, macOS arm64 (Apple M1), Rust/Cargo 1.99.1:
formatting and Clippy with warnings denied passed; all sixteen tests passed on macOS.
During development, an initial native-PTY run hit its ten-second overall limit;
its precise cause was not established. Subsequent tests passed, including eight
consecutive runs after explicit reader shutdown was added. Per-stage output
limits now identify the awaited marker on a timeout. Final checks also cover
fresh reconnection to the real-shell fixture.

This proves a local desktop Rust SSH/PTY transport and native Unix terminal path.
It does **not** prove interoperability with a Proxmox guest/OpenSSH server,
VMAnyware's Tauri/WebView terminal, terminal rendering or keyboard handling,
a manual walkthrough of the native first-use confirmation UI, platform key
storage, long-lived
backpressure/recovery, installers, Windows, iOS or Android. It does not complete
roadmap Phase 0E/0F. The application still has no SSH terminal.

## Primary API references

- [russh client handler and host-key callback](https://docs.rs/russh/0.64.1/russh/client/trait.Handler.html)
- [russh server handler: authentication, PTY, shell and resize](https://docs.rs/russh/0.64.1/russh/server/trait.Handler.html)
- [russh channel methods](https://docs.rs/russh/0.64.1/russh/struct.Channel.html)
- [portable-pty native PTY API](https://docs.rs/portable-pty/0.9.0/portable_pty/)

## Persisted trust experiment

`trust::Store` explicitly creates a private managed directory; reopening requires
an initialized `trust.json`. Missing, malformed, oversized or unreadable content
fails closed. It never discovers or modifies `~/.ssh`. Tests create temporary
managed directories and delete them afterwards. Only endpoint and public-key
records are persisted, never private keys or credentials.

`inspect` returns trusted, an opaque unknown-key observation, or changed-key
failure. Unknown hosts are rejected during key exchange before authentication.
The caller must explicitly pass the exact displayed SHA-256 fingerprint to
`confirm`; dropping the observation cancels with no write. Confirmation checks
the latest store again under a file lock. There is no key-replacement API.
This is a tested confirmation API, not an implemented user confirmation dialog.

Version 1 JSON stores `entries` containing `endpoint: {host, port}` and
`public_key` in comment-free OpenSSH format. Unknown fields, duplicate endpoint
records, noncanonical endpoints and invalid keys are rejected. Limits are
64 KiB and 128 entries. DNS names are ASCII/lowercase, trailing dots rejected;
IP strings normalize without DNS lookups. Different ports and DNS aliases are
separate identities. SSH certificates are unsupported and rejected.

The managed directory must be private (0700); files must have no group/other
permissions (created 0600). Symlink leaves are rejected and file opens use
`O_NOFOLLOW`. Cooperating processes hold an exclusive stable `trust.lock` across
reads and atomic same-directory tempfile replacement, file sync and directory
sync. Lock acquisition is bounded to 250 ms with a fixed busy error; this fixture
uses synchronous filesystem operations, pending async-boundary integration.
Concurrent distinct updates survive; identical confirmations are
idempotent; conflicting first-use confirmations cannot overwrite a winner.
This is a managed local filesystem boundary, not a defense against malicious
same-user processes renaming ancestor directories or lock files, nor a claim
about network-filesystem lock/durability behavior.

Tests cover persistence/reopen, cancellation, wrong confirmation, endpoint scope,
malformed/oversized/missing/insecure stores, stale and concurrent confirmations.
A real local russh fixture proves rejection before authentication, exact
fingerprint confirmation, reopened-store authentication, and changed-key
rejection without replacing the original record. The changed-key fixture uses
an injected new transport address for the same logical endpoint; it does not
perform a DNS or network takeover.

Next: choose platform credential access and connect a desktop terminal renderer
through Tauri, then repeat essential interactions on real iOS/Android devices.


## Disposable OpenSSH-agent authentication

The macOS-only agent fixture starts `/usr/bin/ssh-agent -D -U -a` with an
explicit private temporary socket and a cleared environment. The `-U` flag
prevents personal agent-directory cleanup. It never discovers the user's agent,
reads private-key files or uses `ssh-add`/`ssh-agent -k`. Generated fixture keys
are provisioned over the local agent protocol in memory; the authentication
helper receives only the selected public fingerprint and socket path. Cleanup
kills/reaps only the owned child and removes its directory.

The six additional tests prove verified host handshake before any client-path
agent access; exact fingerprint selection with no fallback; two successful
signed authentications and subsequent protocol PTY/data exchange; empty/missing
identity or socket failure; unauthorized selected-key rejection; agent removal
of a key or disconnect during signing; and a three-second unresponsive-agent
deadline with connection closure. These agent-authenticated channels use the
echo fixture, while the separate real-shell test validates native PTY behavior.

This does not add authentication to the native application, access a personal
agent, test Keychain or establish hardware/third-party/mobile agent support.
The system agent must support `-U`; missing/unsupported tools fail explicitly.
See [agent policy and evidence](../../docs/ssh-agent.md) and
[ADR 0011](../../docs/decisions/0011-desktop-agent-backed-authentication.md).
