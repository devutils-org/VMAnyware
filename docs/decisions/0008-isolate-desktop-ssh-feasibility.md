# ADR 0008 — Isolate desktop SSH/PTY feasibility before product adoption

Date: 2026-10-16
Status: Experimental evidence on macOS arm64; production/mobile choice pending.

Proxmox availability is not required to test desktop SSH protocol behavior.
Keep an opt-in crate in `experiments/ssh` with a separate workspace, lockfile and
dev dependencies, outside the application dependency graph and default checks.
Use a loopback-only generated server identity and signed ephemeral client key;
never load existing credentials, SSH configuration or known-host files.

The russh 0.64.1/ring candidate demonstrates strict expected-key validation,
unknown/changed-key rejection before authentication, signed key authentication,
PTY/session messages, binary data, resize, disconnect and fresh reconnect.
The portable-pty fixture additionally hosts a real local shell and checks actual
terminal size, output and Ctrl-C interruption of a foreground process. It is a
fixture dependency, not a requirement for a future client requesting a remote PTY.

Do not adopt the full experiment as a product subsystem. Its current dependency
set includes prerelease SSH-key code and has not been tested on mobile. The
fixture's explicit expected key is not persistent trust or a first-use UX, and
its generated identity is not platform credential storage. Before integration,
prove interoperability, choose persistent host-key trust and platform key access,
and bound terminal transport/backpressure through Tauri and the renderer.

These tests retire part of the desktop transport risk. They do not validate an
integrated terminal, Windows/mobile behavior, a remote Proxmox guest or Phase 0
exit. Keep the initial unexplained development timeout and actual verification
scope in the experiment README; do not claim reliability beyond the evidence.

Follow-up: [ADR 0010](0010-native-ssh-host-inspection.md) promotes only the
handshake/trust path into a macOS native prototype after the fixture evidence.
The shell fixture and mobile/platform decisions remain separate.
