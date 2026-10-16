# Isolated desktop SSH/PTY feasibility experiment

This is an opt-in Unix test crate with its own workspace and lockfile. It does
not add dependencies to PocketLab's application/server or start a persistent
service. It binds an ephemeral IPv4 loopback port, generates Ed25519 host and
client keys in memory, and closes its fixture on completion. It never reads
SSH configuration, credentials or known-host files. No private key is serialized.

From the repository root:

```sh
CARGO_TARGET_DIR=/tmp/pocketlab-ssh-target cargo fmt --manifest-path experiments/ssh/Cargo.toml -- --check
CARGO_TARGET_DIR=/tmp/pocketlab-ssh-target cargo clippy --manifest-path experiments/ssh/Cargo.toml --locked --all-targets -- -D warnings
CARGO_TARGET_DIR=/tmp/pocketlab-ssh-target cargo test --manifest-path experiments/ssh/Cargo.toml --locked
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
formatting and Clippy with warnings denied passed; all four tests passed.
During development, an initial native-PTY run hit its ten-second overall limit;
its precise cause was not established. Subsequent tests passed, including eight
consecutive runs after explicit reader shutdown was added. Per-stage output
limits now identify the awaited marker on a timeout. Final checks also cover
fresh reconnection to the real-shell fixture.

This proves a local desktop Rust SSH/PTY transport and native Unix terminal path.
It does **not** prove interoperability with a Proxmox guest/OpenSSH server,
PocketLab's Tauri/WebView terminal, terminal rendering or keyboard handling,
known-host persistence/first-use confirmation, platform key storage, long-lived
backpressure/recovery, installers, Windows, iOS or Android. It does not complete
roadmap Phase 0E/0F. The application still has no SSH terminal.

## Primary API references

- [russh client handler and host-key callback](https://docs.rs/russh/0.64.1/russh/client/trait.Handler.html)
- [russh server handler: authentication, PTY, shell and resize](https://docs.rs/russh/0.64.1/russh/server/trait.Handler.html)
- [russh channel methods](https://docs.rs/russh/0.64.1/russh/struct.Channel.html)
- [portable-pty native PTY API](https://docs.rs/portable-pty/0.9.0/portable_pty/)

Next: connect a desktop terminal renderer through Tauri to a disposable,
verified-host-key SSH endpoint, then separately repeat essential interactions
on real iOS/Android toolchains and devices.
