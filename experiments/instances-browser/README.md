# Disposable instance browser check

Run `make check-instances-browser` on Linux with `/usr/bin/firefox` installed.
The command first builds the frontend and runs its Node controller tests, then
opens an owned headless Firefox profile against a temporary loopback Vite server.
No new dependency is installed and no personal browser profile is opened.

The development-only server injects simulated native IPC before mounting the
actual React application. The fixture checks explicit empty/list loading,
queued/running/succeeded observation, a paused QEMU state without guest readiness,
provider-scope failure, stale result/error suppression after editing the local
service port, same-key resume observation identity in operation history, explicit lifecycle blockers without
mutation controls/raw messages, and stale lifecycle-report rejection. Enabled
fixtures separately exercise exact target review, unchecked consent, cancellation,
start/shutdown results, acknowledgement followed by a fresh check, persisted lost
reply/same-key resume, uncertain GET-only reconciliation and late acknowledgement
identity retention after changing ports. These lifecycle commands are simulated.
It waits for enabled buttons before interacting and reports failure stages.

The browser profile, server and owned browser process group are cleaned up on
completion or timeout. The child environment is explicit; no personal SSH agent,
credential file, Rust SSH command, real control plane or Proxmox host is used.
The application has no fixture flag or mock IPC implementation in its production
bundle. Injection exists only in this opt-in development fixture.

Passing demonstrates React/Firefox behavior under simulated IPC. It does not
compile the new Tauri commands, exercise real Rust-to-HTTP IPC, verify WebKit,
prove packaged CSP behavior or establish native/macOS/mobile readiness.


For only the new remote connection form, run
`node experiments/instances-browser/check.mjs --remote-connection` after building
the frontend. It selects a separate development-only IPC fixture and checks token
clearing/no browser storage, local-control isolation, connect/disconnect and late
response suppression. It does not invoke Rust, validate real TLS or prove mobile
runtime; separate native unit tests cover the HTTPS profile transport.


For the application-key SSH UI, run
`node experiments/instances-browser/check.mjs --app-ssh`. This selects a separate
simulated-IPC fixture for consent/passphrase clearing, failed/successful unlock,
changed-host refusal, explicit identity/authentication/terminal, Ctrl-C/disconnect
and stale unlock/start cleanup. It mounts the real terminal renderer but never
contacts an SSH server, accesses an agent or generates/reads a native key. Native
localhost protocol/real-shell tests provide separate backend evidence; neither
proves a phone or actual new-command Tauri GUI/IPC dispatch.
