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

`--deletion` exercises the actual React deletion review with simulated native IPC:
explicit instance selection, disabled/running/changed-target/unknown-effect refusal,
reviewed volumes/snapshot references/fleecing and preserved scope, fixed cleanup
wording, independently unchecked target/data-loss consent and shared mutation
selection lock. It compares unknown-response retry arguments, requires known-ID
tracking to use read commands, retains partial failure, refuses success missing
volume absence and removes active controls for retired identities. It also checks
acknowledgement, fresh-connection consent reset and late preparation/status
suppression. Run `node experiments/instances-browser/check.mjs --deletion` after
building the frontend. No native command, actual server, provider or VM is used.

Run browser modes sequentially. An isolated profile is not evidence that concurrent
Vite/browser runs are independent: a parallel restore run timed out before snapshot
selection on 2026-10-21; its exact sequential rerun passed, with cause unconfirmed.

`--deletion-recovery` checks the separate saved-deletion React panel with simulated
read-only IPC. It covers explicit empty/history requests, registered and retired
rows, scoped row/manual-ID reads, saved cleanup/effect/preserved scope, historical
versus current-state wording, partial/failed records, wrong target/hash refusal,
error versus empty history, stale edited-ID detail, disconnected history errors
and fresh-connection lookup without restored consent. It asserts no preparation,
confirmation or clear command is invoked. Run
`node experiments/instances-browser/check.mjs --deletion-recovery` sequentially
after building the frontend. No native profile, actual server/provider or VM is used.

`--recreation` exercises the deleted-row recreation UI with simulated IPC. It
requires an explicit target VMID, source/archived-definition/target review and
unchecked confirmation. Disabled execution, occupied target and changed source are
refused. Cancel/review resets consent; active reviews lock mutation selectors and
ordinary creation. A lost-response fixture permits only identical-key/consent retry;
known operations expose status reads only. Uncertain work remains retained, false
guest readiness and old-instance reuse cannot offer SSH, and verified completion
hands the new guest to existing SSH host-trust setup. Acknowledgement, disconnect
and late replies must not restore source review/consent/key. Run:

```sh
node experiments/instances-browser/check.mjs --recreation
```

This is actual React/Firefox interaction with simulated IPC, not native Tauri,
server/provider execution or real VM acceptance. Run modes sequentially.

`--recreation-recovery` exercises explicit retired-source history, row/manual-ID
lookup and saved refresh using only the two native read commands. It covers empty
versus error history, historical target/new identity/definition details, fresh-disk
semantics, uncertain status, wrong-source/false-success refusal, invalid IDs,
edited-ID stale replies and reconnect with no restored ID/consent. No checkbox,
mutation command or SSH handoff is offered from historical evidence. Run modes
sequentially:

```sh
node experiments/instances-browser/check.mjs --recreation-recovery
```

This is actual React/Firefox interaction with simulated IPC; current native/server
restart and exact updated-APK acceptance are separate evidence.

## Remote lifecycle workspace

`node experiments/instances-browser/check.mjs --remote-lifecycle` exercises the
actual React remote workspace with simulated IPC only. It covers explicit start/
graceful-shutdown selection, no action on selection, disabled/unknown/mismatched
reviews, initially unchecked consent, expiry before submission, target exclusion,
identical retry after an unknown reply, known-operation reads, unrelated success
refusal, uncertain-state retention, power-versus-SSH wording, terminal acknowledgement
and stale reconnect responses. It never contacts a provider or executes native IPC.
Affected recreation and snapshot-create scenarios cover the shared parent selection
and mutation lock. Actual native UI/provider integration remains separate.

## Saved lifecycle recovery

`node experiments/instances-browser/check.mjs --lifecycle-recovery` tests explicit
history/manual-ID/refresh on registered and archived rows. Real React with simulated
IPC verifies empty/error separation, historical target/action/power wording, no
consent/retry/SSH control, uncertain evidence, wrong-instance/false-success refusal,
invalid ID blocking, edited-ID stale detail and reconnect stale history suppression.
Only the two fixed lifecycle recovery IPC commands may be invoked by the panel.
This is not native HTTP, restart or real VM acceptance. Shared parent action/history
flows are checked with --remote-lifecycle and --recreation-recovery.

## Explicit remote power reads

`node experiments/instances-browser/check.mjs --remote-power` exercises registered-
only explicit read/refresh, exact selected target, running versus paused QMP,
local read-completion time and possible-staleness wording. It checks that failed/
wrong-target reads clear prior evidence, mutation review invalidates in-flight
observations, missing QMP remains unknown, reconnect drops late replies, and no
mutation/SSH command follows from power. This is React with simulated IPC, not
native/provider evidence. --remote-connection and --remote-lifecycle cover the
shared environment-row integration and the revised registration qualification.

## Retained remote instance definition

`node experiments/instances-browser/check.mjs --remote-detail` exercises explicit
registered/archived definition reads, selected target/name, revision/hash display,
literal YAML (including HTML-looking text), desired-versus-observed wording,
failed-refresh clearing, malformed metadata refusal, target replacement/reconnect
stale-result rejection and hide. No mutation or provider/native operation runs.
Hash byte validation is tested in the native command suite; this browser fixture
supplies mock verified native data and tests rendering/selection guards. Adjacent
--remote-power and --recreation-recovery flows cover the shared environment rows.

## Existing-instance manual SSH entry

`node experiments/instances-browser/check.mjs --existing-ssh` tests registered-row
selection into the existing host-trust UI, with blank address/account and an explicit
port. Simulated IPC covers no automatic inspection/login, invalid URL blocking,
independent unknown-key consent and fresh reinspection, changed-key refusal,
reselection/stale inspection cancellation, association archival cleanup and reconnect
reset. It does not authenticate, unlock an identity or open a session. Existing
--remote-connection and --recreation scenarios protect their ready-guest SSH handoffs.
This is UI/selection evidence, not actual native SSH or endpoint-to-VM attestation.


## Declared provisioning inputs and exact review

`node experiments/instances-browser/check.mjs --declared-create` runs the real
remote React form with simulated IPC. It checks empty/unchecked defaults, exact
package/version order and final-snapshot intent, refusal of changed server intent,
fieldset locks, edit/re-prepare clearing consent and changing request key, and the
existing unknown-response retry/status/recovery/SSH-trust journey. The launcher
requires the declared scenario's result marker. --remote-connection still covers
the empty/no-snapshot default. --recreation additionally checks retained package/
final-snapshot summaries. No native provider, SSH or VM mutation occurs here.

The declared-create, recreation and recreation-recovery scenarios also exercise
explicit recorded provisioning progress: ordered package/snapshot stages,
operation/plan/source/target matching, historical wording and no progress-based
SSH grant. Creation covers fixed error/redaction, mismatched plan refusal and a
late read discarded after replacing operation status. Reads supply only connection
and operation IDs; no intent or consent is installed. These remain simulated IPC.
