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


To cover the Android input adapter in a browser, run
`node experiments/instances-browser/check.mjs --app-ssh --android-input`.
The fixture overrides the user agent before mounting React and drives the actual
mobile input component with DOM input/composition events. It asserts exact bytes
sent through simulated IPC for typing, replacements, Unicode deletion,
composition (including a repeated final input event), Enter, Tab, Escape and
Backspace, followed by the existing interrupt/disconnect and stale-session checks.
The password-style input must keep potentially hidden remote input concealed.
This is Firefox component/transport-boundary evidence, not Android WebView, real
IME, native SSH or device keyboard/layout evidence. No production fixture flag or
additional runtime dependency is introduced.

The remote-connection scenario also checks the explicit ready-target SSH handoff:
queued/failed creations cannot offer it, a successful creation does not inspect
or connect automatically, and selecting it prefills the exact IPv4/account behind
the usual unknown-host confirmation and fresh-handshake gate. Reselecting the
target clears authentication state; disconnect removes the SSH screen. Recovery
prefills only the recorded address and leaves the account empty. The separate
`--app-ssh` scenario covers affected default-account callers. This remains
simulated-IPC browser evidence, not an integrated native creation/SSH result.

For only the remote snapshot-list UI, run
`node experiments/instances-browser/check.mjs --snapshots`. It checks explicit
reads bound to the connection/instance, empty versus error states, rejection of a
response for another target, unknown/incomplete metadata, escaped descriptions,
backup wording, and suppression of a pending response after disconnect. It also
asserts the scenario submits no mutation command. This is real React/Firefox
interaction with simulated IPC; it neither compiles the native command nor
contacts HTTPS/Proxmox. The fixture waits for the workspace button to become
enabled before clicking it.

For only the new remote snapshot-creation form, run
`node experiments/instances-browser/check.mjs --snapshot-create` after building the
frontend. This isolated simulated-IPC scenario selects a registered VM without
implicit work, checks disabled/failed/mismatched review refusal, exact consent,
escaped description and disk-only/backup wording, locks target/input, retries a
lost reply with identical key/arguments, refreshes queued/running/uncertain/success
states through read commands, acknowledges only terminal results and suppresses
a pending preparation after disconnect. The fixture records and compares command
arguments. No native command, HTTPS peer or VM is used by this browser check.

For only the snapshot history/saved-ID recovery UI, run
`node experiments/instances-browser/check.mjs --snapshot-recovery` after building
the frontend. The focused simulated-IPC scenario checks explicit per-instance
history, empty/error separation, row and saved-ID reads, target matching,
saved/current distinction, escaped description, uncertain/failed states, stale
results after ID editing, stale errors after disconnect and fresh-connection
recovery. It asserts no snapshot preparation/confirmation/clear command is invoked.
This is browser evidence, not actual native process/server restart proof.

The focused `--snapshot-restore` mode checks actual React restore selection,
review and dual unchecked consent with simulated native IPC. It covers disabled/
failed/mismatched prerequisites, explicit current-data-loss and backup/power limits,
escaped description, same-key unknown-response retry, GET-only known-operation
tracking, wrong-success fingerprint refusal, terminal acknowledgement, target/
creation exclusion, fresh-connection consent reset and late review/status
suppression after disconnect. It never contacts native code, a server or Proxmox.
Run `node experiments/instances-browser/check.mjs --snapshot-restore` on Linux
with Firefox and installed client dependencies. Use the native fixture separately
before claiming a native restore journey or any real VM restore.

`--restore-recovery` exercises the separate saved-restore React panel with simulated
read-only native IPC: explicit history/empty/error, row/manual-ID lookup, historical
versus current state wording, uncertain/failed outcomes, wrong target/fingerprint
refusal, stale edited-ID reply and disconnected history-error suppression, then
fresh-connection lookup with no mutation commands. Run
`node experiments/instances-browser/check.mjs --restore-recovery`. It uses no native
profile, actual server, provider or VM; actual restart evidence requires the native
fixture separately. Existing creation-history checks remain `--snapshot-recovery`.
