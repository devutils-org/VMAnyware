# VMAnyware — current state

Updated: 2026-10-20. Historical evidence and decisions are in Git and docs.

Naming note: historical lab resource identifiers below are normalized to the
current VMAnyware name. External resources were not renamed; use VMIDs and
ignored local lab records to verify actual identifiers before operations.

## Latest resume point

2026-10-21: the user widened the scope to a local provider beside Proxmox
(VMAnyware.md §9). ADR 0062 proposes the seam, direct QEMU (or libvirt) and a
first slice with its acceptance; it awaits the user's choice of hypervisor and
first local target. No code, no VM operation.


Latest server slice adds read-only snapshot listing for registered instances,
with fixed retained scope/target, exclusion of Proxmox's live `current` entry,
explicit incomplete metadata and safe errors. See docs/snapshots.md and the
verification entry below. The remote client now displays snapshot metadata with
explicit reads, target binding and empty/error/incomplete states; frontend build
and focused Firefox simulated-IPC checks pass. The actual native crate now
compiles and all three snapshot HTTPS tests pass using the recreated, signed
package prefix `.lab/native-deps`; four affected shared-read tests also pass.
Actual native WebView -> production Rust HTTPS -> authenticated Axum -> generated
HTTPS provider snapshot listing also passes (details below). Read-only snapshot
creation preflight now validates bounded inputs, capability, uniqueness, incomplete
work and retained target stability, with five targeted tests passing. Snapshot
HTTP submission now requires independent runtime opt-in and full confirmed
target/request with one idempotency key. Schema 11 includes the durable confirmed
snapshot intent, mutation exclusion, dispatch/task guards and restart semantics. Storage tests
pass, including affected lifecycle/provisioning migrations. Fixed one-shot
transport, revalidating executor, task/metadata completion and GET-only recovery
now pass generated-HTTPS tests. The server services this queue with the independent
VMANYWARE_SNAPSHOTS_ENABLED opt-in, default false. Confirmed HTTP enqueue,
capability reporting and generated-provider queue-to-result checks now pass.
Native retained prepare/confirm/read/ack commands now compile and pass ten
focused HTTPS tests, including unknown-response same-key retry and disconnect.
The remote client now has a single retained snapshot review/consent/status form;
frontend build and focused Firefox simulated-IPC checks pass. Actual isolated
Linux WebView -> production native commands -> authenticated server -> generated
HTTPS provider snapshot creation now passes, with one POST and journal/result
verification. Read-only per-instance server operation history/detail recovery now
passes seven targeted HTTP/SQLite tests without provider access or write flags.
Native fixed GET history/detail commands now compile and pass seven targeted
HTTPS tests without preparing a new intent. The client history/saved-ID UI now
builds and passes focused Firefox checks, including a fresh connection and stale
reply suppression. The real native fixture now creates once, restarts both app
and server, and recovers the unchanged succeeded operation through GET only with
provider configuration removed and writes disabled. Read-only disk-only restore
preflight now passes six targeted HTTPS/API tests; it binds selected metadata and
canonical saved configuration, requires stopped power and keeps execution off.
Schema 12 now adds explicit data-loss approval and durable no-replay restore
storage, with matching rollback-task and stopped-target evidence guards.
Fixed one-shot rollback transport and the revalidating executor now pass seven
generated-HTTPS tests. A separate VMANYWARE_RESTORES_ENABLED flag, default false,
is wired into the actual server worker; process tests pass. Confirmed restore HTTP
enqueue now binds both path identities and data-loss approval, reports independent
capability and passes API/generated-provider/actual-process checks. Native retained
destructive restore commands now compile and pass 14 HTTPS tests,
including both consent flags, unknown same-key retry and GET-only known-operation
tracking. Explicit client restore selection, destructive review, dual consent and
status now build and pass focused Firefox checks; affected creation/list checks
also pass. Actual isolated native restore now passes with a deliberately lost
HTTP response, identical API retry, one provider rollback POST and independent
journal/fingerprint/stopped-target checks. Scoped read-only restore history/detail
now pass 11 HTTP/SQLite tests without provider/write capabilities. Native fixed
GET saved-restore recovery now compiles and passes nine HTTPS tests without
preparing consent. Explicit client saved-restore history/manual-ID/refresh now
builds and passes focused Firefox checks. Actual native app/server restart
recovery now passes with provider removed, all writes off, GET-only traffic and
unchanged operation. Read-only deletion target/protection/volume review now passes
five HTTPS/API tests. Permission-backed HA/replication reads now pass four new
and five affected checks. Fixed provider effects, recorded fleecing and candidate/
preserved volume scope now pass ten targeted checks. Read prerequisites can pass,
execution stays off. Full-scope deletion consent and schema-13 durable no-replay
journal now pass eight new and 67 affected checks. Independent deletion
absence observation passes three targeted HTTPS tests. Schema-14 atomic retirement
now requires an opaque matching task/absence proof; retained deleted identities
and client list compatibility pass local checks. Standalone fixed DELETE and
no-replay executor now pass eight generated-HTTPS checks, including GET-only
partial-cleanup recovery. Next: separate default-off deletion runtime capability
and fair worker/process checks, then HTTP/native/UI integration.
Snapshot creation/restore and real infrastructure testing remain open.

Android SDK/emulator were stored under `/tmp/vmanyware-android-resume`, which is
absent on 2026-10-20. Native Android work requires recreating that owned toolchain;
do not rerun tests or claim fresh mobile evidence from the historical reports.
The initial Gboard key-grid visibility and Unicode/composition gates remain open.

Current Android x86_64 APK passes the actual native confirmed-create/API/worker/
generated-HTTPS-provider journey, explicit ready-target handoff and same-key
SSH/OS-shell checks through direct emulator host-alias networking, with zero ADB
reverse mappings (ADR 0053). Earlier tunnel-only limitations are superseded for
this owned emulator. The provider/guest remain synthetic, not a real VM.
Brief Android HOME/background/resume now also passes on the same SSH session
without input or authentication replay (details below). Actual Gboard ASCII
letter/space, backspace and send taps also pass after rotation/control-key checks.
The initial Gboard key-grid visibility issue and Unicode/composition remain
unverified. Keep real VM writes gated by concrete
target/action approval. Full roadmap, other platforms, lifecycle/snapshots/
recreate/delete, deployment and release acceptance remain open.

## Licensing

Original code, documentation and assets are offered under MIT OR Apache-2.0.
Added both license texts and aligned Cargo workspace/client/server/SSH experiment
and npm metadata. Third-party licenses remain unchanged.
Verification: parsed manifests and checked license inheritance/texts; no native,
application or infrastructure tests were needed for metadata changes.
Next: include both texts in future application installers when packaging is added.

## README presentation

The README follows the devutils structure: overview, install, use, documentation,
development and license. Detailed text is preserved in `REFERENCE.md`.
Documentation-only verification: checked relative link targets and section order;
no application or infrastructure tests were run. Product verification is unchanged.
Next documentation task: keep the concise entry points aligned with implementation.

## Priority — first usable mobile journey

The user wants a usable mobile app that creates a VM and opens SSH before completing
the rest of the roadmap. Prioritize this vertical journey: secure remote control-plane
access; complete create/start/readiness workflow; one selected mobile platform with
OS-appropriate identity storage and terminal; then an explicitly approved disposable
VM end-to-end check. Defer snapshots, recreate/delete and broad packaging until this
journey works. The full roadmap remains the overall goal.

A platform preference question is pending (Android/iPhone/both). Platform-independent
backend work can continue. The macOS agent remains platform-specific; application-key
SSH now passes Linux native protocol/real-shell fixtures. Android ARM64/x86_64 debug
APKs build; Android 15 x86_64 native bridge/key persistence/launch now pass. Mobile
SSH/ADB-injected keyboard/rotation/reconnect now pass via an owned ADB tunnel.
Native authenticated HTTPS and read-only recovery now pass against the actual
local service through a verified-TLS ADB tunnel. Direct networking, complete
create-to-SSH, full software IME/lifecycle and deployed access remain unproven.
Do not infer that the desktop agent backend works on mobile or expose unauthenticated
remote control-plane writes. ADR 0034 records this priority correction.

## Resume here — latest Linux slice

- Verification rule: select the smallest tests/checks covering the current slice's
  diff and affected callers. Do not repeat unrelated/already-passing suites without
  a concrete reason. See AGENTS.md; documentation-only edits need review only.
- Registered instance associations, scoped synchronous/queued observations and
  explicit client monitoring are implemented; schema version is 14.
- Latest slice: confirmed minimum create API and production worker pass complete
  generated-HTTPS journeys with/without resize, restart and idempotent replay.
  Remote native create commands, visible confirmation and status now pass local
  targeted tests; read-only reconnect/restart recovery now passes native HTTPS and
  browser checks. Encrypted app identity/public-key insertion now passes Linux
  native and browser checks. Application-key SSH authentication/PTY now passes
  real local shell and browser checks. Android debug APKs and actual emulator
  bridge/key/restart/painted-launch checks pass; current APK SSH/PTY/input/rotation
  and reconnect pass through an explicit owned ADB reverse tunnel. Direct remote
  network/create integration and full software IME remain next. Native HTTPS
  connection/read-only recovery now pass with a connection-scoped private CA.
  Prior Rust/HTTPS/journal evidence is
  recorded below. These are targeted runs, not a new full suite. Earlier baseline: 107 server tests; frontend build + 23 Node tests;
  Linux Firefox simulated-IPC walkthrough and real server/TLS/worker/restart flow.
- Native Tauri HTTP commands now compile and pass on Linux via a temporary
  signed-package WebKitGTK prefix (122 workspace tests: 104 server, 18 native).
  Isolated Linux native lifecycle GUI/IPC passes; normal packaged startup
  and real VM snapshot/restore remain unverified; no lifecycle
  write was executed on real infrastructure. Runtime capability defaults off.
- Read-only lifecycle preconditions now check marker, lock, template, pending
  changes and configuration revision. API capability reflects explicit runtime opt-in;
  passing checks never replace a full action confirmation.
- Lifecycle checks now have an explicit client display with scoped/stale-result
  guards. Durable confirmed action storage and fixed provider transport are
  implemented with a locally tested durable executor, validated opt-in capability
  and fair sequential worker rounds. Confirmed HTTP enqueue is locally tested.
  Real binary HTTPS start/restart/no-replay evidence now passes.
  Typed native action/read commands now compile and pass HTTP tests.
  Standalone confirmed intent/monitoring controller now passes Node tests.
  React explicit confirmation, persistence and resume controls now pass browser tests.
  Real Linux WebKit/Tauri/Axum/generated-HTTPS lifecycle UI now passes.
  Fixed full-clone transport/target observation now passes HTTPS tests.
  Durable provisioning intent/journal and clone dispatch/recovery guards now pass
  targeted SQLite tests. Standalone clone execution/completion and GET-only restart
  reconciliation now pass local HTTPS tests. Fixed CPU/memory configuration and
  observation pass locally. Schema-7 durable CPU/memory execution and GET-only
  restart reconciliation pass. Fixed mapped-disk expansion/configured-size
  observation passes. Schema-8 disk dispatch/task/outcome/recovery guards pass;
  standalone disk execution and GET-only recovery now pass HTTPS fixtures;
  fixed network/Cloud-Init settings and independent observation now pass locally;
  schema-9 Cloud-Init journal and standalone execution/recovery now pass;
  generated seed semantic inspection now passes locally and rejects the known
  scalar form. Remote client reads and explicit template/default-user opt-in
  now pass targeted local checks; next: seed regeneration/start/readiness.
  The detailed final sections below distinguish actual and deferred evidence.

## Implemented

- Tauri 2/React/TypeScript native shell; user confirmed the macOS native bridge.
- Loopback-only Axum service, validated runtime configuration, verified-TLS
  Proxmox reads and safe errors. Fixed lifecycle POST transport is locally tested;
  the service defaults new writes off, with full-target confirmed HTTP enqueue
  available only under explicit runtime capability.
- Strict versioned YAML definitions, deterministic exports/hashes and SQLite
  create/list/read storage. Native editor validates without saving and provides
  safe field hints. Definitions named `validate` can also be read normally.
- Durable inventory jobs with idempotent enqueue, background worker, polling,
  terminal persistence and interruption recovery. SQLite migration 2 and an
  exclusive process lock protect database ownership.
- Explicit server template catalog, deterministic offline provisioning preview,
  and read-only preflight for template/disk/Cloud-Init/storage/bridge/VMID.
  Results are passed/failed/unknown and bound to current inputs/plan hash.
- Read-only provider task observation with validated UPID/node identity,
  redacted failure status and bounded polling. No durable provider-task linkage
  or guest readiness is implied by this helper.
- Safe JSON request/lifecycle events and generated request IDs. Unix SIGINT and
  SIGTERM request graceful shutdown with a five-second HTTP drain limit; startup
  failures and drain timeout emit fixed stage codes.
- Prepared GitHub Actions server/frontend checks with pinned actions and locks;
  workflow not pushed or executed; local Linux server container checks pass.
- Debian Cloud-Init template guide, platform/toolchain evidence matrix and
  interim feasibility report. No real template preparation was executed.
- Isolated opt-in desktop SSH/PTY experiment, outside the root workspace and
  application dependency graph. Generated in-memory keys and localhost fixture
  prove strict host-key checks, signed authentication, real local shell output,
  resize, Ctrl-C foreground interruption and fresh reconnect. See ADR 0008.
- Isolated macOS OpenSSH-agent authentication proves exact public-fingerprint
  selection, delegated signatures, no identity fallback and rejection of
  untrusted hosts before agent access. Agents/keys/sockets are disposable;
  no user agent or credential file is accessed.
- Shared native/experimental persistent endpoint-scoped SSH host trust. On
  macOS, the native UI inspects keys and offers explicit first-use confirmation;
  known keys require a completed verified handshake, while changed keys cannot
  be approved. Host inspection never authenticates or loads private keys.
- Explicit macOS agent-identity listing and an authentication-only native check.
  Rust keeps expiring, one-use window/endpoint/user/socket/key selections; fresh
  persisted host trust is verified before each agent access. Only one selected
  identity is tried, and success closes the connection without a terminal.
  Startup and host inspection never access the configured agent.
- Native Rust PTY/session transport with window-bound IDs, positive PTY/shell
  acknowledgments, bounded byte input/output, resize and explicit close. Startup
  reuses the same selected-agent authentication path. Window destruction aborts
  its actor and closes its owned socket.
- Graphical macOS terminal prototype using pinned xterm.js 6.0.0/fit 0.11.0.
  Explicit opening consumes the chosen identity; output pulls wait for parser
  consumption, input is serialized/bounded and resizes coalesce. Ctrl-C and
  disconnect controls, stale-result cleanup and StrictMode lifecycle are wired.
  Terminal output cannot set the app title, navigate links or access clipboard.
- Rust-owned window/endpoint/key-bound UUID challenges expire after 120 seconds,
  are consumed once and invalidate on new inspection, cancellation or window
  destruction. Owner/probe limits are eight each; probes have a ten-second
  overall deadline. Trust initializes lazily in the client app-local directory.

## Current verification

Environment: macOS arm64, Rust/Cargo 1.99.1, Node 26.11.1, npm 11.20.0,
Apple CommandLineTools. Dependency locks and compiler pin are committed.

- Latest `make check` passed frontend TypeScript/Vite build, seven Node terminal
  controller tests, Rust formatting/all-target Clippy with warnings denied and
  all 83 workspace tests: 50 server, 33 native. Latest `make check-ssh` passed
  sixteen isolated tests. `make check-terminal-webkit` passed both isolated
  development scenarios. The main bundle is ~246 KiB and the lazy terminal
  ~335 KiB before gzip; no chunk-size warning.
- Provider tests use generated localhost TLS fixtures, never runtime credentials.
  They cover certificate/hostname/authentication failures, time/size bounds,
  inventory, preflight, task status validation and bounded polling.
- Actual server subprocess tests passed safe log-field allowlists/redaction,
  unknown routes/methods, generated IDs, SIGTERM/SIGINT exit zero and restart
  with the same database after lock release. Tests clear the child environment.
- HTTP/storage tests cover YAML validation, save/list/read/conflicts, canonical
  hashes, migrations, reopen, idempotent jobs, terminal persistence and recovery.
- Native HTTP tests cover typed requests, validation/provisioning input bounds,
  safe field hints, redaction, previews and preflight result decoding.
- CI YAML passed actionlint 1.7.12 using a temporary checksum-verified binary.
  Its commands passed locally; no GitHub/native/mobile CI result is claimed.
- Latest `make check-ssh` passed isolated formatting, locked Clippy and all sixteen SSH
  tests on macOS arm64. The real PTY test uses two fresh connections, observes
  stty size changes and requires foreground sleep interruption/status 130 within
  two seconds. Fixture cleanup leaves no owned shell/sleep processes. An early
  development timeout remains undiagnosed and is documented honestly; repeated
  final checks passed. The experiment has no existing credentials or config.
- Trust tests cover exact confirmation/reopen/authentication, rejection before
  authentication, changed keys, port/DNS/IP scope, corrupted/missing/insecure
  data, bounded 250 ms locking, stale/concurrent claims and endpoint validation
  after deserialization. Generated public keys persist only in temporary stores.
  The native client and isolated fixture now import the same trust source.
- New native manager tests cover handshake/confirmation/reopen/changed keys with
  zero authentication attempts, replay/wrong-owner/fingerprint/expiry/cancel,
  invalidation and out-of-order results, capacity and corrupt storage. Window
  destruction cleanup also prevents reuse of an old label reviving a challenge.
  Non-macOS wrappers are source-prepared, not cross-compiled or tested.
- Native agent slice: `make check` passed frontend build, fmt, Clippy and all
  76 workspace tests. Six native agent tests use cleared-environment disposable
  system agents with `-D -U -a`, in-memory fixture keys and temporary trust.
  They cover exact selection/no fallback/no channels, unknown/changed hosts
  before agent access, fresh verification before authentication, removed keys,
  wrong-window/expired/cancelled/replayed selections, stale cancellation, bounds
  and unresponsive-agent timeout/socket closure. No personal agent was accessed.
  The isolated experiment remains at sixteen passing tests; no new dependency
  was added. Native screens compile but their manual walkthrough is pending.
- Native PTY slice: `make check-rust` passed all 80 tests. Two localhost
  protocol tests prove binary NUL/255/Ctrl-C round trips, 32 KiB input, resize,
  positive PTY/shell acknowledgments and explicit refusals, exit/EOF/close,
  wrong-window refusal, disconnect/reconnect and zero remaining owned fixture
  connections after explicit close/window destruction. Two policy tests cover
  queue saturation, single-reader enforcement, capacity and invalid dimensions.
  This is protocol echo, not a native graphical or actual remote OS shell test.
- Native real-shell slice: `make check-rust` passed all 81 tests; isolated
  `make check-ssh` passed all sixteen tests. The native HostManager → explicit
  disposable agent → SessionManager path now exercises actual /bin/sh output,
  stty size 24×80 → 43×132, foreground sleep interruption/status 130 within two
  seconds, reconnect and process/socket cleanup after explicit close and window
  destruction. Closing while sleep 30 is running also leaves none of the owned
  shell/foreground PIDs alive. The experimental and native tests share one
  bounded-output PTY fixture. portable-pty is a macOS dev-dependency only; no
  runtime dependency or user agent/credential/real infrastructure was accessed.
- Output pressure slice: a real yes/paused-reader test exposed Ctrl-C starvation
  in the pinned SSH library's full receive queue. The native actor now fails and
  closes after three seconds of blocked output; failure survives a full queue
  and is observed after queued data, followed by Closed. `make check-rust` passed
  all 82 tests. The pressure test verifies bounded failure/owned-process cleanup,
  then a fresh connection with actively consumed output interrupts yes within two
  seconds and remains usable. This is a documented prototype limitation: a long
  renderer pause can close a session (ADR 0014), not transparent indefinite pause.
- Graphical terminal slice: seven Node tests cover binary/split UTF-8 output,
  renderer backpressure, stale results/errors, bounded serialized input, resize
  coalescing, overflow, native saturation retries and exit/EOF/close.
- Isolated macOS WebKit development smoke passed with simulated IPC and a
  nonpersistent store: explicit identity choice/no automatic agent listing,
  split UTF-8 rendered by xterm, resize, synthetic Unicode textarea input,
  Ctrl-C byte, StrictMode no premature close/duplicate login, disconnect and
  cleanup of a late start after edits. It never invokes Rust SSH commands or
  accesses a personal agent. See `make check-terminal-webkit`. This is not an
  integrated native-to-shell test or packaged CSP/real-keyboard evidence.
- Deferred renderer slice: xterm loads only on explicit terminal opening, before
  any native start. The parent owns the native ID even while a lazy mount is
  pending. WebKit tests now deterministically pause/fail the renderer module: an
  edit during loading starts no late connection; a load error is visible, cancels
  the selection and starts no native connection. Existing render/input/resize/
  StrictMode/stale-native-result checks still pass. No new dependency was added.
- Shutdown-bound slice: `make check-server` passed all 50 server tests. A real
  child with an unfinished HTTP body exits zero after the five-second drain
  deadline, emits only the fixed timeout/stopped events, closes its socket and
  restarts with the same database/port. An idle restart still exits on SIGINT in
  under two seconds. The bound covers HTTP drain, not arbitrary blocking OS I/O.
- Real native Tauri slice: the optimized macOS Cargo example passed the full
  compiled frontend/lazy-xterm → real Tauri IPC → generated-key SSH peer → real
  disposable agent → /bin/sh chain. First-use trust is confirmed only against
  fixture-generated public metadata, then rechecked before exact identity use.
  Two sessions render actual stdout, match stty rows to the resized renderer,
  interrupt sleep/status 130 within two seconds and disconnect/reconnect. Rust
  confirms resize observations, zero owned SSH connections and closed/reaped
  shell fixtures. The child environment, trust store, agent and incognito WebView
  are isolated. No new dependency, production test flag, fake IPC or CSP bypass.
  `make check` still passed 83 Rust/seven frontend tests; all-target Clippy passed.
  Two development assertions were corrected (DOM row concatenation and assuming
  uncoalesced resize counts); final native scenario passed all checks.
- Native changed-key slice: the real Tauri example also rotates only its generated
  localhost server key after two terminal sessions. The actual UI refuses the
  changed key and offers neither overwrite nor identity listing. Native report
  confirms exactly two authentications and byte-identical temporary trust data
  before/after rotation. Optimized native smoke, formatting and all-target Clippy
  passed. This adds no production command or real infrastructure operation.
- The owned local service was restarted with current shutdown code on loopback
  8080 and the existing local database, with provider/catalog variables unset. A
  real GET /api/v1/health returned {"status":"ok"}. Earlier validation POST
  evidence returned HTTP 200/canonical hash/request ID. Current native screens
  have an isolated real-Tauri walkthrough; normal startup/user-agent behavior
  and real guest interoperability still require their own checks.
- Template guide: seven shell blocks passed syntax checks without execution;
  links/catalog sample reviewed; official dated image/manifest/build metadata
  exist. No cloud image was downloaded and no VM operation was executed.

Earlier runtime evidence: `make doctor`, actual local HTTP/SQLite restart smokes,
`make dev` native launch and user-confirmed `Native bridge connected (macos)`.
Latest `npm --prefix apps/client run tauri build -- --no-bundle` passed and
produced the current optimized `target/release/vmanyware-client` (1m02s Rust
release build), confirmed Mach-O arm64, including stall handling/lazy rendering. It was not launched for an integrated SSH walkthrough. The
older renderer smoke uses an isolated WebKit host. The new optimized Cargo
example now proves real Tauri IPC/custom-protocol/CSP integration with disposable
resources; normal app startup/distribution and real guest tests remain open.
Windows shutdown code is source-prepared, not compiled or tested on Windows.
At that historical macOS verification point, no Proxmox infrastructure had
been contacted. The later local x86 lab evidence is recorded below. Phase 0
remains incomplete.

- Linux server slice: `make check-linux-server` passed on committed revision
  9f126f8: Debian 12 aarch64, Rust/Cargo 1.99.1, formatting, locked all-target
  Clippy and all 50 server tests. The first run exposed an external kill-command
  assumption in diagnostics tests; POSIX shell builtin signaling fixes it without
  adding dependencies. `make check-server` also passed all 50 tests on macOS.
  Docker used the explicit local socket, isolated anonymous configuration, pinned
  public image, disposable tracked-source archive and bounded container resources.
  Container/source were removed. No personal credentials or infrastructure were
  accessed. This is not GitHub Actions, Linux frontend or desktop evidence.

- Linux frontend slice: `make check-linux-client` passed against committed
  revision a1ceead in Debian 12 aarch64 with Node 26.11.1/npm 11.20.0: locked
  npm installation, TypeScript/Vite build and all seven controller tests. No
  application code or dependency changes were needed. The server/client targets
  share the isolated Docker runner, with separate digest-pinned public images.
  This does not validate Linux native Tauri or GitHub Actions. Temporary sources
  and container were removed; public images remain cached locally.

- Development launch recovery: a second `make dev` failed because an existing
  project Vite/Tauri session already owned port 1420. The identified old session
  and remaining native child were stopped, leaving the API untouched. A fresh
  `make dev` passed Vite startup, compiled the native client and launched
  target/debug/vmanyware-client. README documents single-session recovery.
  No application code changed; native UI interaction was not asserted here.

- Local-service demo slice: an explicit Check local service / Retry service
  check action calls the native health command without requesting inventory or
  discovering an SSH agent. The UI distinguishes unchecked/checking/available/
  unavailable, clears errors on retry and resets health state on port changes.
  A healthy service does not imply Proxmox is configured. README gives the demo
  steps. `make check-client`, `make check-rust` and `make check-terminal-webkit`
  passed: seven JavaScript tests, 84 Rust tests (50 server/34 native), formatting,
  all-target Clippy and both WebKit scenarios. Native HTTP tests cover healthy,
  unhealthy/malformed responses, invalid port and refused connection; simulated
  WebKit IPC covers unavailable/retry/reset with zero inventory calls. These
  are separate transport/UI checks, not a normal-app end-to-end walkthrough.

- Local server handoff: the previously owned PID 25304 held the default SQLite
  lock and port 8080, causing the user's second server launch to fail. It was
  stopped with SIGTERM. A cleared-environment server reopened the same database,
  emitted server_listening on 8080, then exited zero on SIGTERM. No lock file or
  database was deleted. No agent-owned server remains; the user can run make server.

## Remote repository

Private GitHub repository: https://github.com/moha-bekh/VMAnyware
Local remote: origin (currently SSH). Created at the user's explicit request to publish
existing commits. Main is the branch to push; verify remote HEAD after transfer.
Pushing the prepared workflow may trigger GitHub Actions; local checks are not
proof of a successful hosted run.

## Session handoff — 2026-10-17

- User explicitly stopped the session; do not continue background development.
- Latest implementation commit: 557ff7f (independent local-service check/retry).
  Checks passed: 84 Rust tests, seven JavaScript tests, Clippy/formatting/build,
  two simulated-IPC WebKit scenarios. Linux server/frontend checks passed in
  earlier slices; no Linux native desktop or iOS application proof.
- The duplicate database-lock failure was resolved by stopping the known local
  server and verifying reopen/clean exit against the same database. The user
  may subsequently start their own server; inspect current ownership before
  starting or stopping anything. Never delete the lock to bypass an active owner.
- Demo commands: make server and make dev in separate terminals. Run only one
  Vite/Tauri development session on port 1420. Check local service works without
  Proxmox; Connect requests Proxmox inventory and requires provider configuration.
- Agreed priority: improve/verify the macOS demo while the user prepares Proxmox
  on another PC. Keep macOS as the primary development host; use isolated Linux
  checks. An iOS UI demo is a later feasibility step requiring full Xcode;
  current SSH-agent authentication is macOS-specific.

## Next single task

Obtain explicit approval of the concrete clone-202 snapshot/rollback operations
in `docs/x86-snapshot.md`, then execute that bounded experiment. Read-only API
preflight passed: stopped target, no named snapshots, snapshot feature supported,
active local-lvm with 17,977,492,007 bytes available. Thin-pool metadata gates
still need read-only verification before execution. No snapshot/restart/rollback
was performed. Preserve stopped 201/202 and template 9000.

## Resumed session — Linux environment

- Current execution host is Linux x86_64, not the historical macOS arm64 host.
  `make doctor` failed: Node.js, npm, Rust and Cargo are unavailable to the
  process. No build, test suite or native application launch was performed here.
- Prepared `docs/macos-startup-smoke.md` for the normal application entry point:
  distinct test identifier, removed agent environment, unused loopback port,
  unavailable/retry/reset observations and owned-process cleanup. README links
  it. Reviewed against the native startup source, UI labels and Tauri config;
  the procedure and data/WebView isolation remain unexecuted on macOS.
- No roadmap item is newly complete. No service was started/stopped, no
  credentials were inspected and no Proxmox endpoint was contacted.

## x86 host readiness slice — 2026-10-17

- User redirected work toward tests previously blocked by the M1. Actual host:
  Arch Linux x86_64/kernel 7.2.9, Ryzen 9 3900X, 24 threads, approximately
  62 GiB usable RAM and 278 GiB repository-filesystem free space.
- AMD-V (`svm`) is exposed; `kvm_amd` nested parameter is `1`. Outside the
  sandbox, `/dev/kvm` opens and KVM_GET_API_VERSION returns 12. The sandbox's
  missing device is not a host failure. No VM was created to obtain this result;
  successful nested guest execution remains unproven.
- Earlier doctor output was misleading: sandboxed Node spawnSync reports EPERM
  even when a subprocess prints a version. Rustup also encountered read-only
  paths and an incomplete pinned toolchain. Approved external diagnostics let
  rustup repair/install 1.99.1. External `make doctor` now passes: Rust/Cargo
  1.99.1, Node 26.11.1, npm 12.2.0. No doctor code was changed.
- Locked frontend `npm ci --no-audit --no-fund` installed 26 packages. npm
  blocked esbuild's postinstall under its existing script policy; no policy was
  changed, and the actual Vite build still passed. `make check-client` passed
  TypeScript/Vite and all seven tests. `make check-server` passed formatting,
  all-target Clippy with warnings denied and all 50 server tests, including
  subprocess shutdown/restart. Checks ran outside sandbox subprocess/network
  restrictions. Tracked dependency locks remain unchanged.
- QEMU/libvirt/container runtime executables were not found on PATH; WebKitGTK
  4.1 pkg-config metadata is absent. No Linux native application was built or
  launched. Native SSH managers remain macOS-only; x86 does not enable them.
- Prepared `docs/x86-lab.md` with host evidence, outer-VM resources, disk path,
  loopback management/console ports and acceptance boundaries. Requested explicit
  approval for QEMU installation and this VM's creation/start; no VM mutation
  or package installation has occurred. Credentials, inner guests and restores
  need separate concrete approval. No Proxmox endpoint was contacted.

## External inputs and limits

- Historical Apple M1 host reports nested virtualization unsupported. Current
  Ryzen x86 host now runs the approved disposable Proxmox lab described in
  `docs/x86-lab.md`; inner guest execution remains unproven.
- Live version/node/empty-inventory reads now pass against the approved lab
  with an explicit CA and dedicated token. Known-template visibility and
  preflight still need a prepared catalog and scoped permissions. Follow
  `docs/debian-template.md`.
- Real clone/configure/start/snapshot/restore/cleanup operations need approval of
  the concrete disposable target and operations. Guest readiness remains untested.
- SSH/PTY on a real guest and iOS/Android require their own hosts/devices/toolchains.
- `../omniscient` and `../atfq` were inspected; `../distroproof` was not found.

## Remaining scope

Application provisioning, VM instances/lifecycle, snapshots, production desktop/
mobile SSH, authenticated remote control planes and signed distribution remain
incomplete. The macOS terminal is a fixture-tested prototype, not real-host proof.
Default tests work without configured Proxmox, user credentials or mutable VMs.
The macOS native target now includes the pinned russh candidate for handshake/
trust, explicit agent authentication and PTY transport (ADRs 0010–0013); no
production/mobile library guarantee is implied.

## Approved outer lab preparation — 2026-10-17

- User explicitly approved QEMU installation and creation/start of only
  `vmanyware-pve-lab` with the resources/network/disk in `docs/x86-lab.md`.
  This supersedes the earlier pending-approval status. Inner guests and
  credential creation remain outside this approval.
- Added `/.lab/` to `.gitignore`; `git check-ignore` confirms exclusion.
- Downloaded official amd64 Proxmox VE 9.2-1 over verified HTTPS from
  https://enterprise.proxmox.com/iso/proxmox-ve_9.2-1.iso into `.lab/pve/`.
  SHA256 is `4e88fe416df9b527624a175f24c9aa07c714d3332afb1ee3dbf3879573ef2c6c`,
  matching both the official release page and downloaded checksum. The ISO
  was renamed from `.part` only after hashing succeeded. No boot occurred.
- `download.proxmox.com` failed hostname verification; verification was never
  bypassed. The official enterprise download host verified successfully.
- Ports 18006/5901 were free; `/dev/kvm` is visible to this process. QEMU
  executables remain absent. `sudo -n pacman -S --needed --noconfirm
  qemu-system-x86 qemu-img` failed with `a password is required`, without
  installing packages. User was asked to run the package command locally.
- No virtual disk, VM process, credentials or service changes were made.
  No application tests were run for this preparation-only slice.

## Outer VM boot evidence — 2026-10-17

- User completed local package installation. Both QEMU system and image tools
  report 11.1.2. Read installed invocation help before selecting arguments.
- Rehashed the ISO successfully, checked free ports and refused any preexisting
  disk before creating `.lab/pve/pve.qcow2` (64 GiB virtual capacity).
- Started `vmanyware-pve-lab` as UID 1000, PID 171988, with q35/KVM, host CPU,
  eight vCPUs, 16 GiB RAM, virtio disk/network and user-mode NAT. No host bridge,
  service, firewall, shared folder or physical disk was changed. Actual launch
  arguments and PID are saved under ignored `.lab/pve/`; verify PID identity
  before future process operations.
- QMP query-status reports running; query-kvm reports present/enabled;
  query-cpus-fast reports eight CPUs. Screenshot inspection shows the Proxmox
  VE 9.2 ISO release 1 boot menu. No installer selection was made.
- `ss -ltnp` confirms this process alone listens on `127.0.0.1:5901` (VNC) and
  `127.0.0.1:18006` (forward to guest 8006). This is forwarding evidence, not
  an available Proxmox API. QMP is an ignored local Unix socket.
- VM remains running for the next installation step. No credentials or inner
  guests were created. Nested guest execution and installed Proxmox remain
  unproven; no roadmap checkbox was completed. No application code changed or
  application tests ran. `git diff --check` passed.

## Installer boot preparation — 2026-10-17

- User authorized continuing the outer installation. Sent Enter at the verified
  ISO boot menu through the owned QMP socket. The installer kernel booted;
  console shows DHCP assignment of 10.0.2.15 on enp0s2 from QEMU 10.0.2.2.
- No disk installation or credential creation was initiated. Asked the user
  to approve concrete hostname/network defaults and choose local root-password
  handling (self-entry over VNC or explicitly authorized generated local file).
  That answer remains required before dependent installation steps.
- Proposed hostname: pve-lab.vmanyware.test; address 10.0.2.15/24; gateway
  10.0.2.2; DNS 10.0.2.3; ext4/LVM on the sole 64 GiB virtual disk.
  Existing credential files were never read. VM remains running.

## Installed outer Proxmox lab — 2026-10-17

- User explicitly approved the proposed installation settings and generation
  of the local administrator password. Installed only on `/dev/vda` (the new
  64 GiB qcow2), ext4/LVM. Country France, Europe/Paris, US English keyboard;
  hostname pve-lab.vmanyware.test; nic0 bridged by installer vmbr0 with
  10.0.2.15/24, gateway 10.0.2.2 and DNS 10.0.2.3. Notification address
  root@vmanyware.test is a lab placeholder, not working email delivery.
- The generated 32-character random root credential is in ignored
  `.lab/pve/root-password`, mode 0600, used only for owned local-console login.
  No secret was printed. Initial rapid simulated keystrokes failed password
  confirmation; slower input and regenerated pre-installation credential
  succeeded. The final credential was validated by actual console login.
- Installer completed and automatically rebooted from disk. Console reports
  pve-manager/9.2.2/b9984c6d90a4bd80, kernel 7.0.2-6-pve. `pvesm status`
  shows local (dir) and local-lvm (lvmthin), both active. `ip -br address`
  confirms nic0 UP and vmbr0 UP with 10.0.2.15/24. `/dev/kvm` exists and
  `grep -c svm /proc/cpuinfo` returns eight. No inner guest was booted; nested
  execution remains unproven. Root console was logged out afterward.
- Exported only public `/etc/pve/pve-root-ca.pem` via the owned console to a
  temporary bounded loopback HTTP receiver. Receiver closed after receipt.
  CA SHA256 fingerprint:
  D6:11:0C:E7:1B:12:F5:B6:79:22:99:22:E9:19:34:E8:98:FE:84:0D:88:69:56:70:0E:5F:A0:A6:38:5D:1D:2D.
  curl with this explicit CA and hostname pve-lab.vmanyware.test resolved to
  127.0.0.1:18006 returned HTTP 200 from the login page with TLS verification.
  No system trust/DNS configuration was modified or verification bypassed.
- VM PID 171988 remains running; ss confirms both 5901 and 18006 still bind
  only to 127.0.0.1. This is an installed lab and verified login-page transport,
  not Rust/API-token inventory, template, provisioning or snapshot evidence.
  No API user/token or inner VM was created. No roadmap item was checked off.
- Updated lab instructions and state; `git diff --check` passed. No application
  source changed and no application check suite ran for this infrastructure slice.

## Inventory credential preparation — 2026-10-17

- Inspected existing probe and runtime configuration. Downloaded the installed
  Proxmox API schema with verified TLS: version/nodes/qemu-list GET endpoints
  allow authenticated users; VM visibility requires per-VM VM.Audit.
- Prepared `.lab/pve/create-reader.py` for only pve-lab: root HTTPS login,
  passwordless vmanyware-reader@pve, privsep inventory token, no ACLs, both
  expiring in 24 hours. Secret output is exclusive/0600 under ignored .lab.
  Script syntax parsed successfully; it has NOT been executed. Concrete
  credential approval was requested and remains pending.
- `cargo build --locked -p vmanyware-server --example proxmox_probe` passed.
  Bubblewrap read-only hosts-file override resolved pve-lab.vmanyware.test to
  127.0.0.1 without changing host DNS/hosts. No new dependency or production
  code was introduced. Live authenticated inventory remains untested.
- Next: after approval, run the prepared script in the isolated hosts namespace,
  then pass the generated token only in the probe child environment and record
  actual version/node/empty-template evidence. No template or VM operation is
  part of this slice. `git diff --check` passed.

## Live Rust inventory slice — 2026-10-17

- User explicitly approved the prepared account/token operation. Created only
  vmanyware-reader@pve (no password/group/ACL) and privsep inventory token on
  the disposable pve-lab. Both expire 2026-10-18 21:22:17 UTC. Secret output
  is `.lab/pve/inventory-token.json`, verified mode 0600, never printed.
- Root credentials/tickets were used over verified HTTPS only for approved
  creation and readback, not by the Rust probe. Administrative GETs confirmed
  privsep=1, matching expiry and no direct user/token ACLs. Token GET
  /access/permissions returned HTTP 200 with an empty object.
- Existing compiled Rust proxmox_probe exited zero: Proxmox 9.2.2/release 9.2,
  pve-lab online, no visible templates. Separate token GETs to version/nodes/
  nodes/pve-lab/qemu returned 200; qemu data was an empty array. This does not
  prove VM/template visibility or template permissions.
- All connections verified the lab CA and hostname. Process-local Bubblewrap
  hosts mapping avoided any host DNS/trust changes. Only probe child
  environment carried the token; no credential command arguments were used.
- Live invalid-token GET returned 401; TLS handshakes rejected an untrusted CA
  and wrong hostname. Rust probe also exited one with its expected fixed error
  for a bad token and absent lab CA. No mutation request was attempted with
  the reader token. Initial diagnostic helper wrongly assumed 401 bodies were
  JSON; it was corrected to inspect status without parsing error bodies.
- `cargo build --locked -p vmanyware-server --example proxmox_probe` passed.
  No application source/dependency changes; no broad test suite was repeated.
  `git diff --check` passed. Updated roadmap's live connection and TLS/token
  checks only; known-template/provisioning gates and Phase 0 remain open.
- Updated docs/proxmox-setup.md, docs/x86-lab.md, docs/feasibility.md and state.
  VM remains running; no inner guest/template, ACL grant, snapshot or restore
  was created. Next single task is the concrete template preparation plan.

## Frontend visual direction — 2026-10-17

- User selected sibling ../devutils as the temporary visual reference while
  its organization name may change. Read its src/style.css and index.html;
  did not modify that repository or copy its organization identity/assets.
- Bounded visual slice: adapted near-black background, cyan/violet accents,
  fine panel borders, translucent-looking gradients, monospace labels, light
  primary actions and visible cyan keyboard focus. Added a product header and
  phase badge; retained all existing actions, labels and component state logic.
  No dependency, external font request, WebGL or animation system was added.
- Changed apps/client/src/styles.css and main.tsx. Responsive controls wrap and
  stack on narrow screens; reduced-motion preference removes button transitions.
  Existing native terminal transport/rendering logic was untouched.
- `make check-client` passed TypeScript/Vite and all seven Node controller tests;
  `git diff --check` passed. Firefox headless screenshots of the built browser
  preview at 1280x1000 and 390x844 were visually inspected for header/initial
  controls. Captures are ignored under .lab/frontend-preview/. These are not
  full-page, native Tauri, terminal, physical keyboard or mobile-device checks.
- Used fresh disposable browser profiles and a temporary loopback static server;
  server stopped after capture. No native IPC, credentials or VM operation was
  invoked. The underlying application remains a Phase 0 prototype.
- Next single task remains the concrete Debian template preparation plan on
  pve-lab; further frontend layout work should follow a separate bounded slice.

## Workspace layout slice — 2026-10-17

- User asked to take the devutils-inspired frontend further. Kept one bounded
  workspace-layout slice: persistent desktop sidebar, wrapping narrow-screen
  navigation, compact workspace header, observed service/node/template summary,
  and adjacent definition editor/provisioning preview on wide screens.
- Navigation uses normal fragment links to focusable section headings. All
  components remain mounted: navigation does not reset YAML or intentionally
  close an SSH session. Active destination styling uses CSS :target/:has.
  Added a keyboard skip link, visible heading focus and native-size checkbox/
  radio controls. No routing library, new runtime dependency or provider action.
- Summary cards derive exclusively from current UI state: service starts as
  Not checked, node/template values are dashes until inventory is loaded.
  Browser preview is identified explicitly; historical lab results are never
  injected as current application data.
- Modified main.tsx, styles.css and section headings in ProvisioningPreview.tsx
  and SshHostTrust.tsx. Existing action handlers, native IPC and terminal
  transport remain unchanged. No sibling repository was modified.
- `make check-client` passed TypeScript/Vite and seven Node tests on the final
  changes; `git diff --check` passed. An isolated Firefox/Marionette browser
  smoke verified all five fragment links/focus destinations, unknown initial
  state, YAML edit persistence and actual Tab/Enter skip-link activation.
  No horizontal document overflow at actual widths 1440, 800 and 500. Firefox
  window resizing clamps requests below 500; no narrower automated assertion
  is claimed. Separate CLI screenshots at 390x844 and 1440x1100 were visually
  reviewed. Captures are in ignored .lab/frontend-preview/workspace-*.png.
- Early browser helper read fragment/focus before the default anchor action
  completed; separated click/read commands fixed the assertion. No production
  behavior change was needed. No native IPC or credentials were used.
- This does not validate macOS Tauri layout, live inventory states, error/result
  layouts or SSH continuity during navigation. Next frontend task: validate
  this workspace with loaded/error states and a live terminal in the isolated
  macOS native walkthrough. Infrastructure next remains the Debian template
  plan, not an authorized template/guest creation.

## Workspace state/lifecycle validation — 2026-10-17

- Host is Linux; macOS Tauri/WebKit execution is unavailable here. Continued
  the authorized validation by extending existing fixture scenarios rather
  than claiming native evidence or changing the product's support boundaries.
- Extended experiments/terminal-webkit/fixture.js: loaded one-node/one-template
  inventory; failed refresh clears counts; successful empty inventory shows
  zeros; validation error/retry; mixed passed/failed/unknown preflight; port
  changes invalidate results. All data/IPC is explicitly simulated.
- While xterm is mounted, four workspace anchors must focus their headings,
  preserve the exact terminal DOM element and cause no additional session
  start/close. Loaded preflight and terminal states assert no horizontal
  document overflow. Existing lazy-renderer cancellation/failure, split UTF-8,
  input/Ctrl-C, resize, StrictMode and stale-result checks remain in the runs.
- Executed this same extended fixture under isolated Firefox 158 on Linux
  using temporary Vite injection/module gating and a disposable Marionette
  profile. Both normal and renderer-failure scenarios passed at 1440 and 500
  CSS-pixel viewport widths (four scenario runs). Temporary adaptation lives
  under ignored .lab/frontend-preview/; it is not a new supported check target.
  Browser profiles and Vite instances were closed/removed by the harness.
- Extended experiments/terminal-native/scenario.js to navigate the same four
  anchors during each actual shell session, require the same terminal element,
  then require fresh AFTER_NAVIGATION_OK shell output. Syntax checked only;
  the modified native scenario and macOS WKWebView fixture have NOT run.
- Updated both experiment READMEs with the assertions and platform evidence
  boundary. `make check-client` passed TypeScript/Vite and seven Node tests;
  both scenario files passed node --check; git diff --check passed. No product
  source/dependency changes were needed and no Rust suite was repeated.
- No credentials, agents, SSH peers, Proxmox endpoints or VMs were accessed by
  these browser tests. Next frontend task: run make check-terminal-webkit and
  make check-terminal-native on a macOS GUI host for native workspace evidence.
  The Debian template plan remains the independent infrastructure next task.

## macOS transfer handoff — 2026-10-17

- User requested commit/push so the Mac can pull and execute the native checks.
  Reviewed the accumulated workspace/tests and x86 lab evidence documents.
  Remote main was still ca2db67, equal to the local base, before publication.
- Publishing only application source, fixture source and repository documents.
  All .lab runtime files (disk/ISO/credentials/CA/profiles/screenshots/helpers)
  remain ignored. No dependency lock or Rust source changed in these slices.
- Current checks: make check-client passes build/seven tests; extended normal
  and renderer-failure fixture scenarios pass in Firefox at 1440/500 widths;
  both fixture scripts parse; git diff --check passes. Existing x86 server and
  real read-only provider evidence is recorded separately above.
- README now gives the Mac handoff: git pull --ff-only, locked npm ci,
  make check, make check-terminal-webkit, make check-terminal-native. Then
  review the normal UI in one make dev session. Neither smoke needs the x86
  lab or any transferred credential. No new macOS execution is claimed.
- Next single frontend task: execute those checks in a macOS GUI session and
  record native workspace/terminal outcomes. Linux work can proceed later on
  the independent Debian template plan without treating native gates as passed.

## Native navigation smoke correction — 2026-10-18

- Reproduced the reported native failure at step 8 on macOS arm64 with
  `node experiments/terminal-native/check.mjs`. Added example-only assertion
  reporting to stderr; the exact failure was waiting for AFTER_NAVIGATION_OK
  after workspace navigation. All four destination-focus/terminal-identity
  assertions had already passed.
- The navigation scrolls the terminal offscreen, where xterm defers rendering.
  Corrected the scenario to scroll the retained terminal back into view before
  typing/asserting fresh shell stdout. The same real IPC/agent/shell scenario
  then passed step 21: two authentications, resize, Ctrl-C, reconnect, changed
  host rejection, unchanged trust and owned socket/shell cleanup. No product
  behavior, dependency, credential or infrastructure change was needed.
- Full `make check-terminal-native` passed after the fix, including the
  frontend TypeScript/Vite build, seven Node tests, release example build and
  native step-21 scenario. Example-specific locked Clippy with warnings denied, Rust formatting, scenario syntax and
  diff whitespace checks passed. The sandbox blocks the disposable agent;
  native GUI execution requires the approved unsandboxed command.
- This is local fixture evidence, not a real guest, personal agent, physical
  keyboard, packaged distribution or mobile check. Next single frontend task:
  run `make check-terminal-webkit` for the extended macOS simulated-IPC states;
  the independent infrastructure task remains the Debian template plan.

## Debian template preparation — 2026-10-18

- User reports all Mac tests green after edits and authorized continuing the
  remaining work. Clean workspace already contains 759aac5: example-only native
  failure reporting and scrollIntoView before checking post-navigation xterm
  output. Its recorded real macOS native run passed step 21. User's broader
  all-green report is accepted as user-reported evidence, not a local Linux
  rerun or real-guest/mobile proof.
- Current host remains Linux. Owned outer QEMU PID 171988 still has loopback
  ports 18006/5901. Verified-TLS read-only lab observations: no VM/container
  resources; nextid 9000 available; local-lvm active/enabled with image content
  and 21,302,870,016 available bytes; vmbr0 active over nic0. Requested custom
  role VMAnywareTemplateReader does not exist. Observations reserve nothing.
- Downloaded official dated Debian 13 genericcloud amd64 20261015-2632 image,
  SHA512SUMS and build JSON to ignored .lab/pve/images/. Exact SHA-512 matches
  the HTTPS manifest (recorded in docs/x86-template.md). qemu-img confirms qcow2,
  3 GiB virtual size and no backing file; metadata JSON parses. No image boot,
  mount, import or guest filesystem change occurred.
- Prepared docs/x86-template.md: create only VM9000 on pve-lab, import/expand
  scsi0 to 8 GiB on local-lvm, Cloud-Init/debian/NoCloud, 2 cores/2048 MiB,
  serial console and vmbr0, convert while stopped and never booted. Separate
  approval also covers a VM.Audit-only role and nonpropagating /vms/9000 grants
  for existing reader user/token. No token regeneration or expiry change.
- Prepared ignored .lab/pve/templates.yaml mapping this candidate; not claimed
  live-validated. Template DHCP is dormant, not proof of nested guest routing.
  A clone/start slice must establish an explicitly approved usable guest network.
- Reviewed installed qm/pveum manuals fetched with explicit CA/hostname trust.
  All three proposed shell blocks pass bash -n without execution. No product
  source changed, so no application suite was repeated; git diff --check passed.
  Read-only observations used only the lab's generated root credential over
  verified HTTPS, without logging it or inspecting personal credentials.
- Next: explicit approval of this concrete template/ACL operation, then create
  and verify stopped template metadata through the scoped reader/Rust probe.

## Real unbooted Debian template — 2026-10-18

- User explicitly approved the concrete template/ACL operation. This approval
  persists for that target/operation; no repeated confirmation is needed. It
  does not authorize arbitrary VM creation, clone/start, network changes,
  token renewal or deletion.
- Rechecked 9000 availability, active image-capable local-lvm, vmbr0 and absent
  role. Downloaded the same pinned Debian image inside pve-lab over verified
  HTTPS; required the exact SHA-512, qcow2/no backing file/3 GiB before import.
  Created only 9000, imported scsi0, expanded it to 8 GiB, added Cloud-Init and
  configured 2 cores/2048 MiB/debian/NoCloud/ciupgrade=0/serial boot/vmbr0.
- Verified stopped/configured state by HTTPS API before conversion. Converted
  to template; final disk is local-lvm:base-9000-disk-0,size=8G, Cloud-Init disk
  is vm-9000-cloudinit. No guest password/key, extra disk, hooks, passthrough or
  custom QEMU args. No start was issued, so the template remains unbooted.
- Created VMAnywareTemplateReader with VM.Audit only; exactly two direct
  nonpropagating /vms/9000 ACLs for vmanyware-reader@pve and its inventory token.
  Administrative readback confirms the role/grants, privsep and unchanged expiry.
  Reader's own effective-permissions endpoint reports only VM.Audit at that path.
- Reader GETs list exactly one stopped template and return its current config
  HTTP 200. Existing Rust proxmox_probe exits zero with 9.2.2, pve-lab online
  and Template 9000: vmanyware-debian-13-20261015. Explicit CA/hostname checks
  remain enabled; token is passed only in child environment, never arguments.
- A diagnostic helper initially used an unsupported administrative token-
  permissions URL; replaced it with role/ACL metadata readback and the reader's
  own effective-permissions endpoint. No mutation was repeated after this error.
- Script hashes were compared through the owned QMP console before each stage.
  Temporary public-only loopback script server stopped; root console logged
  out. Outer VM remains running. All image/runtime/credential/evidence files
  stay ignored under .lab/pve/. No private user credentials were inspected.
- Updated template/lab/setup/feasibility docs and roadmap known-template item.
  No product source changed; actual infrastructure checks above are the
  verification for this slice. Shell syntax and git diff --check pass; no
  application suite was repeated. Catalog is prepared locally, not live
  provisioning/preflight proof. Phase 0 remains incomplete.
- Next single task: propose an isolated guest-network/clone plan. No DHCP/SSH
  reachability, nested guest boot, Cloud-Init completion, snapshot or restore
  is implied by a readable stopped template.

## Isolated clone plan — 2026-10-18

- Continued with the requested network/clone preparation. Live read-only lab
  observations confirm VMID 201 available, vmbr1 absent, template 9000 stopped,
  active local-lvm with 20,301,635,126 bytes available. Host port 18222 is free;
  inspected host routes show no 192.0.2.0/30 route. Recheck all inside/outside
  routes, listeners and resources immediately before execution.
- Prepared docs/x86-clone.md: full clone 9000 -> 201, 2 cores/2048 MiB/12 GiB,
  runtime-only private vmbr1 at 192.0.2.1/30, guest 192.0.2.2/30 without default
  gateway. Literal-address SSH through a fixed Proxmox Python relay and owned
  outer QEMU hostfwd on 127.0.0.1:18222 avoids NAT/DHCP, package installation,
  Linux host networking changes and management-bridge reloads. DNS may inherit
  host defaults; no guest Internet or working DNS is assumed by this smoke.
- Plan includes exactly one newly generated disposable SSH identity (no personal
  agent/config/key), serial capture attached before first CPU execution, trusted
  console host-fingerprint comparison, strict SSH, Cloud-Init/resource/PTY checks
  and graceful shutdown/owned transport cleanup without deleting the clone.
- Read installed qm documentation for freeze-at-startup; owned outer QMP help
  confirms hostfwd_add syntax. info usernet shows only existing management
  forwarding. No new bridge, listener, forward, identity or VM was created.
- Static-address manual clone does not match current DHCP-only product preview.
  Documented this limitation rather than claiming application provisioning.
  No product source changes; two shell blocks pass bash -n, git diff --check
  passes. No application suite was repeated for this preparation-only slice.
- Next single task: approval of the concrete clone/network/SSH-identity and
  graceful-shutdown operations, then execute the prepared smoke in stages.

## First real nested clone/SSH smoke — 2026-10-18

- User approved the concrete docs/x86-clone.md plan. Rechecked all target/
  storage/network/port collisions; created only runtime vmbr1 and full clone
  201 from template 9000. Used root lab HTTPS API for clone/config/resize/
  frozen start/continue/shutdown with bounded task polling and recorded UPIDs.
  No product mutation method, reader ACL expansion or new API token.
- Generated one new disposable Ed25519 key in ignored .lab/pve/clone201, mode
  0600. Only its public key was supplied through Cloud-Init. No personal key,
  agent or SSH configuration was accessed. Outer forward bound only to host
  127.0.0.1:18222; owned Python relay bound only to 10.0.2.15:18222, reaching
  guest 192.0.2.2:22. No NAT/DHCP, management/host network reload or package install.
- QEMU confirmed prelaunch before the serial recorder attached; guest final
  Cloud-Init stage completed at about 6.8s after CPU continuation. Captured
  Ed25519 host key independently matched SSH endpoint before isolated trust
  was written. Strict selected-key SSH/reconnect pass; mismatched known-host
  data rejected with 255, without changing any guest key.
- Real guest resource assertions pass: Debian 13, debian, 2 CPUs, 1974 MiB RAM,
  12 GiB disk, 12,468,432,896-byte expanded root filesystem, private /30 route
  and no default route. Linux OpenSSH PTY passes 24x80 -> 43x132, foreground
  sleep Ctrl-C/status130 in 0.022s, usable shell and reaped SSH child. No
  remaining debian sleep process. This is separate from native macOS Tauri.
- Strict initial readiness failed correctly: cloud-init 25.1.4 returns 2,
  done/degraded done, errors=[], recoverable DEPRECATED scalar user warnings.
  Root HTTPS readback confirms Proxmox emits user: debian. Warning is recorded,
  not suppressed; separate SSH/resource checks do not constitute clean readiness.
  Leave phase-0 clone/readiness checkbox open pending a deliberate correction.
- Initial PTY harness expected the remainder of an interrupted interactive Bash
  list to execute. A separate post-prompt status command corrected that test;
  final real interruption passes. No product source change was needed.
- Graceful shutdown task OK; 201 and template9000 both stopped. Owned transport
  PID exited, no attached bridge links remained, vmbr1 removed, only added
  outer forward removed. Temporary script/serial receiver stopped and root
  console logged out. Clone/disk/identity retained; freeze=1 and absent vmbr1
  mean restart requires deliberate restoration. No snapshot/restore/deletion.
- Updated docs/x86-clone.md, template guide, feasibility and roadmap with exact
  evidence/limits. No application source/dependency changed or broad suite
  repeated; infrastructure assertions above and git diff --check are checks
  for this slice. Raw logs/keys/tasks stay ignored, not committed.
- Next single task: investigate scalar-user Cloud-Init generation and define
  a compatible correction/explicit readiness policy before another guest test.

## Offline Cloud-Init compatibility correction — 2026-10-18

- Confirmed Proxmox generator's scalar user output against upstream source and
  the lab's generated data. Cloud-Init 25.1.4 internally maps that scalar to a
  name dictionary but emits a recoverable deprecation. Proposed minimal change
  is user: {name: debian}, retaining users:[default] and all other allowed data.
- Added experiments/cloud-init-user/convert.py, check.py and README; ignored
  generated Python __pycache__ directories. Converter
  accepts only observed passwordless/default-user data and explicit matching
  expected username; rejects unknown features/passwords/upgrades; writes a new
  0600 output exclusively. It has no provider access and never executes data.
- Downloaded exact upstream 25.1.4 source archive into ignored .lab; SHA256
  fa70a77fc3cd3167a051e9ab04af4d4f56d3ffa0deb320735c889a6a367d3a3d.
  Installed upstream Python requirements only into an experimental venv. No
  application/system dependency was added. Local tested packages are recorded
  under .lab/cloud-init-compat/tested-python-packages.txt.
- Actual upstream normalizer/schema check passes: original scalar causes one
  deprecation, mapping causes none, normalized user/groups/default/sudo/shell/
  locked-password values are identical. Unexpected input and file-overwrite
  guards pass. Applied the same conversion offline to real stopped-201 generated
  data: schema valid, identity equivalent and exact public key retained.
- Lab read-only observations: local storage content currently import/backup/iso/
  vztmpl (no snippets); 202 available; local-lvm free 19,140,628,710 bytes; both
  201 and 9000 stopped. No Proxmox package, storage, VM, network or credential
  mutation was made. No old guest cache was reset and no warning was suppressed.
- Prepared docs/cloud-init-compatibility.md with exact fresh-202 retest: scoped
  local snippets capability/file, converted 202-specific user-data, same bounded
  private SSH topology, strict first-boot exit-zero/error-free acceptance and
  graceful shutdown/transport cleanup. This requires concrete approval; the
  existing 201 candidate must not be reused for a differently named/keyed clone.
- Added link to clone evidence. Offline semantic/schema/CLI guards and README
  shell syntax passed; git diff --check passed. No product source changed, so
  no application suite was repeated. First-boot compatibility remains unproven.
- Next single task is approval/execution of that fresh-202 retest. Keep the
  phase-0 readiness gate open until actual guest exit-zero evidence exists.

## Fresh Cloud-Init compatibility retest — 2026-10-18

- User approved the concrete 202 retest. Rechecked unused VMID/storage/ports;
  full-cloned 9000 to 202, configured 2 CPUs/2048 MiB/12 GiB, new disposable
  key and private static /30. Clone/resize/frozen-start/shutdown tasks all OK.
- Converted fresh 202-specific user-data, verified exact key/hostname and
  upstream schema/normalization. Added only local storage snippets capability,
  exclusively created root-owned 0600 snippet and bound only 202 cicustom user.
  Generated network/metadata retained; no template/201/package changes.
- Serial capture preceded CPU continuation. Initial key check correctly refused
  before the boot block was available; after capture the endpoint key matched.
  Strict selected-key SSH and reconnect passed; wrong pinned key rejected 255.
- Actual first-boot Cloud-Init 25.1.4 exit 0, done/extended done, no fatal or
  recoverable errors globally or per stage; final completion at 6.95s. Consumed
  user-data SHA256 equals prepared snippet (recorded in compatibility document).
- Debian 13/debian, 2 CPUs, 1974 MiB, 12 GiB disk and 12,468,432,896-byte root
  filesystem verified; no default route. PTY 24x80 -> 43x132, Ctrl-C/status130
  in 0.036s, usable shell/reaped child/no leftover sleep.
- Graceful shutdown OK; final API readback confirms 202/201/9000 stopped.
  Owned relay/recorder exited, runtime bridge removed, added loopback forward
  removed, HTTP receiver stopped and root console logged out. Retain stopped
  202/disks/snippet/storage capability/key; freeze=1 and absent vmbr1 require
  deliberate restoration before another start. No snapshots/restore/deletion.
- Updated compatibility evidence, experiment README, feasibility, roadmap and
  ADR 0017. Phase-0 manual provisioning prototype gate is now evidenced;
  application execution, least-privilege mutation ACLs and mobile remain open.
- Checks: actual infrastructure assertions above, offline upstream semantic/
  schema/guard checker and git diff --check. No product source changed; no
  broad application suite rerun. Next single task: concrete snapshot test plan.

## Snapshot experiment preparation — 2026-10-18

- Read STATE.md, roadmap and phase-0 brief; scoped this slice to preparation.
- Verified by trusted lab HTTPS API: 9000/201/202 stopped, 202 exact disk/network/
  snippet/freeze configuration, snapshot list only current, snapshot feature
  hasFeature=1, active enabled nonshared LVM-thin pve/data, available bytes
  17,977,492,007. No VM/storage/network/credential mutation was performed.
- Added docs/x86-snapshot.md: exact named stopped-disk snapshot on 202, existing
  strict SSH identity/trust, A/B fsynced guest marker plus post-snapshot file,
  explicit stopped rollback then guest A/absence proof, bounded task observation,
  capacity/metadata/config gates and graceful shutdown/owned transport cleanup.
- Retain snapshot and stopped target; no deletion/live RAM snapshots/ACL changes.
  External user-data snippet is preserved and is not claimed as snapshot backup.
- Official upstream LVM-thin docs and QEMU API consulted; installed API behavior
  and thin-pool metadata remain execution gates, not asserted runtime evidence.
- Validation: git diff --check. No application source changes or suite rerun.
- Next single task: explicit approval and execution of this concrete plan.

## Product naming — 2026-10-18

- Standardized product/UI/docs, Rust/npm package names, Tauri identifiers,
  environment variables, YAML API namespaces, fixtures and scripts on VMAnyware.
- Existing databases, app trust directories and environment configuration need
  explicit migration/reconfiguration; no automatic migration is implemented.
- Historical lab identifiers in documents use current branding. Ignored local
  records and external resources retain actual names; verify these before any
  operation, particularly TLS hostname/snippet binding and snapshot preflight.
- Next single task remains the concrete 202 snapshot plan, resolving actual
  external identifiers from VMID and local lab records before execution.
- Naming verification: make check-client passed (build + 7 tests); make
  check-server passed (fmt/clippy + 50 tests). Locked Cargo metadata, workspace
  formatting, changed JS syntax and git diff --check pass. Case-insensitive
  source/document search finds no former product name outside ignored artifacts.
- Full native workspace check cannot complete on this Linux host: GTK/WebKit
  development libraries are missing. No native application launch was performed.
- make check-ssh: fmt/clippy pass, 9 tests pass; the existing real-shell PTY
  fixture fails waiting for its expected READY line framing (actual READY was
  received). No renamed strings occur in that assertion; left this independent
  Linux fixture issue unchanged. Native macOS validation remains to be rerun.

## Phase 0 boundary and operation history — 2026-10-18

- User requested continued sequential work across phases, grouping checks where
  useful. No Proxmox/VM/network/credential operation was executed in this slice;
  the prepared 202 snapshot/rollback still has no execution evidence.
- ADR 0019 deliberately limits the initial MVP terminal to macOS. Mobile and
  Linux/Windows native terminals remain unsupported pending separate evidence.
  Updated platform/feasibility documents and the mobile-limitations roadmap
  item. Phase 0 stays open; no mobile build/runtime success is claimed.
- Corrected the existing isolated SSH real-shell READY framing: printf now
  emits an explicit leading newline rather than relying on OS command echo.
  Linux real-shell stdout/resize/Ctrl-C/reconnect now passes. No SSH runtime
  code, dependency, real host, personal key or agent was involved.
- Added GET /api/v1/operations: latest 100 summaries, deterministic descending
  creation-time/ID order, no inventory result bodies/request keys. Existing
  detail reads retain full results. Database work remains off the async executor;
  history never contacts Proxmox, claims work or retries mutations.
- Added explicit native history loading with empty/error/interrupted states;
  port-keyed unmount invalidates pending results. Added typed Tauri command and
  an HTTP decoding test. Native Rust additions are source-prepared only here.
- Verification: make check-server passed fmt/Clippy and all 51 server tests.
  New history test covers 107 persisted records, reopening, tied timestamps,
  response bounds/payload exclusion, unchanged queued/interrupted records and
  POST rejection. make check-client passed TypeScript/Vite and the existing
  terminal-controller Node test script. No graphical history walkthrough ran.
- make check-ssh passed fmt/locked Clippy and all 10 tests enabled on Linux
  after approved rerun outside sandbox (initial localhost bind was denied).
  macOS-only agent tests are not part of that Linux count.
- make check-rust formatting passed, but workspace Clippy stopped in native
  dependencies: WebKitGTK/JavaScriptCoreGTK 4.1 development packages missing.
  New native command/test did not compile or run; no native launch is claimed.
- Phase 1 command-setup checkbox now reflects existing commands and recorded
  evidence. No lifecycle/provisioning/Phase 2 completion claim was added.
- Remaining history limits: most recent 100 only, no pagination/retention or
  cancellation; records remain addressable by ID. No schema/dependency change.
- Next single task: compile/test and walk through native operation history on
  an equipped native host (empty history, failed/interrupted refresh, port edit
  during loading), then resume the concrete 202 snapshot execution gate.

## Phase 2 — Explicit read-only VM status — 2026-10-18

- Resumed from state/roadmap/phase-0 brief. Current Linux host still lacks
  WebKitGTK/JavaScriptCoreGTK 4.1; sudo requires a password. No package was
  installed and native history compilation/walkthrough remains open. Continued
  with a server-only vertical slice rather than inferring native success.
- Scope/acceptance: read a precise validated node/VMID; use verified TLS and
  bounded existing transport; reject wrong provider identity; distinguish power,
  QEMU execution and unproven guest readiness; never perform VM mutations.
- Added validated VmReference and typed power/QMP observations. New GET
  /api/v1/provider/nodes/{node}/vms/{vmid}/status makes exactly one provider GET.
  Checks returned VMID, omits unrelated fields, normalizes unknown QMP states,
  preserves missing QMP as null and rejects invalid power/identity responses.
- API validates canonical decimal VMIDs/node before provider access, catches path
  decoding rejection, and returns fixed safe validation/provider errors. It
  neither registers ownership nor infers readiness, persistence or authorization.
- Added static log route for VM status and corrected missing history route in
  diagnostic allowlist. Real child-process tests prove dynamic node/query text
  stays out of request logs and both route patterns are recorded correctly.
- Documented contract/limits in docs/vm-status.md and linked from docs/api.md.
  Consulted upstream QEMU API implementation for status/current and VM.Audit;
  installed-lab permissions/runtime behavior are not newly verified here.
- Checks: make check-server passed fmt/Clippy and all 55 server tests. Four new
  tests cover target bounds, power/QMP states, unknown/missing/malformed fields,
  wrong VMID, exact GET path/no extra request, untrusted CA, safe HTTP errors,
  timeout and method refusal. After expanding diagnostic assertions, its real
  subprocess log/redaction/restart test passed again. git diff --check passed.
- No dependency/schema change, real infrastructure request, credential access,
  VM mutation, native build or application launch. Phase 0 snapshot/desktop
  integration and Phase 2 managed ownership/start-stop remain open. Marked only
  the existing Axum/versioned-route foundation item evidenced, not Phase 2 done.
- Next single task: persist an explicit managed-instance/provider reference and
  definition hash before implementing durable start/shutdown intent and task
  reconciliation. Keep native history verification as a host-dependent gate.

## Phase 1/2 — Persist local instance references — 2026-10-18

- Read current state, domain/phase roadmap and phase-0 brief. Scope: persist an
  explicit definition/target association, retain the exact definition snapshot,
  reject duplicates/stale inputs and read it after restart. No provider mutation.
- Added instance registration types and SQLite migration 3. An instance keeps
  canonical definition YAML/hash/revision, generated ID, endpoint-scoped provider
  reference, node/VMID, creation time and registered status. Stored definition
  schema/hash/name are checked in the registration transaction before copying.
- Provider scope is SHA-256 of the canonical configured HTTPS origin. Tokens,
  CA bytes and URLs are not copied into the record. Token rotation retains the
  scope; different endpoint/port changes it. This is not cluster identity or
  authorization; aliases, VMID reuse and node movement need reconciliation.
- Unique (scope, VMID) prevents duplicate local associations across nodes and
  concurrent database connections. Expected definition hash rejects stale
  clients; snapshots survive future edits to the saved definition. A definition
  can still be reused for several instances. No existing record is overwritten.
- POST /api/v1/environments performs local registration only, requires provider
  configuration for scope derivation, rejects Origin writes and unknown fields,
  returns fixed safe errors and responds 201/Location after commit. No provider
  network access or VM existence/ownership/readiness claim is made.
- GET /api/v1/environments returns the newest 100 deterministic summaries without
  YAML; GET /api/v1/environments/{id} returns the retained detail. Both work with
  provider configuration absent and preserve older records by ID. No native UI,
  deletion, pagination, retention or lifecycle execution was added.
- Documented contract and migration in docs/instances.md, API/definition docs and
  ADR 0020. Roadmap annotations distinguish this preparation from unfinished
  provisioning/lifecycle items; none of those gates was marked complete.
- Verification: make check-server passed formatting, warnings-denied Clippy and
  all 63 server tests. Eight new tests cover v2->v3 migration preserving old
  definitions/operations, clean creation/reopen, immutable retained snapshots,
  invalid/stale/missing inputs, concurrent connections, scope isolation/token
  rotation, bounded tied-timestamp lists, HTTP rejection/redaction and detail
  reads without configuration. A listening localhost fixture confirms no provider
  connection occurs during HTTP registration/list/detail. Existing v1 migration
  and newer-schema refusal still pass. Subprocess tests verify static environment
  route logging without names/IDs/queries/body text. git diff --check passes.
- No runtime dependency, credential access, real Proxmox request or VM/network
  mutation. Server-only change: no frontend/native suite or launch repeated.
  Native history verification, phase-0 snapshot/desktop integration and real
  least-privilege lifecycle testing remain open.
- Next single task: expose a read-only observation for a registered instance,
  refusing a changed configured provider scope before any network access and
  using its persisted node/VMID. Ownership verification and durable lifecycle
  intent/task reconciliation must still precede any mutation implementation.

## Registered-instance observation preparation — 2026-10-18

- User requested consecutive slices without a confirmation turn between them.
- Added GET /api/v1/environments/{id}/status using the saved node/VMID. Configured
  provider scope must match before network access; invalid persisted targets fail
  closed. Observation returns separate instance/scope identity and VM power/QMP
  data; registration remains unchanged and no ownership/readiness is inferred.
- Two focused generated-TLS tests pass for exact target/no retargeting, unchanged
  registration, changed-scope refusal and malformed stored-target refusal. Added
  malformed/unknown-ID and method checks; full checks are grouped with next slice.
- Next slice: durable asynchronous instance observation through the existing
  read-only worker, binding idempotency to kind/instance and preserving interrupted
  results after restart. No provider mutation or generic job payload is planned.

## Durable instance observation — 2026-10-18

- Continued sequentially into a second slice in the same turn at user request.
  Added POST /api/v1/environments/{id}/refresh returning 202/Location before
  provider work. Existing single worker handles fixed inventory/instance reads.
- Migration 4 retains all inventory operation IDs/keys/results/timestamps and
  instance snapshots, adding an instance FK/kind constraint and per-instance
  active-read uniqueness. Idempotency now binds globally to kind and instance;
  cross-kind/target key reuse conflicts instead of returning unrelated work.
- Worker freshly checks provider scope and stored target, persists scoped
  power/QMP results or fixed failure codes, and leaves registration unchanged.
  Interrupted reads remain interrupted; queued reads may resume after restart.
  No provider mutations, UPID linkage, ownership/readiness claim or new dependency.
- make check-server passes fmt/Clippy and all 73 tests. New focused tests cover
  scope/target checks, async HTTP acceptance, kind/target idempotency, concurrent
  enqueue, distinct-target queues, v3->v4 preservation, interrupted recovery,
  safe provider failures and exact single GET/no replay of completed reads.
- Documented ADR 0021 and API/storage semantics. Next slice: explicit client
  instance list and resumable observation, with a disposable simulated-IPC browser
  walkthrough; native Rust compilation remains gated on missing WebKitGTK.

## Client observation and real-process workflow — 2026-10-18

- Continued into client and integration slices without a confirmation turn.
  Client explicitly lists registered instances and starts/resumes scoped read
  operations. A per-port/instance non-secret request key survives failed/paused
  monitoring and is cleared only after a confirmed terminal state.
- Added typed Tauri list/begin/read commands and prepared HTTP contract tests.
  Result kind/job ID/instance binding is checked in Rust; renderer additionally
  checks scope/node/VMID. Source preparation is not native compilation evidence.
- Added six Node observation-controller tests for polling, deadlines/same-key
  resume, stale completions, leaving during delays, unrelated responses and
  transport failure/interruption. Existing seven terminal tests remain passing.
- Added opt-in make check-instances-browser: owned Linux headless Firefox,
  temporary profile/loopback Vite and development-only injected fake IPC. Actual
  React interactions verify empty/list loading, queued/polled success, paused VM,
  safe scope failure, port-change stale result/error rejection, same-key resume
  and observation identity in history. No native commands/provider are invoked.
- Initial browser runs intermittently timed out with the list stuck Working.
  Stage diagnostics exposed first-click invalidation during StrictMode's initial
  effect cleanup. Instances/history now enable actions only after lifecycle
  setup. The fixture clicks at the first enabled DOM commit via MutationObserver;
  the final full make target and strengthened direct browser run both pass.
  Firefox's headless graphics diagnostic appeared on failed runs; no renderer
  failure is attributed to it. No owned fixture Firefox process remains.
- Added real binary subprocess workflow with cleared environment, generated
  HTTPS CA/peer and fake token only: HTTP definition save/register/202 enqueue ->
  actual background worker -> TLS GET -> persisted paused-VM observation. A
  second held provider read survives graceful SIGTERM/restart as interrupted;
  same-key polling returns the original record and never replays the GET.
  Instance snapshot stays identical. Final SIGINT exits zero; logs stay redacted.
- Extended static log coverage for scoped status/refresh and safe path rejection
  for malformed operation IDs. No real infrastructure/credential access, VM
  mutation, production fixture flag or new runtime dependency was introduced.
- Verification: make check-server passes fmt/warnings-denied Clippy and all 74
  tests; make check-instances-browser passes TypeScript/Vite, all 13 Node tests
  and the simulated-IPC browser walkthrough. Workspace formatting and git diff
  --check pass. Latest bundle is ~257 KiB plus lazy terminal ~335 KiB before gzip.
- make check-rust was attempted: formatting passes, native Clippy/build stops in
  missing WebKitGTK/JavaScriptCoreGTK 4.1 dependencies. Native additions/tests and
  normal native launch are still unverified. No container engine/socket is
  available here as an alternative; no system package or dependency was installed.
- Next single task: define and implement read-only ownership/identity preconditions
  for a registered instance before preparing durable start/shutdown execution.
  The saved association, matching endpoint/VMID and power state alone cannot
  authorize writes. Real 202 snapshot/restore approval, native integration and
  least-privilege mutation tests remain separate external gates.

## Read-only lifecycle preconditions — 2026-10-18

- User explicitly requested uninterrupted sequential continuation; an active goal
  now tracks authorized roadmap work without requiring another continue prompt.
- Implemented GET /api/v1/environments/{id}/lifecycle-preflight. It reuses scope/
  target checks and makes current-config -> pending -> power -> current-config
  GETs under one 20-second deadline. No schema/dependency/provider write added.
- Checks an exact sole instance tag, non-template, absent lock, no pending values
  or deletes (including force-delete 2), and unchanged config digest. Missing or
  malformed metadata fails closed; action preconditions can be passed/failed/
  unknown, while executionEnabled remains false in every response.
- ADR 0022 explicitly treats the marker as an administrative declaration, not
  independent ownership proof, and digests as observations rather than an atomic
  reservation. No marker/tag/ACL was installed or changed on infrastructure.
- Four focused HTTPS fixture tests and then make check-server passed (80 tests,
  fmt/Clippy). Tests cover exact read order, power/QMP distinctions, foreign/
  duplicate markers, locks/templates/pending deletions, changing digest, malformed
  data, scoped API blockers, unchanged registration and no write method.
- Final grouped make check-server also passes all 80 tests after conservative
  malformed-tag-list rejection and new subprocess static-route log assertions.
  git diff --check passes. No client/native code changed in this slice.
- Next slice: explicit client lifecycle preflight display with identity-bound
  results and stale-result suppression. Native compilation remains an external
  gate; do not infer ownership/readiness or enable writes from the new report.

## Client lifecycle preflight — 2026-10-18

- Continued from the verified read-only server preflight. Added an explicit
  per-instance Check lifecycle control, typed Tauri GET command and prepared HTTP
  contract test. It displays fixed local check labels and never exposes mutation
  controls or raw provider/server messages.
- Renderer validates instance/scope/definition hash/marker/config digest and
  node/VMID, exact known check set/statuses and executionEnabled=false. New reads
  clear old results; leaving/changing ports suppresses late preflight results.
- make check-instances-browser passes TypeScript/Vite, all 15 Node tests and the
  disposable Firefox simulated-IPC walkthrough. New checks cover lifecycle
  blockers, absence of mutation controls/raw messages and stale report rejection.
  No native command, real control plane or real provider was used by this smoke.
- Workspace formatting and git diff --check pass. Tauri command/test source is
  prepared; native compilation remains unproven pending the dependency attempt.
- Next bounded slice: try signed Arch packages in a temporary user-owned build
  prefix to compile/test native HTTP commands without root/system installation.
  If unavailable, retain the external gate and continue durable lifecycle work.


## Native Linux command compilation unblocked — 2026-10-18

- Classified prior goal turn as progress (server preflight implementation and
  passing evidence). This turn delivered its client display, then resolved the
  native build dependency gate without system/root installation.
- Queried public package metadata/local missing-dependency plan; downloaded seven
  exact Arch packages/signatures through verified HTTPS into the owned temporary
  /tmp/vmanyware-native-deps-61ktnqj7 prefix. Verified every signature against the
  public packaged Arch keyring (17 files, zero altered). Initial armored-keyring
  format failure was corrected by dearmoring a local copy; no trust bypass or
  personal GPG config/key/credential file was accessed. No package hooks ran.
- Only copied pkg-config paths were rewritten for that prefix. With its pkg-config
  and library paths, make check-rust passed fmt, all-target warnings-denied Clippy
  and all 95 workspace tests: 80 server and 15 native HTTP. The actual history,
  scoped observation and lifecycle preflight command contracts now run on Linux.
  A standalone client test-list build also succeeds (15 tests listed), independently
  of workspace server feature unification. No native graphical launch is claimed.
- No root package, system database, repository dependency/lock or real VM was
  modified by setup. Temporary helper/manifest/checksums/signature evidence stay
  in that owned /tmp directory; docs/native-linux-prefix.md records reproduction
  and limits. Future goal turns should check it still exists before using it.
- Client evidence: make check-instances-browser passes build, 15 Node tests and
  Firefox simulated IPC, including read-only lifecycle blockers and stale results.
  New native HTTP test uses a complete legitimate preflight shape. Workspace
  formatting/git diff --check pass. Main bundle ~259 KiB before gzip.
- Proposed ADR 0023 records the next actual lifecycle execution contract: explicit
  full-target/action binding, initially disabled capability, persistent dispatch/
  UPID linkage, no POST replay, uncertain outcomes and read-only reconciliation.
  This is planning only; no start/shutdown method, route or enabled write exists.
- Next single task: implement durable lifecycle action inputs/state/storage and
  fixed provider dispatch/task validation with local HTTPS fixtures, then wire
  worker/API only after recovery and no-replay behavior are verified. Keep the
  full roadmap goal active; real infrastructure mutations still need concrete
  approval. Native GUI/mobile/real snapshot and least-privilege gates remain open.


## Durable lifecycle intent storage — 2026-10-18

- Continued the active roadmap goal sequentially. Scope was storage and recovery
  invariants before wiring provider writes; no real infrastructure was accessed.
- Migration 5 preserves existing reads/associations while adding fixed start and
  graceful-shutdown intent, full confirmed input hash, server confirmation time,
  immutable dispatch acquisition and private validated task linkage. No new crate.
- Confirmations expire before dispatch after 120 seconds; retrying an identical
  key cannot extend this window. Changed input/key bindings fail. Per-instance
  queued/running/uncertain mutation exclusion permits diagnostic reads.
- Startup interruption after dispatch becomes uncertain, never an automatic POST
  retry. Known tasks permit future GET-only reconciliation; unknown references
  retain the exclusion. Undispatched running work becomes interrupted.
- Success persistence requires successful task and matching independent power
  evidence; no guest readiness inference. Existing read workers exclude mutation
  jobs, and generic read completion cannot complete them. No lifecycle enqueue
  HTTP route, provider write transport or execution worker is implemented yet.
- History/native contracts understand uncertain state and a task-known boolean;
  raw UPIDs remain private and Debug-redacted. Documentation records this boundary.
- make check-server passes fmt, warnings-denied Clippy and 88 tests, including
  eight new SQLite tests for binding, expiry/clock rollback, concurrency, dispatch
  and crash recovery, task validation/redaction, terminal immutability, corruption
  rollback and schema-4 preservation. Via the temporary signed-package prefix,
  make check-rust passes all 103 workspace tests (88 server, 15 native).
- Next single slice: fixed verified-TLS start/graceful-shutdown transport with
  retries disabled, exact returned task identity checks and local HTTPS tests.
  Wire durable worker/API only after no-replay recovery is covered. Real VM
  operations still require separate concrete approval; GUI/mobile gates remain.


## Fixed lifecycle provider transport — 2026-10-18

- Previous goal turn made verified progress: schema 5, eight lifecycle storage
  tests, passing server/client/native checks. This slice added actual fixed
  provider transport, without exposing service execution or touching real VMs.
- Checked current official upstream src/PVE/API2/Qemu.pm for POST paths, form
  parameters, task kinds and permission checks. An initial old source path failed;
  the correct source includes src/. No upstream source was copied into the repo.
- Added typed start and graceful-shutdown requests. Start sends an empty form;
  shutdown sends exactly timeout=60&forceStop=0. No force/skiplock/passthrough.
- Explicitly disabled reqwest retries on the shared verified-TLS client. Redirects
  remain refused. GET and POST share the bounded safe envelope decoder.
- Returned UPIDs must bind node, canonical VMID and qmstart/qmshutdown kind.
  Unexpected responses stay safe errors; callers must treat post-dispatch
  ambiguity as uncertain. No raw response/UPID enters public operation records.
- Extended the disposable HTTPS fixture to consume request Content-Length;
  three tests exercise both exact POST bodies/paths/auth, malformed/unbound tasks,
  permission/server/redirect errors, trust rejection, timeout and response bounds.
  Error fixtures see no repeated request. HTTP/2 retry signals are not simulated;
  explicit retry::never policy addresses internal client retries.
- make check-server passes formatting, all-target warnings-denied Clippy and all
  91 tests. git diff --check passes. Client source was unchanged in this slice;
  the earlier 103-test workspace result predates these three server tests.
- docs/lifecycle-execution.md records contracts, evidence and boundaries. No
  roadmap item is newly marked complete: integrated execution is still missing.
- Next single slice: default-disabled lifecycle executor linking fresh scoped
  preconditions to committed dispatch, persisted task and GET-only reconciliation;
  exercise local crash/no-replay flows before wiring the confirmed HTTP route.


## Durable lifecycle executor — 2026-10-18

- Previous goal turn was verified progress (fixed provider transport and 91
  passing server tests). This sequential slice links the durable state machine
  to actual local-HTTPS start/shutdown requests and subsequent observations.
- New lifecycle executor checks capability, retained provider scope/target, fresh
  preconditions and approved digest. Approval expiry is rechecked while committing
  the dispatch guard. It persists task linkage before polling, then separately
  verifies resulting power. Task failure is terminal; ambiguous post-dispatch
  outcomes are uncertain. No guest readiness/SSH success is inferred.
- Retained tasks reconcile by GET even with writes disabled; unknown references
  are never automatically reclaimed. Storage failures propagate rather than
  allowing the service worker to silently lose durable state. Database operations
  use spawn_blocking. The default worker invokes this only after idle read claims
  and always passes false for write capability. No HTTP action route exists.
- Six integrated HTTPS/SQLite executor tests exercise start/shutdown success,
  zero-POST disabled/scope/revision/expiry/foreign-marker rejection, provider task
  failure, malformed submission reply, independent power mismatch, same-key
  identity and conflicting uncertain actions. Local worker cancellation after
  POST receipt and after task linkage is followed by SQLite reopen/recovery; no
  POST repeats. Known-task diagnosis occurs with writes disabled. A seeded
  post-link crash boundary reaches success using task/power GETs only.
- These are async worker cancellation/connection-reopen fixtures, not actual
  process-kill or real provider mutation evidence. All credentials/certificates
  are disposable fixture data. No real VM, credential or infrastructure changed.
- make check-server passes fmt, warnings-denied Clippy and all 97 tests. Client
  code is unchanged; latest native workspace check remains the earlier 103-test
  result. git diff --check passes. Execution docs/ADR record the new boundary.
- Next single slice: validated default-disabled runtime capability and fair
  sequential worker scheduling, then confirmed enqueue API with local HTTP/TLS
  and real binary restart/no-replay coverage. Continuous read traffic currently
  may delay lifecycle reconciliation. Real-target authorization remains separate.


## Runtime lifecycle opt-in and fair worker rounds — 2026-10-18

- Previous turn made verified progress: integrated local executor and 97 passing
  server tests. This slice wires validated runtime capability into the real binary
  and removes read-queue starvation of lifecycle reconciliation.
- Config defaults lifecycle to false; VMANYWARE_LIFECYCLE_ENABLED accepts only
  exact lowercase true/false. Invalid values emit a fixed startup reason before
  provider/database initialization and never echo the value. Loopback-only binding
  is unchanged. Explicit true still requires valid persisted confirmation, scope,
  revision, marker/preconditions and expiry before any new provider dispatch.
- Main passes this capability to one sequential worker. Each round processes at
  most one read and one lifecycle claim; long bounded requests can delay work but
  continual reads cannot indefinitely exclude lifecycle claims. Idle rounds wait
  250 ms. Default library runner remains disabled; retained task GET reconciliation
  is independent of permission to start new writes.
- Tests: exact config parsing/default/redaction; repeated rounds completing both
  queues under true/false; actual cleared-environment binary startup rejects invalid
  settings without creating database/lock; absent/false/true binary runs process
  preseeded intent with no provider, proving option-to-worker wiring and persisted
  failures without acquiring dispatch markers. SIGTERM exits successfully.
- make check-server passes fmt, all-target warnings-denied Clippy and 101 tests.
  git diff --check passes. No client code, dependency, real VM or credential changed.
  Latest native full workspace evidence remains the earlier 103-test result.
- No enqueue HTTP route exists yet and preflight executionEnabled remains false.
  These subprocess tests do not prove real binary provider POST/restart recovery;
  async worker restart fixtures from the prior slice remain the current evidence.
- Next single slice: explicit full-target/action confirmed start/shutdown enqueue
  routes with default-disabled API policy and idempotency/error tests, then local
  TLS-backed real binary success/interruption/no-replay verification.


## Confirmed lifecycle HTTP enqueue — 2026-10-18

- Previous turn delivered verified runtime opt-in and fair sequential worker
  rounds. This slice exposes fixed POST /api/v1/environments/{id}/start and
  /shutdown with a complete explicit Approval body, global idempotency key and
  4096-byte bound. Origin, unknown fields and mismatched path/action/identity are
  rejected. Default routers disable execution; main shares validated capability
  with HTTP state and worker. No force-stop/generic mutation route exists.
- Handler checks capability, configured scope and saved target and commits queued
  intent before HTTP 202/Location, making no provider request itself. Same key/body
  returns the original operation; changed input/cross-kind/active conflicts fail.
  Expiry and fresh marker/config/power conditions remain worker responsibilities
  checked before the durable dispatch marker. Existing GET history/detail still
  retrieves outcomes if enqueue becomes disabled or provider configuration changes.
- Preflight API now explicitly reports configured capability (default false),
  independently of preconditions. Renderer/native contracts accept both booleans;
  renderer displays capability and still has no mutation control. An enabled
  report does not confirm or trigger any action.
- Two HTTP/TLS tests cover disabled/missing-key/origin/provider rejection, target/
  action/hash/VMID/scope binding, unknown/oversized input, idempotency/conflicts,
  zero-network enqueue, accepted request -> actual executor -> one fixture POST
  -> persisted task/power result, and enabled preflight reporting. Real binary
  diagnostic coverage additionally verifies static/redacted start/shutdown routes.
- Native full check initially failed its obsolete rejection-of-true assertion;
  updated that contract test to accept both boolean values while rejecting string
  capability/unrelated identity. Final prefix make check-rust passes fmt/Clippy
  and all 118 tests (103 server, 15 native); final make check-server also passes
  103 tests after added diagnostic coverage. make check-instances-browser passes
  build, all 16 Node tests and disposable Firefox simulated-IPC walkthrough.
  git diff --check passes. No new dependency, real VM or credential change.
- Real process lifecycle provider POST/restart behavior and approved real-host
  least-privilege execution remain unverified; previous async cancellation tests
  are not substituted for them. Native lifecycle commands/UI remain pending.
- Next single slice: generated-HTTPS real binary success, interruption with and
  without persisted task, restart/reconciliation and exactly-once dispatch evidence.
  Then add native explicit action confirmation/monitoring. Keep real tests opt-in.


## Real binary lifecycle start and restart — 2026-10-18

- Previous turn delivered confirmed HTTP enqueue and final 118 passing workspace
  tests plus client/browser evidence. This slice adds actual server-process
  lifecycle success/recovery evidence, beyond async worker cancellation fixtures.
- Added a disposable HTTPS lifecycle peer with generated certificate/fake token,
  explicit fixture marker, separately observed power/task state and one held POST
  or task-status response. Peer state stays in memory; it never touches a VM.
- Real cleared-environment binary saves definition/registers association, fetches
  enabled passing preflight, sends confirmed enqueue and persists task success/
  running power. Duplicate same-key requests return the same terminal operation.
- SIGTERM while POST reply is held leaves the durable dispatch marker but no UPID;
  actual process restart preserves uncertainty, same-key identity and new-key
  conflict. A 600 ms observation spans more than two idle intervals with no new
  provider request. Unknown submission is never automatically replayed.
- SIGTERM during the first task GET proves UPID linkage already persisted;
  actual restart with new writes disabled reconciles task/power by GET to success.
  Every scenario records exactly one POST; final SIGINT exits successfully,
  association snapshots stay unchanged and logs omit fixture token/raw UPID.
- make check-server passes formatting, all-target warnings-denied Clippy and all
  104 tests. git diff --check passes. Client/native source unchanged this slice;
  latest full workspace evidence remains 118 tests from the preceding slice.
- These are local start/normal exit/restart fixtures, not SIGKILL/power-loss fsync,
  real Proxmox, least-privilege ACL, shutdown subprocess or guest-readiness proof.
  Shutdown remains locally covered in the transport/executor TLS tests. No actual
  infrastructure/credential mutation occurred; real approval boundaries persist.
- Next single slice: typed native confirmed action command and bounded operation
  monitoring, then explicit target/action confirmation controls with same-key
  resume/uncertain handling and simulated-IPC browser evidence.


## Native confirmed lifecycle commands — 2026-10-18

- Previous goal turn delivered verified real binary local-HTTPS start/restart
  no-replay evidence and 104 server tests. This slice adds registered Tauri
  begin_lifecycle_action/read_lifecycle_operation commands, without a new UI flow.
- Typed deny-unknown-fields full approval accepts only start/shutdown, exact
  lowercase hash/ID formats, validated node/VMID and confirmed=true. Input/key
  validation occurs before network. Fixed loopback POST sends JSON and the original
  idempotency key; native reqwest retries are now explicitly disabled as well.
- GET-only operation resumption binds requested operation ID and complete original
  target context. Responses validate kind/instance/state/result/failure consistency,
  retained task presence and separate successful task plus matching action/scope/
  node/VMID/desired power. Only known failure codes pass; fixed app-owned lifecycle
  errors replace raw server messages. Unknown/inconsistent responses fail safely.
- Three native HTTP tests cover both exact action requests, JSON/key headers,
  uncertain and successful GET resumption, wrong operation/target/evidence, invalid
  inputs before transport, unknown fields and safe error mapping.
- Prefix make check-rust passes fmt, warnings-denied Clippy and all 122 workspace
  tests (104 server, 18 native). git diff --check passes. Frontend unchanged; latest
  frontend build/16 Node tests/Firefox simulated-IPC result is from the prior API
  slice. No dependency, real VM, infrastructure or credential changed.
- Native GUI/WebView lifecycle actions remain unverified. Source is compiled and
  command HTTP tests run on Linux; that does not prove a user-facing action flow.
- Next single slice: client confirmation and bounded lifecycle monitoring with
  same-key resume, explicit uncertain handling, stale target/port suppression and
  Node/browser simulated-IPC evidence. Never automatically reconfirm or replace
  a key after transport loss. Real-provider execution stays separately opt-in.


## Client lifecycle monitoring controller — 2026-10-18

- Previous turn delivered typed native commands and 122 passing workspace tests.
  This small sequential slice implements standalone client intent/monitoring logic
  before adding React controls; no automatic invocation or user-facing action
  was introduced.
- confirmedLifecycle builds a full target/digest/action/key only for an explicitly
  confirmed enabled report with passing common and action power/QMP conditions.
  Cached intent validation can additionally bind the current saved instance.
- monitorLifecycle snapshots context and requires persistence before transport.
  Unknown IDs reuse original approval/key; acknowledged IDs use GET only. Every
  response validates kind/ID/scope/action/task/power and known failure codes.
  Acknowledgements retain IDs even after leaving, without stale UI updates.
- Polling pauses after a monotonic 60-second interval plus at most one native
  bounded in-flight request. Leaving stops later reads. Uncertain returns a
  distinct outcome and retains context; terminal records also remain for explicit
  review. No error, pause or uncertain state automatically replaces a request key.
- Seven Node tests cover confirmation blockers and bound inputs, persistence before
  POST, one-ID polling, lost reply/same-key resume, GET-only known/uncertain resume,
  deadline/leave behavior, late ack persistence and malformed evidence rejection.
- Initial build rejected explicit .ts import syntax; enabled the compiler option
  under existing noEmit mode so direct Node source tests and TypeScript build use
  the same module. No runtime dependency or lock changed.
- Final make check-client passes TypeScript/Vite build and all 23 Node tests.
  git diff --check passes. Rust unchanged; latest workspace remains 122 passing
  tests. No native GUI, browser action flow or real infrastructure test is claimed.
- Next single slice: React explicit target/action confirmation and per-port/instance
  pending-intent persistence, resume and uncertain display. Protect stale views
  and block fresh confirmation while a prior intent needs review; then exercise
  the flow with disposable Firefox simulated IPC.


## React lifecycle confirmation and resume — 2026-10-18

- Previous turn delivered standalone lifecycle intent/monitoring logic and 23
  passing Node tests. This slice connects it to per-instance React controls.
- Enabled/passing preflight exposes review for allowed start/graceful shutdown.
  Review displays exact instance, service port, node/VMID, scope, definition and
  config digest; initially unchecked consent plus a separate confirm click is
  required. Cancel performs no command. Changed reports reset review.
- Full pending context persists by port/instance before transport; corrupt or
  unavailable storage blocks fresh actions. Existing pending intent prevents a
  new key. Known IDs resume by GET; unknown IDs explicitly explain same-request
  retry. Local collision checks do not claim cross-window transactional locking;
  server idempotency/exclusion/preconditions remain authoritative.
- Uncertain outcomes cannot be acknowledged/cleared into fresh actions. Terminal
  success/failure/interruption require explicit result acknowledgement, which
  clears only local pending context and invalidates preflight. Late ack retains
  its scoped ID without updating a departed port view. No automatic resume on load.
- Strengthened disposable Firefox simulated-IPC walkthrough proves start review/
  consent/cancel/success, exact scope/revision display, result acknowledgement and
  fresh-check requirement, lost reply preserved across port remount, original-key
  retry, uncertain GET reconciliation, late ack/stale suppression and graceful
  shutdown/stopped power. Initial fixture still expected the old no-actions text;
  updated its disabled-capability assertion and added enabled scenarios. Final
  make check-instances-browser passes build, all 23 Node tests and the walkthrough.
- git diff --check passes. Rust unchanged; latest 122 workspace tests remain valid
  native-command evidence. No actual native GUI/provider action is claimed, no
  real infrastructure/credential/dependency changed.
- Next single slice: inspect the available Linux native GUI/WebView harness and
  exercise lifecycle confirmation -> real Tauri IPC -> disposable loopback server
  if locally available. Keep GUI evidence distinct from Firefox simulation and
  real Proxmox tests; continue remaining provisioning roadmap work afterward.


## Real Linux native lifecycle UI/IPC — 2026-10-18

- Previous goal turn delivered React confirmation/resume and passing Firefox
  simulated IPC. This slice closes an additional local integration gap with real
  WebKit/Tauri IPC and actual server/provider transport, without touching real VMs.
- Verified/extracted three signed Arch display packages into the owned temporary
  prefix (libxmu/xorg-xauth/Xvfb); retained manifest/signatures in display-packages.json.
  No root/system package installation or credential inspection occurred.
- Added opt-in lifecycle_smoke example with production command module, built React
  assets, fixture-only scenario/reporting and separate ephemeral app identity.
  Runner owns Xvfb auth cookie/display, XDG directories, D-Bus, generated HTTPS
  certificate/key/fake token and actual Axum database/process. A read-only user
  namespace overlay supplies WebKit helper paths; existing home directories hide.
- Native UI checks real Linux bridge, list/preflight, review/cancel/consent/confirm
  start, task/running power, acknowledgement/fresh check and graceful shutdown to
  stopped power. Runner requires exactly one POST per action, unchanged association
  and safe server logs. IPC is never mocked. No production fixture flag was added.
- Initial Xvfb initialization crashed in host NVIDIA EGL; explicit software Mesa
  fixes this test backend. Initial OpenSSL fixture certificate CA constraints
  caused verified-TLS rejection; generated server-auth end-entity constraints fix
  it without trust bypass. Minimal D-Bus/a11y/portal warnings remain nonfatal.
  WebKit sandbox, TLS/hostname verification and X11 cookie auth stay enabled.
- Rustfmt found an ambiguous imported module test path; explicit control_plane/
  tests.rs path fixes formatting/all-target compilation. Final prefix make
  check-rust passes fmt/Clippy and 122 tests. Complete make check-lifecycle-native
  target passes frontend build/23 Node tests, native/server builds and real GUI
  start/shutdown fixture. git diff --check and Python syntax parsing pass.
- This proves isolated Linux software-rendered lifecycle UI/IPC. Normal packaged
  startup, native app restart/persistent profile, GPU, macOS/mobile, guest readiness
  and real least-privilege Proxmox mutations remain distinct open evidence gates.
- User again requested nonstop continuation; goal remains active without waiting
  for another prompt. Next single slice: inspect existing provisioning plan/
  preflight contracts and implement durable explicitly confirmed clone intent/
  execution against local HTTPS fixtures, preserving no-replay and real-target
  approval boundaries before Cloud-Init/resource/start orchestration.


## Phase 3 fixed full-clone transport — 2026-10-18

- User explicitly asked to keep going and try finishing the roadmap. Active goal
  remains unchanged; real/platform gates remain evidence-based rather than being
  marked complete to satisfy scope. Previous turn made progress with real Linux
  native lifecycle UI/IPC and passing full native/build/browser checks.
- New slice derives fixed CloneRequest from existing validated deterministic plan
  plus instance ID. Same-node full clone sends only newid/name/storage/full=1 and
  a generated instance/definition/plan description. No linked clone, arbitrary
  fields, tags/pool/snapshot/cross-node override or new public API route.
- Rechecked official upstream Qemu.pm: qmclone UPID binds SOURCE VMID, unlike
  start/shutdown target VMID. Transport validates exact source/node/kind/canonical
  ID. Wrong destination/source aliases, foreign node or action are rejected.
- Post-task target observation independently reads current config then status
  within 20 seconds: exact description/name/digest, non-template, no lock, stopped
  power and absent/stopped QMP. It does not claim ownership authorization, resource
  configuration, Cloud-Init or guest readiness.
- form_urlencoded is a new direct server dependency for dynamic multiline forms;
  it already existed transitively in Cargo.lock. Lockfile changes only the server
  dependency edge, with no new resolved version/crate. ADR 0024 records this choice
  and future durable stage/reservation/no-replay contract.
- Three HTTPS tests cover exact encoded form, source-task binding, invalid replies,
  auth/provider/redirect errors, timeout/size bounds, no observed replay and target
  identity/lock/template/digest/power/QMP failures. make check-server passes fmt,
  warnings-denied Clippy and all 107 tests; git diff --check passes. Native/client
  unchanged; latest full workspace evidence is the preceding 122-test result.
- No transport caller exists in service orchestration yet, and no real VM or
  credential was accessed/changed. Clone success cannot complete Phase 3 without
  durable provisioning intent, resource/disk/Cloud-Init stages and readiness.
- Next single slice: durable full-plan provisioning intent with canonical definition
  snapshot, local scope/VMID exclusion, per-stage dispatch/task retention and
  interruption recovery, then connect the clone stage to local HTTPS fixtures.


## Schema 6 provisioning intent and journal — 2026-10-18

- User authorized autonomous continuation after establishing diff-scoped checks.
  Scope: durable full-plan inputs, local target exclusion, ordered stage journal,
  clone dispatch/task guards and restart recovery. No provider execution was wired.
- Migration 6 adds vm_provision to existing operations and retains approval/hash/
  server timestamp, canonical instance definition snapshot, catalog and exact plan.
  Plan stages are persisted in order; only clone currently has claim/dispatch/link
  methods. Journal/task data is validated against regenerated retained inputs.
- Association allocation shares one immediate transaction with operation/journal.
  Existing scope/VMID uniqueness excludes manual registration/other provisioners;
  status registered continues to mean only local association, not VM existence.
  Incomplete provisioning blocks lifecycle writes through a database trigger,
  including failed/interrupted intent; diagnostic observations remain permitted.
- Global key/body binding returns original retained plan/timestamp on retry even
  with changed current catalog. New inputs must match current definition/catalog/
  plan. Dispatch commits workflow+clone stage markers once within original 120s
  approval; task linkage validates SOURCE qmclone identity and cannot be replaced.
- Startup recovery now transactionally updates running stages and operations.
  Known dispatched tasks permit future read-only reconciliation; unknown clone
  submissions remain uncertain and unclaimable. No confirmation is renewed.
- Verification: new intent suite first passed five of six tests; migration fixture
  used obsolete SQL column names, corrected to definition_yaml/definition_hash/
  revision and reran only that test successfully. New tests cover snapshots/order,
  binding/exclusion, dispatch/recovery/task redaction, expiry/clock rollback, corrupt
  claim rollback, concurrent keys and schema-5 lifecycle preservation/FK integrity.
- Five affected compatibility tests passed: schema-2 instance migration, running
  read restart, and three lifecycle recovery/task cases. Earlier schema-4 migration
  test also passed after updating expected schema to 6. Scoped library/new-test
  Clippy passes; its boolean assertion lint was corrected. Changed-file rustfmt and
  git diff --check pass. No broad server/client/native/GUI suite was rerun: earlier
  107-server/122-workspace evidence remains historical rather than a new full run.
- No dependency, real VM, infrastructure or credential changed. No provisioning
  HTTP route/worker or completed provisioned VM is claimed. Failed intent currently
  retains its target association until a future explicit recovery/release policy.
- Next single slice: clone-stage completion/failure/uncertain persistence and
  executor linking retained intent, fresh provider/plan checks, one POST and GET-only
  task/target reconciliation against local HTTPS fixtures. Keep creation disabled
  until later resource/Cloud-Init/readiness stages are implemented.


## Durable clone execution and completion — 2026-10-18

- Scope: finish only the clone stage against generated HTTPS fixtures, without
  exposing provisioning writes through an API/worker or advancing later stages.
- Executor checks current catalog/plan, seven observational preflight gates,
  source digest, lock/pending state and stopped power before one durable dispatch.
  Public preflight still reports unknown provenance/permissions/readiness; these
  clone gates do not establish full provisioning feasibility or guest readiness.
- Retained tasks reconcile by GET with new creation disabled. Unknown submissions
  cannot replay. Task failure and ambiguous task/target results persist distinct
  fixed failures; a five-second delay bounds known-task reconciliation frequency.
- Completion validates retained task, scope/instance/plan and exact stopped target
  observation. Only clone becomes succeeded with evidence; workflow stays running,
  result remains absent and later stages stay queued. Restart preserves completed
  clone evidence without allowing another clone claim. Reservations remain held.
- Verification: original six intent tests passed; two new outcome tests passed;
  completed-stage restart assertion then passed separately. Four generated-HTTPS
  executor tests cover disabled/changed inputs, source gates, task success/failure,
  bad target marker, ambiguous submission and disabled GET-only recovery. Nine
  affected preflight tests and scoped library/intent-test Clippy passed. No broader
  server/client/native suite was rerun; previous full-suite counts are historical.
- No real infrastructure/credential mutation, new dependency or production caller.
- Next single slice: fixed resource configuration transport and durable stage
  completion/reconciliation on the exact stopped clone, without replaying clone.


## Stopped-clone CPU/memory transport — 2026-10-18

- Scope: fixed digest-bound CPU/memory update and applied-value observation on the
  exact stopped clone. No production resource executor/stage transition was added.
- Plan builds validated inputs with generated identity and retained revision.
  Transport rechecks clone marker/name/digest/template/lock/power/QMP and pending
  changes, then sends one synchronous PUT with fixed topology, vCPU count, no CPU
  limit, requested MiB and disabled ballooning. Proxmox digest comparison detects
  concurrent configuration changes; external power transitions are not locked.
- Observer requires exact applied values and matching digests across settings and
  independent clone observation, with no pending changes. Reconciliation is GET
  only and allows a new applied revision. CPU/memory evidence implies no readiness.
- Official upstream QEMU synchronous API reviewed; ADR 0026 records semantics and
  limitations. Four targeted HTTPS tests pass after adding explicit CPU-limit
  handling: exact form, pre-write target gates, observation values/revisions, and
  no replay on malformed submission replies. Scoped library Clippy passes. Prior
  clone/storage/client/native suites were not rerun. No new dependency or real VM
  mutation; production provisioning remains unavailable.
- Next single slice: durable resource-stage claim/dispatch/completion and GET-only
  uncertain reconciliation using clone completion evidence; never replay clone.


## Durable CPU/memory execution and recovery — 2026-10-18

- Scope: advance only from validated clone evidence into one resource update,
  persist its revision/dispatch boundary, reconcile uncertainty by GET and retain
  stage completion separately from incomplete whole-plan status.
- Migration 7 adds a private stage config_digest with resource-kind/dispatch/hex
  constraints. Claims validate clone evidence and bindings and roll back corrupt
  state. Retained resource revision must match the completed clone revision.
- Resource stages interrupted before dispatch may resume after fresh checks;
  dispatched stages always reconcile by GET, without a resource UPID, even with
  new creation disabled or no current catalog. Five-second backoff limits repeats.
  Earlier clone task stays retained as history and is never replayed.
- Full-plan approval expires for first clone dispatch; subsequent retained-plan
  resource work can outlast 120 seconds without renewing confirmation or changing
  inputs. First resource dispatch requires capability/current plan and exact
  stopped-clone revision/pending checks, then commits the marker before PUT.
- Fixed observations can prove the requested state after success, malformed reply
  or actual connection loss. Unproven outcomes remain uncertain, never repeat PUT.
  Completion binds identity/plan/CPU/memory/stopped target and saves only resource
  evidence. Workflow remains running, result absent; all following stages queued.
- Verification: 8 affected pre-existing intent cases passed; 5 new SQLite cases
  passed in focused runs, including restart, corruption/rollback and schema-6
  migration. After refining pre-dispatch resume, only that changed test was rerun.
  Three HTTPS executor tests pass, including real connection-close injection and
  one-PUT counting. Two affected old migration cases and future-schema refusal
  pass. Scoped library/intent-test Clippy, changed-file rustfmt and diff review
  pass. No broad server/client/native suite or real VM test was run.
- No new dependency, infrastructure/credential mutation or production caller.
  ADR 0027 records approval lifetime and synchronous no-task recovery semantics.
- Next single slice: fixed disk expansion transport and exact post-expansion
  observation, using retained resource completion and the mapped disk device.


## Fixed mapped-disk expansion transport — 2026-10-18

- Scope: fixed absolute-size expansion and configured-size observation for only
  the catalog-selected primary device and exact observed volume/revision.
- Plan builds bounded inputs on the mapped target storage. Same-size/non-expansion
  requests are rejected; no disk stage is planned when source/target are equal.
  Transport verifies current CPU/memory, stopped clone identity, pending changes,
  consistent digests and exact original source volume/size before one PUT.
- Official upstream resize API reviewed: returned resize task binds TARGET VMID,
  unlike qmclone. Wrong node/action/source/canonical-ID aliases fail validation.
  Form contains only mapped disk, absolute requested GiB and digest; no relative
  increment, skiplock or generic fields. Existing TLS/retry policy is unchanged.
- GET-only post-task observation requires exact configured size/volume/device and
  independently consistent current clone/resources. Integer byte/K/M/G/T syntax
  accepted; duplicates, fractional/overflow/missing sizes and CD-ROM media rejected.
  This proves no backend allocation, guest partition/filesystem growth or readiness.
- Four targeted HTTPS tests pass (first three together, added mapped-device case
  separately), covering fixed form/task identity, before-PUT failures, exact size,
  actual connection loss/malformed replies with one PUT, all mapped devices and
  file-backed volume. Scoped library Clippy, changed-file rustfmt and diff review
  pass. Earlier unrelated clone/storage/client/native checks were not repeated.
- No schema/dependency or real infrastructure/credential change. No production
  caller or completed disk journal step; ADR 0028 records this boundary.
- Next single slice: persist exact disk volume/revision and dispatch/task linkage,
  then execute/reconcile only expand_disk after validated resource completion.


## Schema 8 retained disk dispatch and journal outcomes — 2026-10-18

- Scope: durable exact volume/revision, disk-stage claim/dispatch/task/completion
  and recovery guards, after validated clone/resource evidence. No executor wired.
- Migration 8 adds a separate private disk-dispatch record; resource-only digest
  constraints remain intact. Reader validates volume inputs, resource revision and
  target-bound resize tasks. Invalid evidence/dispatch/task causes claim rollback.
- Undispatched queued/interrupted disk work can resume. Dispatched uncertainty can
  be claimed only with a known task after backoff; unknown submissions never replay.
  Begin atomically saves volume/revision and marker, clearing current task while
  retaining the earlier clone task on its stage. Linking is bound and immutable.
- Completion binds scope/instance/plan/task, exact device/volume/bytes, CPU/memory
  and stopped target; only disk becomes succeeded. Whole operation stays running
  without result, later stages queued. Definite auth refusal or known failed task
  may fail; ambiguous outcomes remain uncertain. Associations remain reserved.
- Verification: 13 affected existing intent/resource tests passed after migration
  and reader changes. Six new journal tests passed in targeted groups, covering
  binding, completion, no replay, restart, failure classification, corrupted state
  rollback and schema-7 upgrade/FK preservation. Two affected schema-2/schema-4
  compatibility cases pass. Scoped library/intent-test Clippy and changed-file
  rustfmt/diff checks pass. No unrelated HTTPS/frontend/native suite repeated.
- No dependency, actual VM, infrastructure or credential changes. ADR 0029 records
  storage guarantees separately from pending disk execution and real size proof.
- Next single slice: standalone disk executor with fresh exact-volume/resource
  checks, one PUT, durable resize task linkage/polling and GET-only reconciliation
  against generated HTTPS fixtures; keep production provisioning unavailable.


## Standalone disk execution and HTTPS recovery — 2026-10-18

- Scope: connect the disk journal to fresh provider checks, one PUT, durable resize
  task retention/polling and independent configured-size observation. No public
  enqueue route or production worker was enabled.
- New-dispatch capability, scope and current catalog gate volume selection. Only
  the mapped drive is selected; full clone/resource/source-size/revision/pending
  checks run before the marker and again in transport after its commit.
- Known tasks reconcile using retained inputs with GET only, even if new creation
  is disabled or current catalog absent. Unknown replies, including actual HTTPS
  connection loss, remain unclaimable submissions. A post-marker config change
  prevents PUT and still cannot authorize automatic replay.
- Success requires task success plus exact configured volume size and retained
  target/resource bindings. Disk alone succeeds; operation stays running without
  result and later stages remain queued. Task failure and observation uncertainty
  retain distinct safe failures and the local target association.
- Verification: 3 initial HTTPS executor tests passed; an added post-marker race
  test passed separately. Cases include one PUT, failed/unknown/foreign tasks,
  size/marker mismatch, disabled known-task versus unknown restart, and first-write
  capability/catalog/scope/volume/revision/resource/pending/power gates. Four
  affected disk transport tests pass after extracting the shared read-only guard.
  Scoped library Clippy, changed-file rustfmt and diff review pass. No unrelated
  storage/client/native or full suite was repeated.
- No schema/dependency, real infrastructure or credential change. No backend
  allocation, guest filesystem growth, readiness or full provisioning is implied.
- Next single slice: fixed network/Cloud-Init transport and independent applied
  settings observation, retaining strict SSH-key/identity checks. Keep documented
  scalar-user compatibility and real first-boot evidence separate from API setup.


## Fixed network and Cloud-Init settings transport — 2026-10-18

- Scope: bounded definition/plan-bound network/user/public-key settings and exact
  independent observation, without durable stage/worker/API activation.
- Supports one virtio net0; preserves its observed MAC/firewall and uses catalog
  bridge. Rejects extra NIC/IP configs and unmodeled VLAN/rate/MTU/etc. Requires
  exact retained primary volume/requested disk size and target-specific ide2
  Cloud-Init drive, current resources/stopped identity/revision/pending checks.
- Fixed synchronous PUT sets IPv4 DHCP, NoCloud, catalog user, disabled implicit
  upgrades and canonical public keys; deletes inherited cicustom/cipassword.
  Public-key comments are removed while key material and original saved definition
  remain intact. API URI-unescaping requires inner URI + outer form encoding.
- Observer requires exact applied settings and no inherited override/password,
  with independent consistent current resources/disk/drive. Reports key count and
  guest_ready=false; never returns key contents. Request Debug redacts keys.
- Verification: 3 initial targeted HTTPS tests pass; an added multi-key encoding
  test passes separately, comparing decoded key material through both layers.
  Tests cover fixed form, definition binding, source gates, applied settings and
  actual dropped/malformed reply with one PUT. Scoped library Clippy and changed
  rustfmt/diff review pass. No unrelated disk/storage/client/native suite repeated.
- No schema/dependency or real VM/infrastructure/credential change. ADR 0030
  records supported template shape and the still-open scalar-user compatibility
  requirement from ADR 0017. Built-in API setup does not implement the lab's
  corrected target-specific snippet or prove first-boot readiness.
- Next single slice: durable configure_cloud_init stage after validated resource
  and optional disk evidence, retaining observed network/volume/revision for
  one-shot dispatch and GET-only uncertainty recovery. Keep seed compatibility
  and strict guest first-boot readiness as subsequent uncompleted requirements.


## Schema 9 durable Cloud-Init API-settings journal — 2026-10-18

- Scope: validated prior-step evidence, exact observed network/volume/revision
  retention, one dispatch marker, completion/failure/uncertainty and recovery.
  No provider executor or production worker/API is wired in this slice.
- Migration 9 adds private Cloud-Init dispatch inputs. Prior resource completion
  supplies the source revision when no expansion is planned; validated disk task/
  completion supplies revision and exact volume otherwise. Claims require every
  preceding planned stage succeeded and reject corrupted evidence transactionally.
- Begin validates the original full-plan approval and fixed request, then commits
  inputs and marker together. Clears current task while retaining old stage tasks.
  Dispatched work cannot begin again; uncertainty allows future GET-only recovery
  without UPID. Undispatched interrupted work may resume; backoff is preserved.
- Completion binds identity/plan, primary disk, network, user/key count and current
  stopped resources/revision, requiring guest_ready=false. Only API-settings stage
  succeeds; full operation stays running with no result and later stages queued.
  Post-marker errors cannot become definite failures. Reservations remain held.
- Verification: 19 affected prior journal tests pass with migration/reader changes.
  Five new focused SQLite cases pass for optional expansion, binding/restart,
  corruption rollback, schema-8 upgrade and failure exclusion. Clippy found the
  larger success enum variant; boxed its result and reran all five changed cases
  plus scoped library/intent-test Clippy successfully. Two affected older migration
  cases and changed-file rustfmt/diff checks pass. No broad/client/native rerun.
- No new dependency or real VM/infrastructure/credential mutation. ADR 0031 records
  storage guarantees separately from future execution, generated seed compatibility
  and strict guest readiness; the known scalar-user issue remains unresolved.
- Next single slice: standalone Cloud-Init executor linking current catalog/target/
  network/disk checks, one synchronous PUT and GET-only restart reconciliation to
  generated HTTPS fixtures. Keep production provisioning unavailable.


## Standalone Cloud-Init settings execution — 2026-10-18

- User asked to resume; scope connects validated journal inputs to current
  provider/catalog/target checks, one synchronous PUT and independent observation.
  No production worker/API creation caller was activated.
- Executor selects only mapped volume and net0, verifies optional disk identity,
  then checks current source revision, resources, disk size, Cloud-Init drive,
  stopped target and pending changes. Commits original observed inputs/marker
  before transport repeats guards and attempts the fixed configuration update.
- Missing/malformed response still permits exact independent settings observation;
  unproven results stay uncertain. Restart reconstructs only retained inputs and
  uses GET with new creation disabled and current catalog absent. Prior stages
  never replay. Settings success leaves full workflow running without result;
  start/later steps remain queued and guest_ready stays false.
- Verification: 3 targeted HTTPS executor tests passed. The new pre-marker gate
  fixture initially used an unsuitable sequential response order; changed it to
  path-based reads and reran that case successfully. Tests cover one PUT, actual
  dropped/malformed replies, applied/uncertain outcomes, disabled GET-only restart
  and new-write capability/catalog/scope/volume/revision/network/drive gates.
  Three affected transport tests pass after sharing their read-only guard. Scoped
  library Clippy and changed-file rustfmt/diff checks pass. No unrelated suite run.
- No schema/dependency, real infrastructure or credential mutation. Compatibility
  of generated seed and strict guest first boot remain unfinished; this slice does
  not reproduce the lab's corrected custom snippet or weaken readiness.
- Next single slice: inspect generated Cloud-Init seed via fixed read-only API,
  validate its user/key/network semantics and implement a pre-start compatibility
  gate. Evaluate the image default-user path before adding snippet transport; any
  change must retain the declared guest-user/key contract and strict readiness.


## Read-only generated seed semantic preflight — 2026-10-18

- User authorized continuation. Scope: fixed generated user/network previews,
  validated semantics and explicit rejection of known incompatible scalar users;
  no seed write, start implementation or first-boot/readiness claim.
- Reviewed official QEMU dump and Cloudinit generator: dumps are built-in generated
  previews, not custom snippet or attached-ISO reads. Existing applied-settings
  observer excludes overrides. Fixed user/network GETs are enclosed by full target
  settings observations with matching digests under one deadline.
- Strict typed YAML bounds payloads and rejects unknown/duplicate fields, commands,
  passwords, upgrades, wrong hostname/user/key material and unexpected network.
  Only exact network-v1 eth0/MAC/DHCP4 with bounded DNS is supported. Named-user
  shape is a compatible candidate; exact legacy scalar returns incompatible.
- Safe result exposes only revision/payload hashes/classification/compatibility
  and guest_ready=false. Does not expose key bodies, DNS addresses or seed text.
  Known upstream ciuser generation remains scalar and does not pass this check.
- Verification: two targeted HTTPS tests pass with multi-case safe/rejection
  coverage. Initial compile used borrowed query values rather than the client's
  owned query contract; fixed conversion before tests. Updated fixtures to actual
  upstream False spelling and reran only these tests. Scoped library Clippy,
  changed-file rustfmt and diff checks pass. No unrelated suite was repeated.
- No schema/dependency, actual provider write or infrastructure/credential change.
  ADR 0032 separates preview semantics from full schema, seed regeneration/attached
  contents and strict first-boot proof. Start integration remains pending.
- Next single slice: offline proof of an image-default-user generation strategy
  without the deprecated scalar, preserving catalog user and exact public keys.
  Require explicit template/default-user evidence before changing product behavior;
  evaluate corrected snippet lifecycle only if the default-user strategy is invalid.


## Offline image-default-user compatibility proof — 2026-10-18

- User asked to resume. Scope: determine whether omitting legacy user preserves
  declared user/keys when supplied image-default evidence matches; no product or
  provider behavior changes in this slice.
- Verified primary Proxmox generator and pinned Cloud-Init normalizer/reference
  sources. With ciuser absent, built-in generated user-data can use users=[default]
  without the deprecated scalar. This needs the exact image default to match the
  catalog account; distribution name alone is not sufficient evidence.
- Added isolated default-user candidate and check. Renders upstream Debian
  reference defaults (not an actual image), proves scalar/named/omitted-user
  normalization equality and candidate schema validity. Legacy emits one expected
  deprecation; omitted-user emits none. Exact SSH keys/declared user are selected
  by the actual upstream SSH handler with side-effect entry points mocked.
- Modified default resolves to another account and is rejected. Missing/unlocked/
  inherited-key evidence and unexpected password/commands/upgrade seed inputs
  also fail. No host SSH files are scanned or credentials applied by the check.
- Verification: only new check_default_user.py ran against the existing isolated
  upstream 25.1.4 venv/source and passed. No Rust/client/native, old converter or
  unrelated suite rerun. Diff review passes. ADR 0033 records proof limitations.
- No product dependency/catalog behavior, real VM/infrastructure/credential or
  installed system package changed. Strict first-boot and actual image-default
  evidence remain unverified for this alternative.
- Next single slice: explicit optional server-catalog image-default attestation
  bound to guest user/template version, preserving existing mapping hashes and
  behavior. Enable default-user transport/preflight only for opted-in mappings;
  retain strict first-boot/readiness and mark operator evidence as a declaration.


## First mobile journey: control-plane authentication — 2026-10-18

- User explicitly prioritized the smallest create-VM/SSH mobile vertical journey
  before extending features. Scope here: optional whole-API single-operator bearer
  auth, with main-process configuration and safe startup/failure behavior.
- VMANYWARE_ACCESS_TOKEN enables protection for reads, writes, health and fallback
  paths. Missing/invalid/duplicate headers return fixed 401; successful auth reaches
  real routes. Invalid configured tokens fail before database/listener startup.
  No token leaves Debug/errors/logs; request tokens are never accepted in URLs.
- Store only a SHA-256 verifier in middleware; use subtle constant-time comparison.
  subtle 2.6.1 already existed transitively; direct dependency/lock edge added,
  with no new resolved crate/version. Existing default local development remains
  usable without a token; service still binds loopback. TLS/remote profiles pending.
- Verification: 3 focused router/config cases passed, then actual binary fixture
  proved unauthenticated 401, authenticated reads, rejected unauthenticated write,
  safe logs and fail-closed invalid startup with no database created. Child env
  cleared; owned temporary database/token and loopback only, no provider access.
  Scoped lib/bin/auth-test Clippy and changed-file fmt/diff review pass. No unrelated
  server/client/native suite repeated. No real credentials/infrastructure changed.
- Tool discovery: adb/sdkmanager/xcodebuild unavailable on PATH; only host Linux
  Rust target is installed. This is not proof that no SDK exists elsewhere, and no
  mobile compilation/runtime is claimed. Phone-platform preference remains pending.
- Next single slice: native client secure remote endpoint/token connection profile
  and a visible connection check, preserving local mode and keeping secrets out of
  saved frontend state. Then connect the create workflow and mobile SSH backend.


## First mobile journey: remote native connection form — 2026-10-18

- Scope: visible remote HTTPS/token form and native in-memory connection check;
  existing VM controls remain local and are hidden in remote mode to prevent
  cross-server actions. This is not the complete create/SSH mobile journey.
- Native state retains a verified endpoint/client/sensitive bearer header only
  after fixed health succeeds. New attempt/disconnect invalidates old generations;
  stale replies cannot replace a newer connection. URL credentials/path/query/
  fragment/plain HTTP are rejected; redirects/retries and proxy discovery disabled.
- Require server's accepted-auth response marker, added only by the access-token
  middleware. An unauthenticated 200 health response cannot establish a secure
  remote connection. Tokens are cleared from the input on submission and never
  stored in frontend storage. Profile status and Debug omit token material.
- Enabled client's existing reqwest rustls feature. TLS dev fixtures add rcgen,
  tokio and tokio-rustls test usage; all already resolved in the workspace lock,
  with dependency-edge changes only and no new resolved crate/version.
- Verification: 3 filtered native tests pass using the owned Linux WebKit prefix:
  real generated HTTPS/correct bearer, untrusted TLS, redirect/rejection/malformed
  and missing auth marker, invalid URLs, disconnect and stale native generations.
  Affected server auth-marker route case passes. Frontend production build passes.
  Focused real Firefox/React fixture with simulated IPC passes form/token-clearing/
  no-storage, local-control isolation, disconnect and late-result suppression.
  Native/server library Clippy and changed-file fmt/diff checks pass. No broader
  native/server/controller/GUI suite or actual mobile build was rerun.
- No real connection/deployment/VM/credential changes. Native app launch for this
  new command, platform credential storage and actual phone runtime remain open.
- Next single slice: route fixed remote reads through the retained connection ID
  and show remote templates/environments, then attach the minimal create workflow.
  Do not expose local VM mutations through the remote mode or claim it is ready.


## First mobile journey: remote workspace reads — 2026-10-18

- Scope/acceptance: read only the connected server's fixed catalog/environment
  routes; show lists, empty states and independent errors; discard stale reads
  after disconnect. No new VM mutation or infrastructure access.
- Native commands require the retained connection generation, use verified HTTPS
  and bearer authentication, require the accepted-auth marker, bound response
  bodies and validate the supported response shapes. Only fixed GET paths exist.
  Connection locks are released before network waits; stale successes/errors are
  rejected after reads. Catalog sourceDiskGiB spelling matches the server contract.
- Remote UI explicitly loads configured templates and registered environments.
  Registration is labelled as live state not checked; a catalog is not proof of
  template availability. One failed list does not hide the other successful list.
- Verification: 3 new generated-HTTPS native tests pass (fixed authenticated paths,
  malformed/unauthenticated responses and in-flight disconnect). The 3 affected
  native connection tests also pass after GET refactoring. Focused Firefox/React
  simulated-IPC test passes empty/populated lists, partial failure, token clearing,
  local-control isolation and stale reads/connect responses. Frontend production
  build, native library Clippy and changed-file formatting/diff checks pass.
  No unrelated server/controller/full-workspace suite was repeated.
- These are native HTTP tests plus browser UI tests, not a real mobile/native-IPC
  launch or remote deployment. Creating/controlling VMs and SSH in remote mode
  remain unavailable. No real VM, credentials or infrastructure were changed.
- Next single slice: unblock strict first boot with explicit catalog default-user
  attestation, then wire the minimal durable create/start/readiness workflow into
  the server API/worker and remote UI. Defer additional management features.


## First mobile journey: attested image-default user — 2026-10-18

- Scope/acceptance: unblock compatible Cloud-Init user generation with explicit
  template-bound opt-in; preserve legacy mappings and test both transport modes.
- Optional imageDefaultUser declares matching guestUser/templateVersion, locked
  password and no inherited keys; root, mismatches, false/missing/unknown fields
  are rejected. Absent fields are omitted from serialization, preserving legacy
  mapping hash inputs. Present declarations are bound into the plan hash/snapshot.
- Opted-in transport deletes ciuser; exact observation requires absence. Seed
  inspection accepts omitted user only with attestation, rejects explicit null,
  named/scalar entries in that mode and retains exact keys/network/revision checks.
  Operator declarations and generated previews do not prove installed defaults,
  attached ISO contents or first boot. ADR 0037 records the choice.
- Verification: 8 existing catalog/API tests passed after the model extension;
  new targeted catalog/attestation case and generated-HTTPS transport/seed case
  pass. The 2 affected seed-preflight tests and 8 Cloud-Init transport/executor
  tests pass (the new HTTPS case is included in the latter filter). Scoped server
  library Clippy and formatting/diff checks pass. No unrelated
  client/full-workspace checks or real infrastructure operations were performed.
- Next single slice: durable seed regeneration/start stage and strict readiness,
  then production create API/worker and visible remote create controls. Existing
  clone/resources/disk/Cloud-Init executors are still standalone; do not claim
  the create/SSH journey works yet.


## First mobile journey: fixed seed regeneration transport — 2026-10-18

- Scope: implement the provider regeneration call and pre/post seed checks; durable
  journal and start gating remain the next slice. Upstream API source confirms a
  synchronous null response, no UPID and no digest parameter (ADR 0038).
- New fixed dispatch_seed_regeneration repeats compatible preview/revision/hash
  checks before one empty authenticated PUT to the exact target cloudinit route.
  Null success is followed by independent settings/seed checks; changed payloads
  fail. Only safe hashes/revision/format are returned, always guestReady=false.
- Verification: generated-HTTPS test passes seven variants: success, incompatible
  report, stale revision/hash, lost reply, unexpected task-like reply and changed
  postflight contents. Exact route/auth/empty body and zero-or-one writes asserted.
  Scoped server library Clippy and changed-file formatting/diff checks pass. No
  unrelated suite repeated, new dependency/schema or real infrastructure access.
- This is transport preparation, not durable regeneration or a usable create
  workflow. Callers must persist a dedicated dispatch guard and successful result;
  previews alone cannot recover unknown regeneration. Endpoint lacks atomic digest
  protection, so bounded pre/post observations cannot exclude concurrent edits.
- Next single slice: durable seed-regeneration journal and executor, retaining
  success across restart and forbidding replay after uncertain submission. Require
  this evidence for start, then implement strict readiness and wire create API/UI.


## First mobile journey: durable seed regeneration — 2026-10-18

- Scope/acceptance: persist regeneration intent/result, preserve success across
  restart and never replay an uncertain submission. Implemented as start_vm's
  first substep without changing saved plan steps/hashes. Migration 10 adds a
  bounded private report journal because stage results are terminal-only.
- Validated prior Cloud-Init settings now expose typed evidence tied to instance,
  plan/scope, exact resources/disk/network/user/keys, stopped state and revision.
  Seed claim requires this chain. Current capability/catalog/provider scope are
  checked before preflight. Expected compatible hashes/revision/format and stage
  dispatch marker commit before one fixed regeneration PUT; completed proof is
  retained separately. No raw keys/seed, fabricated UPID or new dependency.
- A successful regeneration leaves start_vm/workflow running, with no power task.
  regenerated_seed() is the necessary gate for the future start executor; it is
  not guest readiness. Before-marker interruption can resume. After-marker
  interruption/lost reply remains uncertain and is never claimed for another PUT.
  Successful journal proof survives recovery and does not trigger another request.
- Verification: 2 focused storage tests pass for approval/proof rejection, both
  disk shapes and restart before/after markers/success. Generated-HTTPS executor
  test passes 6 cases (success, lost reply, disabled, missing catalog/provider,
  scalar incompatibility), with exact write counts and no restart requests.
  Five provisioning schema checks, including 9->10, plus affected lifecycle/
  instance upgrade checks pass. Scoped server library Clippy and formatting/diff
  checks pass. No real VM/infrastructure/credentials changed or mobile launch.
- Next single slice: durable power-start substep that requires regenerated seed
  evidence, reconciles retained start tasks without resubmission, then strict
  readiness and production create API/worker/remote controls. ADR 0039 explains
  journal semantics; do not treat the start_vm marker as a power submission.


## First mobile journey: durable provisioning power start — 2026-10-18

- Scope/acceptance: start only after retained compatible regenerated seed; persist
  separate power intent/task before polling; never replay unknown submissions and
  reconcile known tasks through GET after restart. Implemented in the existing
  schema-10 journal with an optional backward-compatible power substep, no new
  migration/dependency or change to saved plan steps/hashes.
- A transactional claim distinguishes seed-only completion, claimed power work and
  submitted power. Current capability/catalog/scope and exact stopped-target seed
  revision/hashes/format are checked before a separate durable marker and fixed
  start POST. Task binding uses target node/VMID and qmstart. Corrupt seed/power/
  task evidence fails closed; claiming rolls back without changing the journal.
- Known-task recovery works with writes disabled/catalog absent. Unknown tasks
  never reclaim. Successful task plus independent running/QMP observation succeeds
  start_vm only, with guestReady=false; readiness remains queued and operation
  running. The standalone executor is not wired into the production worker/API.
- Verification: 2 filtered storage tests pass for approval/proof/task binding,
  both disk shapes and restart before/after power markers. Separate corruption
  test passes. Generated HTTPS runs actual seed then start executors across 7
  cases: success, lost start response, task recovery, changed seed, disabled writes,
  failed task and paused VM. Recovery asserts exactly two GETs and no new POST.
  Three affected seed-journal regression cases and server library Clippy pass;
  changed-file formatting/diff checks pass. No unrelated client/full suite, real
  VM/infrastructure mutation, credential access/change or mobile runtime claim.
- Next single slice: strict bounded guest-readiness execution and durable result,
  validating the retained start proof before any ready claim. Then wire the
  minimum create API/worker and visible remote creation controls. ADR 0040 records
  the start journal; retain honest separation of task, power and guest evidence.


## First mobile journey: fixed guest-readiness transport — 2026-10-18

- Scope/acceptance: bounded strict Cloud-Init status plus MAC-bound IPv4 observation;
  source preparation is now compiled/tested, not durable readiness or real guest
  validation. ADR 0041 records API/source findings and limitations.
- New plan-bound request retains clone/instance marker, expected revision and MAC.
  Fixed read-only cloud-init status --format json command uses QGA command-array
  POST without shell/stdin/wait/arbitrary arguments. Positive PID is observed with
  fixed GET exec-status and network-get-interfaces. Each method is limited to 20s.
- Guard exact marker/revision, running/QMP, no template/lock/pending changes before
  observations and after a ready candidate. Require exit 0, status/extended_status
  done, no global/per-stage fatal or recoverable errors and completed timestamps
  in all four Cloud-Init stages. Running/not-started/unfinished is pending; no
  stderr, signal, truncation, missing stage or exit-2 workaround is accepted.
- Only one usable IPv4 on the expected MAC can count. Wrong NIC, loopback/link-local/
  subnet/broadcast/unspecified address does not establish readiness; ambiguous IPs
  or matching interfaces fail closed. Results/errors omit raw guest stdout/stderr.
  IPv4 evidence does not prove SSH reachability, authentication or host identity.
- Verification: 3 scoped generated-HTTPS tests pass, including boolean/integer
  provider flags, fixed command/PID/routes, pending/error/truncation/address/
  revision/QMP cases, foreign markers, invalid PID and lost reply without retries.
  Scoped server library Clippy and changed-file formatting/diff checks pass. Pinned
  local Cloud-Init source and official provider source were inspected for actual
  formats. No unrelated checks, runtime dependency/schema or real VM changes.
- Next single slice: durable readiness PID/deadline/result and executor, validating
  retained successful power-start proof before probing or reporting ready. Then
  connect the minimum create API/worker and visible remote controls. Production
  readiness, overall deadline/restart behavior, actual guest and mobile remain open.


## First mobile journey: durable guest readiness — 2026-10-18

- Scope/acceptance: persist probe PID and fixed overall deadline, resume bounded
  observations after restart, reject stale/invalid evidence and never skip later
  declared stages. Implemented in the schema-10 journal's optional readiness
  field, preserving old reports/plan hashes without a migration/dependency.
- Claim validates the entire retained power-start/seed/settings chain. Read-only
  agent availability is checked before marking a new fixed command; PID persists
  before observation. Known PIDs recover with GET only. Lost submissions never
  re-execute. Completed pending probes clear their PID before another command,
  respecting QEMU's consuming exec-status semantics. No arbitrary command exists.
- Retain one 300-second window, five-second scheduling and maximum 60 commands;
  restarts do not renew it. Backward clock/expired deadline prevents ready claims,
  including late completed responses. Safe failure codes distinguish rejected,
  unconfirmed and timed-out readiness, retaining the VM for inspection.
- Clean scoped Cloud-Init/IP evidence completes readiness; overall success occurs
  only when it is the last declared stage. Packages/snapshots remain queued when
  declared. No SSH authentication/reachability or real first-boot claim follows.
- Verification: 4 focused storage tests pass (proof/PID guards, retained deadline/
  restart, completed-pending refresh, expiry and minimal-workflow completion).
  Generated HTTPS executor tests prove known-PID GET-only recovery, unknown-command
  non-replay, no provider calls after timeout and real fixture exit-2 rejection.
  Three affected guest transport tests and scoped server library Clippy pass;
  changed-file fmt/diff checks pass. Official QEMU semantics checked. No unrelated
  client/full suite, actual guest command, VM/infrastructure/credential changes.
- Next single slice: expose the minimum fully confirmed create enqueue and connect
  these standalone stages to the sequential production worker. Reject unsupported
  package/snapshot requests for that initial route explicitly, then add visible
  remote create controls. Do not start another isolated preparatory helper first.
  Mobile build/SSH backend and approved real disposable end-to-end test remain open.


## First mobile journey: confirmed create API and production worker — 2026-10-18

- Scope/acceptance: expose fully confirmed creation under explicit opt-in, connect
  durable stages to the actual worker and reject unsupported features explicitly.
  Implemented POST /api/v1/provisioning/create with one request key and complete
  Approval. Enqueue commits immutable definition/catalog/plan/association/stages
  before 202/Location and performs no provider call. Same-key retries retain one
  operation; target/scope/hash/feature/config errors reserve nothing.
- VMANYWARE_PROVISIONING_ENABLED defaults false, accepts exact true/false and fails
  startup safely before database creation when invalid. Main passes the same flag/
  catalog to router and sequential production worker. Lifecycle opt-in is separate;
  listener remains loopback and whole-API bearer middleware still covers creation.
- Minimum route requires empty packages, snapshotAfterProvisioning=false and an
  attested image-default mapping. Unsupported features return 422. New mutations
  recheck current capability; known tasks reconcile without replay. Every round
  runs one read, one lifecycle claim and at most one provisioning stage, prioritizing
  existing later-stage work. Unknown submissions never repeat or auto-clean up.
- Preview now exposes configured providerScopeId and runtime creation capability;
  no reservation/live verification claim. Absent scope preserves saved-plan JSON.
  Existing preview type accepts the boolean capability; remote creation UI is not
  yet present. Cloud-Init form serialization now leaves scope before await so the
  full production worker future is Send; explicit-user form behavior is retained.
- Verification: 5 targeted API/config/disabled-worker cases pass, including many
  rejection variants and an actual env-cleared binary invalid-startup fixture.
  Full generated-HTTPS Axum enqueue -> production worker -> clean guest evidence
  passes with 8 GiB/no resize and 40 GiB/resize, restart after clone, inventory
  progress and no repeated mutation on enqueue replay. Nine affected catalog/API
  checks and the explicit-user form check pass. Server lib/bin/new-test Clippy,
  changed-file formatting/diff and client TypeScript checks pass. No unrelated
  client/native/full-workspace suite repeated.
- No real VM/provider/guest commands, credentials or infrastructure changed. This
  proves local production-code integration, not actual Proxmox/phone/SSH behavior.
  ADR 0043 and the updated create API contract capture current limits.
- Next single slice: fixed remote native definition/plan/create/operation commands
  and a visible full-target confirmation flow, tied to current connection ID and
  retained idempotency key. Then phone build/SSH backend and the explicitly
  approved disposable end-to-end gate. Do not return to isolated preparatory helpers.


## First mobile journey: remote create form and native commands — 2026-10-18

- Scope/acceptance: fixed remote prepare/save/create/status commands and visible
  full-target confirmation, current connection generation and retained request key.
  Implemented form fields for definition/image/VMID/resources/public keys, with no
  packages or snapshots; native preparation saves or verifies matching definition
  hash before presenting a supported/capable server plan.
- Native session retains the plan/key and constructs Approval itself. Confirmation
  requires the matching key, generation and explicit true; concurrent submission is
  serialized. Lost responses keep the same immutable key/body for manual retry.
  New intent preparation is blocked after submission on that connection. Reads use
  the retained operation ID and reject another ID/kind or mismatched ready proof.
- Verified HTTPS/auth marker, bounded JSON and fixed paths remain mandatory; errors
  omit raw bodies/tokens. Disconnect clears session context and late replies, not
  the server operation. The UI displays complete server/target/template/storage/
  bridge/resources/user/key-count review and separate confirmation; manual status
  refresh distinguishes initialized guest from actual SSH access/trusted identity.
- Verification: 6 targeted native cases pass for plan/save/conflicts, confirmation/
  exact approval POST, safe failures/key retention, scoped ready validation, exact
  operation GET and disconnect during submission. One affected connection-generation
  case passes. Focused real Firefox/React simulated-IPC check passes confirmation,
  lost-response same-key retry, queued/ready display and stale read suppression,
  alongside existing remote list/connection guards. Frontend production build,
  scoped native library Clippy and changed-file fmt/diff checks pass.
- No server/full-workspace/controller suites repeated. No actual new-command Tauri
  IPC launch, deployed service, VM, credential/infrastructure changes, SSH or phone
  runtime is claimed. ADR 0044 records this boundary.
- Remaining limitation: one submitted intent per connection; context is memory only
  and GUI recovery after full restart/reconnect is not yet implemented. Users can
  retain key/operation ID for server-side inspection. Next single slice: explicit
  current-server operation recovery after reconnect/restart, then mobile build/SSH
  backend and an approved real disposable end-to-end check. Full roadmap remains
  active, including snapshots/recreate/delete and platform/release gates.


## First mobile journey: read-only remote creation recovery — 2026-10-18

- Scope/acceptance: reconnect to an explicitly authenticated server and find/read
  a retained creation and its target without submitting creation again. Added
  fixed native history/operation/environment GET commands and visible recent-list,
  saved-ID and manual-refresh controls. No credentials or intent persist locally.
- History examines the server's latest 100 operations and filters creations;
  older operations remain accessible by saved ID. Recovery works without the
  original prepared plan/session. IDs, kind, association, provider scope and clean
  successful guest evidence are validated before returning sanitized fields.
- Disconnect invalidates pending results. Recovery never resubmits/retries/cancels
  an operation or restores a native approval/key. Target associations and ready
  results are historical evidence, not current power, ownership or SSH proof.
- Verification: 3 focused native recovery tests pass, including malformed/wrong
  responses and disconnect during a read. Two affected prepare/ready regression
  tests pass. Real Firefox/React simulated IPC passes remount recovery, empty/list
  states, retained-target display, unchanged creation-submit count and stale-read
  suppression. Frontend production build, native library Clippy and changed-file
  formatting/diff checks pass. No server/unrelated/full-workspace suites repeated.
- No actual new-command Tauri IPC launch, phone build, real guest/VM/infrastructure
  or credential changes. Original prepared create/key remains session-only; an
  uncertain submission after restart can be inspected, not automatically retried.
  ADR 0045 records the recovery contract and supersedes ADR 0044's GUI limitation.
- Next single slice: choose and implement the smallest mobile SSH authentication
  path with strict host trust, then prove its native build/runtime. Existing agent
  authentication remains macOS-only; platform preference is still unanswered.
  Do not defer this behind snapshots or more recovery helpers. Real disposable
  end-to-end execution still needs explicit target/action approval.


## First mobile journey: encrypted application SSH identity — 2026-10-18

- Scope/acceptance: explicitly create a reusable Ed25519 key on the device, keep
  only encrypted private material on disk, reopen with the correct passphrase,
  refuse replacement and insert only its public key into remote-create YAML.
  Implemented native fixed-path create/unlock commands and visible form controls.
- Uses resolved ssh-key crypto, AES-256-GCM/OpenSSH and fixed bounded bcrypt KDF;
  private owned directory/file, no-follow reads, bounded input and atomic no-clobber
  persistence. Public-only return values; decrypted key lifetime is operation-local.
  No private-key import/discovery/export or passphrase/browser storage. Existing
  user-entered public keys are retained; insertion is explicit and deduplicated.
- Verification: 2 focused native tests pass for encrypted persistence/reopen,
  same fingerprint, no replacement, wrong phrase, busy/permission/symlink/size/
  corruption/plaintext rejection and valid-envelope GCM tampering. Real Firefox/
  React simulated IPC passes create, confirmation, cleared fields, failed/successful
  unlock and public-only definition submission, plus affected remote create flow.
  Frontend production build and native library Clippy pass; changed-file fmt/diff
  checks pass. No server/unrelated suites run.
- Source enabled for Unix, compiled/tested only on Linux. Not OS-keystore-backed;
  no biometric unlock, migration/backup or mobile storage/lifecycle proof. Only
  x86_64 Linux Rust target installed; adb/sdkmanager/xcodebuild unavailable in PATH.
  No actual new-command native IPC, phone launch or SSH authentication is claimed.
  All generated keys in checks were disposable; no user credentials/VMs changed.
- Next single slice: connect this persisted identity to explicit authenticated SSH
  and the existing bounded PTY/terminal with fresh strict host trust. Validate with
  generated localhost SSH fixtures, then mobile native build/runtime. ADR 0046
  records the storage decision and remaining platform gates. Full roadmap active.


## First mobile journey: application-key SSH authentication and terminal — 2026-10-18

- Scope/acceptance: use the retained encrypted app key through fresh strict host
  trust and the existing selected-identity/PTY flow, proving real local output,
  resize, Ctrl-C, close and reconnect. Implemented on Unix with explicit UI source,
  account/passphrase/consent, public fingerprint selection and check/open controls.
- Native selection verifies persisted host trust before decrypting; authentication
  verifies again before signing. Selection is window/account/endpoint-bound,
  single-use and expiring. Cancellation/replacement/destruction invalidate it;
  idle app tickets have a cleanup timer. No fallback key/agent/password is used.
  Store clones share the same nonblocking KDF mutex; private material stays native.
- Verification: 3 focused native SSH tests pass, including unknown/changed host,
  wrong phrase/fingerprint/window, cancellation/replay/expiry, rejected username,
  channel-free authentication, actual /bin/sh output, stty resize, foreground Ctrl-C
  with status 130 in under 2 seconds, close and reconnect with zero owned fixture
  connections left. Five affected host and five trust tests plus two PTY-policy
  tests pass. These tests use generated keys and owned localhost SSH fixtures only.
- Real Firefox/React simulated IPC passes application-key consent/unlock/failure,
  cleared secret fields, changed-host refusal, explicit identity, check/terminal,
  Ctrl-C/disconnect, no agent access/storage and stale unlock/start cleanup.
  Frontend production build, native library/test Clippy and changed-file fmt/diff
  pass. No server/full-workspace/unrelated suites repeated.
- Source now compiles the shared host/authentication/PTY path on Unix; Linux has
  protocol/real-shell evidence, not a new-command native GUI/IPC launch. The macOS
  agent fixture was adapted for explicit source choice but not run on this Linux
  host. No phone, real guest, OS-keystore/biometric or mobile lifecycle proof.
  No user credentials, VM or production infrastructure changed. ADR 0047 applies.
- Next single slice: Android native build/launch feasibility for the existing
  create-to-SSH screens using an isolated toolchain on this Linux host. Platform
  preference remains unanswered; Android first is a provisional host-driven choice,
  not cancellation of iOS. Then actual mobile input/lifecycle and an explicitly
  approved disposable real create-to-SSH gate. Full roadmap remains active.


## First mobile journey: Android packages and native emulator smoke — 2026-10-18

- Scope/acceptance: build the complete app for Android and verify actual packaged
  launch/bridge where this host permits. Generated editable Android project source
  is now retained, while generated glue/JNI, local paths, signing and outputs stay
  ignored. No product runtime crate or frontend/server behavior changed.
- Both aarch64/ARM64 and x86_64 Rust libraries compile and separate debug APKs build.
  JDK 21.0.2, SDK/build-tools 36, NDK 28.2.13676358, CLI 2.12.3, Gradle 9.6.1,
  AGP 9.3.1 and Kotlin 2.2.10 were used in an owned temporary prefix. Gradle
  distribution checksum is pinned. Configured minimum API 24 is not tested parity.
- Native Activity reserves system-bar/cutout space, sets a matching dark backdrop
  and forwards handled inset types as zero while keeping IME notifications. Initial
  padding/forwarding gave a mounted but unpainted native surface; revised settings
  pass cold visual capture. No keyboard/IME or physical-device claim follows.
- Explicit no-backup plus both cloud/device extraction exclusions are packaged.
  APK public signatures verify; native ABI and embedded frontend/terminal asset
  names are inspected. Actual Android backup/transfer and release signing untested.
- Verification: real Android 15/API 35 x86_64 (Pixel 6 AVD, WebView 124.0.6367.219)
  packaged app passes actual UI native-platform button, native SSH support/guard
  dispatch, encrypted identity creation, wrong phrase/no replacement, identical
  identity after force-stop/relaunch and 412px layout without horizontal overflow.
  The opt-in check inspects the actual framebuffer, not DOM readiness alone, and
  records a reviewed launch screenshot. No fake IPC or app fixture mode is used.
- Evidence: experiments/android-native/check.mjs and README; ignored
  .lab/android-native/report.json, launch.png and artifacts.json hold runtime and
  artifact hashes. Logs are /tmp/vmanyware-android-build-final.log and
  /tmp/vmanyware-android-native-smoke.log. ARM64 APK is at
  apps/client/src-tauri/gen/android/app/build/outputs/apk/arm64/debug/app-arm64-debug.apk;
  x86_64 is the analogous x86_64 path. Both are development/debug artifacts, not
  release packages. Current verified frontend assets were reused; no unrelated
  server/Linux/full-workspace suites repeated. ADR 0048 records decisions/limits.
- Reuse hint: toolchain is /tmp/vmanyware-android-reovczwi with env.sh; confirm it
  still exists before use. AVD is vmanyware-native-smoke in ANDROID_USER_HOME/avd,
  dedicated ADB port 5047, emulator serial emulator-5580, gRPC port 8558 with JWT.
  The emulator/app/ADB and owned Gradle daemon are stopped after verification.
  Download archives and the duplicate standalone NDK are removed; SDK/AVD caches
  remain for the next slice. No personal credentials or real infrastructure touched.
- Next single slice: actual Android SSH authentication/PTY with this app-owned key
  and strict host trust against an owned local SSH fixture. Then authenticated
  remote create/readiness-to-SSH integration and keyboard/lifecycle, followed by an
  explicitly approved disposable real VM gate. ARM64 physical runtime, release,
  iOS and the rest of the roadmap remain open; no requirement is silently dropped.


## Commit checkpoint: Android SSH and keyboard work — 2026-10-19

- Scope: preserve the accumulated roadmap work and push the current source at the
  user's request. This checkpoint does not close additional roadmap gates.
- Added an opt-in owned localhost SSH fixture and actual Android native SSH
  harness. One earlier emulator run completed authentication, strict host trust,
  shell, interrupt, reconnect and cleanup. Subsequent stronger input checks exposed
  duplicated/lost characters in Android xterm input; reliable mobile typing is not
  established by that earlier pass.
- Prepared a standard Android WebView terminal input adapter with special-key
  controls and keyboard-aware Activity insets. Its current source must still pass
  an actual rebuilt APK keyboard/SSH journey. Earlier APK evidence in the preceding
  section applies to the earlier source, not this adapter. No current APK rebuild
  result or temporary toolchain/log availability is assumed at this checkpoint.
- The host SSH fixture previously built and passed scoped Clippy; three affected
  application-identity tests passed. This checkpoint runs frontend production build,
  Android harness JavaScript syntax checks and diff whitespace review only; no
  unrelated server/workspace suites are repeated.
- No real VM, production infrastructure or personal credential mutation occurred.
- Next single task: rebuild the Android APK and validate the new keyboard adapter
  through the owned emulator SSH fixture, including Enter, resize and Ctrl-C.


## Android input transport-boundary regression — 2026-10-19

- Scope/acceptance: check the prepared Android input adapter's exact outbound
  bytes, composition and special-key segment boundaries before native validation.
  The previously recorded Android SDK/JDK/emulator prefix is absent on this host;
  no current emulator/build process or native runtime evidence is assumed.
- Added opt-in `--app-ssh --android-input` mode to the existing owned Firefox
  fixture. It mounts the actual React mobile component via a fixture-only user
  agent, with simulated native IPC and no production behavior changes/dependencies.
- Verification: `node experiments/instances-browser/check.mjs --app-ssh
  --android-input` passes exact typing/replacement bytes, composition held until
  commit without a duplicate final input, Unicode codepoint deletion, Enter,
  Tab/Esc/Backspace segment reset, Ctrl-C/disconnect and stale selection/session
  cleanup. The first run exposed an incorrect expected byte-count in the new
  test; corrected that assertion and the expanded final run passes.
- No application code changed in this slice; no unrelated suites or prior build
  checks repeated. Native WebView/IME/SSH, viewport behavior and the complete
  mobile journey remain unverified. No infrastructure or credential mutation.
- Next single task: restore an isolated Android toolchain, rebuild the x86_64
  debug APK and execute the owned native SSH/keyboard harness. The roadmap remains
  active; this browser check does not close Phase 0F or Phase 5 mobile acceptance.


## Android toolchain restored; APK rebuild in progress — 2026-10-19

- Previous goal turn made progress: browser input assertions and documented
  evidence were added; native acceptance remains open.
- Restoring the owned toolchain under /tmp/vmanyware-android-resume. JDK 21.0.2
  and Linux command-line tools 16111833 were downloaded from their official
  HTTPS repositories and verified against the published SHA256 / repository SHA1.
  No system Java/SDK configuration or personal credential file was changed.
- SDK installation completed successfully in exec session 35502, logging to
  sdk-install.log in that prefix. Packages are
  platform-tools, API 36, build-tools 36.0.0, NDK 28.2.13676358, emulator and
  API 35 google_apis x86_64. Package activation completed; no installer restart is needed.
- env.sh in that prefix sets the owned Java/SDK/AVD/Gradle paths. The newest
  command-line package delegates SDK installation to Android CLI; installed
  packages are staged under sdk/.sdk before activation. Tool output reports CLI
  1.0.16625370. KVM is present and accessible.
- Rebuilt the current frontend successfully for APK embedding; this is preparation,
  not Android runtime evidence. No application/backend suite repeated.
- Added the native SSH harness invocation/guards to its README and ADR 0049 for
  the prepared streaming input decision and its remaining editing/runtime limits.
- Created only vmanyware-native-smoke with Pixel 6/API 35 and a 2 GiB data
  partition. Owned emulator is live in exec session 92711, serial emulator-5580,
  ADB port 5047, JWT-protected gRPC 8558; sys.boot_completed returned 1.
- Current x86_64 rebuild is live in exec session 12748, logging to apk-build.log
  in that prefix. Rust Android compilation passed; Gradle distribution download
  finished and APK assembly is pending. Revalidate the exact handle/log before
  waiting; do not restart because output pauses.
- Next: finish this APK build, install it only on the guarded owned emulator,
  then run the native SSH/keyboard harness with the existing
  target/debug/examples/android_ssh_fixture executable. No real provider/VM or
  personal credential mutation. Native acceptance remains open.


## Current Android APK: native SSH/input/PTY — 2026-10-19

- Scope/acceptance: rebuild the prepared mobile input adapter and execute actual
  native SSH, shell input, resize, interrupt, reconnect and cleanup against the
  owned generated localhost fixture. This slice advances the mobile journey,
  not the complete roadmap or the real-VM gate.
- Restored isolated JDK/SDK/NDK/AVD/Gradle tooling at
  /tmp/vmanyware-android-resume; env.sh retains its paths. SDK install session
  35502 and APK build sessions 12748/26699 completed successfully. Frontend and
  x86_64 Rust compile plus Gradle debug APK assembly pass on the current source.
- An independent owned TCP listener probe returned `Network is unreachable`
  from the emulator toward 10.0.2.2. Two host-alias native attempts consequently
  failed before authentication. Added explicit opt-in
  VMANYWARE_ANDROID_SSH_ADB_TUNNEL=1: no-rebind mapping only for the owned
  ephemeral SSH port, report labels the tunnel, and finally removes that mapping.
  Direct emulator networking remains unresolved; no trust verification bypass.
- Initial tunneled native run passed. Actual screenshot review exposed vertically
  stacked special controls from a generic narrow-screen rule. One scoped CSS
  override now wraps those controls with 44px minimum targets. Rebuilt/reinstalled
  the updated APK. Strengthened the native layout assertion accordingly.
- Repeated resize checks exposed a fixture race: ADB typing reveals the keyboard
  after the earlier size sample. The harness now waits for size stability after
  typing and before submitting `stty size`. Final run (exec 3496) passes native
  trust/changed-key refusal, wrong phrase, auth-only/no channel, real shell input,
  rotation and actual stty dimensions, Ctrl-C/status130, reconnect, process-death
  shell/socket cleanup and persisted host trust. Three auth attempts/two PTYs;
  final connections zero, all shells closed. The terminal screenshot was reviewed.
- Evidence: ignored .lab/android-ssh/{report.json,ssh-report.json,artifacts.json,
  launch.png,ssh-terminal.png}. artifacts.json records current APK SHA256/size.
  Logs are under the owned prefix (frontend-build.log, apk-build.log,
  native-ssh.log). Native harness syntax and diff whitespace checks pass. No
  unrelated server/Rust/browser suite rerun. ADR 0049 records input decisions.
- Owned emulator session 92711 is stopped; dedicated ADB server and owned Gradle
  daemon stopped. Harness reverse mapping list was empty after cleanup. SDK/AVD
  caches remain for reuse; no personal credentials or real VM/provider touched.
- Limits: ADB injection with a visible software keyboard is not full IME,
  accessibility, physical ARM64, suspend/resume, iOS or release proof. The latest
  x86_64 APK is built/run; no new ARM64 build/run is claimed. Direct network and
  the authenticated create/readiness-to-SSH journey remain open.
- Next single slice: actual Android authenticated HTTPS control-plane connection
  and read-only recovery against an owned local service, then integrate confirmed
  creation/readiness to SSH. Resolve/report the emulator network boundary rather
  than treating the ADB tunnel as deployment evidence. Real VM writes still need
  explicit concrete-target approval; the overall roadmap stays active.


## Android authenticated HTTPS and read-only recovery — 2026-10-19

- Scope/acceptance: actual Android native secure connection, issuer/hostname/token
  refusal and read-only retained-operation recovery after restart against an owned
  local production service. Earlier keyboard/browser changes in the dirty tree
  retain their previously recorded checks; this slice does not repeat them.
- Found missing Android integration for reqwest 0.13's platform TLS verifier.
  Added Android-only JNI/verifier dependencies already present transitively in
  Cargo.lock, synchronous application-context initialization before Tauri startup,
  Cargo-matched Kotlin artifact resolution and ProGuard retention. Current APK
  launches successfully through this initialization; no TLS fallback/bypass.
- Added optional single public PEM CA input (16 KiB maximum), validated natively
  and scoped to the current profile. Android cannot merge extra platform roots:
  the first CA run exposed a client-builder failure, corrected by using only the
  supplied CA with normal WebPKI signature/validity/hostname verification. No
  device certificate/credential store changed. Public CA clears on disconnect;
  tokens still clear on submission and no connection persists across restart.
- New opt-in --remote Android harness launches the actual production server
  executable, a generated HTTPS proxy and a disposable SQLite database with one
  seeded failed creation/target. Child environment has no provider configuration;
  the proxy rejects non-GETs. This is recovery evidence, not VM creation evidence.
  TLS private keys/tokens stay in the owned fixture and are never printed; the
  generated directory is removed after stopping the owned service.
- Verification: current frontend production build and x86_64 Rust/Gradle APK
  build pass. Final actual Android run (exec 60023) passes invalid/private/
  bundled/oversize CA refusal, unknown issuer and hostname mismatch before HTTP,
  actual server token rejection, authenticated connection, environment/history
  reads, actionable missing-catalog error, force-stop/relaunch without request
  replay or restored credentials/profile, explicit reconnect/read-only recovery,
  and disconnect/CA clearing. All 11 recorded requests are GETs. Native recovery
  screenshot reviewed. Early fixture attempts also exposed a missing remote-mode
  selection and a screenshot-before-scroll race, both corrected.
- Targeted Firefox --remote-connection scenario passes (exec 77791), including
  forwarding/locking the optional CA and existing affected create/recovery callers.
  New harness JavaScript syntax, changed Rust formatting and diff whitespace pass.
  Attempted new Linux parser unit test could not compile: WebKitGTK 4.1/pkg-config
  prerequisite is absent on this host. It is not reported as a passing unit run;
  corresponding validation cases passed through real Android native IPC instead.
  No full server/workspace or unrelated keyboard suite rerun.
- Evidence: .lab/android-remote/{report.json,remote-report.json,artifacts.json,
  remote-recovery.png,launch.png}; artifacts.json hashes the current APK. Logs
  under /tmp/vmanyware-android-resume: remote-frontend-build.log,
  remote-apk-build.log, native-remote.log, remote-browser.log and ca-test.log.
  ADR 0050 records the security/runtime choice. Android SSH evidence above is for
  its earlier APK; no new SSH run is claimed for this TLS-only source change.
- Owned service, TLS sockets, generated keys/DB and both ADB reverse mappings are
  cleaned up; the mapping list is empty. Emulator session 79401 and owned Gradle
  daemon are stopped; the dedicated ADB server is stopped. Retained
  toolchain/env.sh allow reuse.
- Limits: explicit ADB tunnel remains necessary; direct emulator/deployed network,
  actual OS/public-root positive verification, complete create/readiness-to-SSH,
  physical devices, iOS, full IME/lifecycle and release remain unproven. No real
  Proxmox/VM or personal credential operation occurred. Full roadmap stays active.
- Next single slice: integrate confirmed creation/readiness with this native
  Android HTTPS connection against the owned generated provider fixture, then
  connect the resulting ready target to the app's SSH flow. Real VM mutations
  remain gated by explicit approval of a concrete disposable target/operation.


## Ready creation/recovery to explicit SSH target — 2026-10-19

- Previous goal turn made progress: native HTTPS verification, optional private CA
  and actual read-only recovery evidence were added. Full roadmap remains open.
- Scope/acceptance: connect the existing ready-result UI to the existing SSH
  inspection/authentication flow in remote mode, preserving explicit trust and
  clearing old state on reselection/disconnect. Inspection of current source showed
  that remote mode hid the local terminal and had no handoff action.
- Fresh succeeded/guestReady creation now offers Prepare SSH connection, carrying
  exact recorded IPv4 and native plan guestUser. Recovery offers the same explicit
  action with address only; account remains blank because its contract lacks a
  verified username. Queued/failed results offer no handoff. No automatic network
  request, trust, unlock or login occurs on readiness or target selection.
- Remote connection owns the selected target in memory. Selecting/remounting its
  host-trust flow clears old challenge/authentication/terminal state; disconnect,
  connection replacement and leaving remote mode remove it. Heading focus/scroll
  moves to the selected target. Existing local/default-account callers are intact.
- Verification: production frontend build passes (exec 37739); expanded targeted
  Firefox --remote-connection fixture passes (final exec 52378): queued/failed
  guards, no automatic inspection, exact address/account, independent unknown-host
  consent followed by fresh inspection, reselection/disconnect reset, recovery
  username left empty, and affected stale create/recovery behavior. Separate
  --app-ssh fixture passes for default-account callers, explicit identity,
  changed-host refusal and stale terminal cleanup. The broader browser scope is
  warranted by changing shared host/authentication component props. No server,
  Rust, unrelated Node/controller or Android suite repeated for this TS-only diff.
- Logs: /tmp/vmanyware-android-resume/handoff-build.log, handoff-browser.log,
  handoff-ssh-regression.log. Diff whitespace review passes. ADR 0051 and the
  provisioning/browser documentation record behavior and proof boundaries.
- Earlier dirty TLS/Android input files retain their separately recorded evidence.
  This slice does not rebuild/run an APK or establish native handoff, real VM
  creation, full software IME or direct remote-network support. Toolchain remains
  retained; no new emulator/service process, credentials or VM mutation started.
- Next single slice: exercise confirmed creation and readiness on actual Android
  against the production service plus an owned generated provider fixture, then
  use this explicit handoff to enter the native SSH flow. Treat browser IPC results
  as component proof only; retain the approved disposable real-VM gate and the
  rest of the original roadmap.


## Android native confirmed creation to SSH fixture — 2026-10-19

- Previous goal turn made progress: explicit ready-target/account handoff was
  implemented and browser-verified. This slice supplies native integration evidence.
- Scope/acceptance: actual packaged Android confirmed creation through the
  production service/worker against an owned generated HTTPS provider, followed
  by explicit handoff and same-key native SSH/PTY; no real infrastructure changes.
- Added opt-in --remote --create mode with a loopback, in-memory Proxmox protocol
  peer. Service starts from an empty disposable DB and clean fixture-only
  environment, with generated token/CA/catalog and execution enabled only there.
  Proxy accepts GET plus exact plan/definition/create POST paths in creation mode;
  normal remote mode remains GET-only. No production API/worker code changed.
- Rebuilt x86_64 debug APK with the already verified current frontend handoff
  assets (build exec 11552 passes). Native UI unlocks its existing Android key,
  inserts the exact public key, previews without creation, requires explicit
  consent and submits one create request. Production worker reaches guestReady
  through clone/resources/Cloud-Init/seed/start and strict guest-proof validation.
- Final native run exec 26350 passes: three API POSTs (plan, definition, create),
  exactly one clone/seed/start/fixed cloud-init-status execution and two config
  writes; 2 CPUs/2048 MiB/no ballooning and the same Android public key verified
  at the provider boundary. Exact 192.0.2.42/debian handoff review appears only
  after readiness and an explicit action. No automatic host inspection/auth.
- SSH then explicitly overrides that synthetic guest address with the owned ADB
  tunnel endpoint. The same key passes native strict host trust, wrong phrase,
  auth-only/no channel, real OS-shell input, rotation/stty, Ctrl-C/status130,
  disconnect/reconnect, process-death cleanup and changed-key refusal. Three auth
  attempts/two PTYs, final connections zero/all shells closed. Provider/API
  mutation counts remain unchanged across SSH and application restart.
- Early runs exposed fixture-only selector/route issues: header class mistaken
  for the key details panel; missing current=1 config query; and generic terminal
  Disconnect hitting the enclosing server button. Corrected selectors/fixture
  route and scoped shared SSH buttons to their trust section. Existing targeted
  remote Firefox scenario passes after its matching key-panel selector correction
  (exec 7834). No unrelated browser/server/Rust suites rerun.
- Native harness JS syntax/diff whitespace review passes. Evidence is ignored
  .lab/android-create/{create-report.json,ssh-report.json,report.json,
  artifacts.json,ssh-terminal.png,launch.png}; APK hash/size are recorded.
  Screenshot reviewed: terminal output and wrapped controls visible above keyboard.
  Logs: /tmp/vmanyware-android-resume/create-apk-build.log, native-create.log,
  create-browser-selector.log. ADR 0052 records boundaries. Earlier dirty source
  slices retain their separate evidence; no general roadmap completion claimed.
- Owned provider/service/SSH fixture, generated TLS/DB directory and reverse
  mappings cleaned up; reverse list is empty and no owned server/SSH process
  remains. Emulator session 2706, dedicated ADB server and owned Gradle daemon are stopped;
  retained SDK/AVD/env.sh support reuse.
- Limits: synthetic provider/guest readiness, explicit SSH endpoint override and
  ADB injection are not real VM provisioning, direct networking or full software
  IME proof. No physical-device, iOS or release claim. No real provider, personal
  key, production infrastructure or user credential store was touched.
- Next single slice: resolve the owned emulator host-alias/direct-network boundary
  and repeat the native HTTPS/SSH transport checks without ADB reverse mappings.
  Then complete the remaining mobile usability/lifecycle and explicitly approved
  real disposable VM gate while retaining the full original roadmap scope.


## Direct emulator HTTPS/SSH transport — 2026-10-19

- Previous goal turn made progress: actual native creation/API/worker/provider
  integration and same-key SSH passed through an explicit tunnel. This slice
  removes that transport limitation without changing production application code.
- Scope/acceptance: diagnose the owned host-alias boundary and validate native
  verified HTTPS and strict SSH/PTY without data-plane ADB mappings. The route
  now shows a validated default Wi-Fi network; a correctly bounded owned TCP
  marker probe reaches 127.0.0.1 through 10.0.2.2. No route/firewall/emulator
  feature/device trust mutation was needed. The exact cause of older transient
  network-unreachable failures is unproven; do not claim a permanent defect/fix.
- Added explicit VMANYWARE_ANDROID_HTTPS_DIRECT=1 to the opt-in harness, using
  host alias 10.0.2.2 and a matching SAN while preserving the separate wrong-host
  certificate test. SSH already defaults to the host alias without its tunnel
  opt-in. Creation mode now supports that direct transport and labels both paths
  accurately. No automatic fallback or TLS/SSH-verification bypass.
- Direct-mode startup now checks an empty reverse list and performs a bounded
  owned TCP readiness probe independently of sys.boot_completed. Combined direct
  creation checks an empty reverse list again at completion. ADB/CDP only drives
  and observes UI; API/SSH bytes use normal emulator TCP.
- Verification: first direct native journey passes (exec 3336), then stop/restart
  the exact owned emulator for a cold-boot repeat with the new readiness gate.
  Final exec 23770 passes current-APK TLS issuer/hostname/token refusal,
  confirmed creation through actual service/worker and generated provider,
  resource/key checks, exact ready-target handoff, direct host-alias SSH trust/
  real-shell input/rotation/stty/interrupt/reconnect/process-death/changed-key
  checks and no creation replay. Probe: one attempt, 439 ms, zero reverse mappings.
  HTTPS report is direct; SSH target is host alias; final SSH connections zero.
- APK unchanged from the previously verified handoff build; no frontend/Rust/APK
  rebuild or unrelated suite rerun for harness-only changes. Changed JS syntax
  and diff whitespace checks pass. Evidence: .lab/android-direct/{report.json,
  create-report.json,ssh-report.json,artifacts.json,ssh-terminal.png,launch.png};
  hashes are recorded. Logs under /tmp/vmanyware-android-resume:
  native-direct.log, direct-emulator.log; diagnostic connectivity.txt/wifi.txt
  capture only the owned AVD. ADR 0053 records boundaries and upstream reference.
- Owned service/provider/SSH sockets and generated TLS/DB data cleaned up;
  reverse mapping list remains empty. Emulator exec 20471 and its dedicated ADB
  server are stopped; toolchain remains reusable. No real provider/VM, personal
  key, host firewall, device trust/credential store or production service changed.
- Limits: synthetic guest endpoint is still explicitly replaced by the host alias
  and ephemeral SSH port. Direct emulator networking is not deployed/LAN/VPN or
  physical-device evidence. Real VM, full software IME/background lifecycle,
  OS/public-root positive verification, iOS/other platforms and release remain
  open. This evidence supersedes only the emulator host-alias blocker.
- Next single slice: actual software-keyboard taps and composition plus native
  background/resume terminal behavior, then the remaining mobile/platform and
  explicit disposable real-VM gate. Preserve the full roadmap, including pending
  snapshot/recreate/delete and deployment/release requirements.

## Android brief background/resume — 2026-10-19

Scope: verify HOME/background/return on the current Android native SSH terminal
without implicit reconnect or input replay. The optional
`VMANYWARE_ANDROID_BACKGROUND=1` native SSH fixture now observes the real WebView
visibility transition, waits two seconds while hidden, and compares server-side
authentication attempts, channel count and received input bytes before/after.
Explicit return resumes the same shell and produces fresh `RESUME_OK` output.

Earlier diagnostic runs passed this scenario, but a separate initial software
keyboard check proved insufficient: Android/Gboard reports the input view shown
while screenshots and native window geometry show only the bottom navigation
strip, with no letter grid. Later terminal screenshots do show the full grid;
the trigger is not isolated. Removed the exploratory capture-only check rather
than count it as successful software-key input. No product behavior was changed.

Limitations: two-second foreground/background transition only; no screen lock,
Doze, long suspension, process eviction or background-service guarantee. Text
commands still use ADB injection; software-key taps/composition remain open.
Next task: isolate initial Gboard letter-grid visibility and verify actual taps
with exact server-side byte/output assertions. Keep real VM operations gated.

Verification: final cleaned-up `--ssh` run with the background flag passed on
Android 15/API 35 x86_64, WebView 124, via direct 10.0.2.2 transport. Evidence:
`.lab/android-background/ssh-report.json` and terminal screenshot. JavaScript
syntax checks and `git diff --check` passed. No APK rebuild or unrelated suites
were needed for this harness/documentation-only slice.

## Android actual software-key taps — 2026-10-19

Scope: prove taps on displayed Gboard keys reach the real SSH shell, including
editing and sending, without interpreting the IME shown flag as sufficient proof.
Added an opt-in `VMANYWARE_ANDROID_SOFTWARE_KEYS=1` stage to the owned native SSH
fixture. It is confined to the reviewed Pixel 6 1080×2400 English QWERTY layout
and checks the Gboard window/touch region before tapping. Every letter/space and
backspace is checked against the actual controlled input. `echo imeox`, Gboard
backspace, `k`, and Gboard send must yield exactly 13 additional received SSH
bytes, a cleared input and real shell output `imeok`, without new channels or
authentication. The existing screenshot captures the full keyboard before taps.

The first run passed with the brief background/resume scenario also enabled.
This proves actual ASCII software-key input on this owned emulator after the
rotation/control-key sequence; it does not fix/prove initial-focus keyboard
visibility, Unicode/composition, other IMEs, iOS or physical devices. No product
code or APK change was needed. Next task: isolate the initial Gboard key-grid
visibility issue before extending the real-key composition scenario.

Verification: final run with the stricter Gboard-window guard and both keyboard/
background flags passed on Android 15/API 35 x86_64 and WebView 124, using direct
10.0.2.2 transport. Evidence: `.lab/android-software-keys/ssh-report.json` and
`ssh-terminal.png`. JavaScript syntax checks and `git diff --check` passed;
no unrelated suites or application rebuild were run. Emulator/owned ADB server
were stopped after verification.

## Scoped snapshot listing — 2026-10-20

Previous goal turn made progress: actual Gboard taps yielded exact SSH bytes and
shell output. This turn revalidated the workspace and found the prior temporary
Android SDK/emulator prefix absent, with no live process to wait for. No mobile
product fix or fresh native evidence is claimed. Continued with an independent
roadmap slice instead of recreating a large toolchain for another diagnostic.

Implemented read-only `GET /api/v1/environments/{id}/snapshots`, fixed to the
registered instance's provider scope/node/VMID. Added the fixed Proxmox read,
normalized sorted metadata, synthetic-current exclusion, duplicate/invalid-data
refusal, preserved incomplete provider state, safe permission/timeout errors and
static diagnostic route naming. No dependencies, schema migrations, operation
records or VM writes were added. Contract and limits: docs/snapshots.md.

Verification: `cargo test -p vmanyware-server --lib snapshot_list -- --nocapture`
passes all four targeted tests (85 unrelated tests filtered out). They exercise
the actual Axum route and verified generated-HTTPS provider. An initial timeout
case failed because it used the normal client timeout; corrected the fixture to
use an explicit 200 ms deadline, then reran the four tests successfully. Formatting
and diff review cover this slice's Rust/docs changes; prior dirty Android/client
changes were retained without rerunning their earlier checks. No real Proxmox
request was made. Roadmap listing stays unchecked pending client/provider proof.
Next task: add the explicit read-only snapshot list to the client, including
empty/error/incomplete states and the distinction from independent backups.

## Remote snapshot-list UI — 2026-10-20

Previous goal turn made progress: scoped server snapshot listing and four targeted
HTTPS/API tests passed. This slice adds the remote native read command and actual
React list under each registered environment. Reads remain explicit and bound to
the active connection/canonical instance ID; replies are discarded after native
disconnect and UI unmount. UI binding additionally checks instance/scope/node/VMID.
Displays empty/provider-error/unknown/incomplete states and parent metadata,
escapes descriptions, and distinguishes snapshots from independent backups.
No creation, restore, new dependencies, schema changes or real VM requests.

Verification: `npm --prefix apps/client run build` passed. The focused
`node experiments/instances-browser/check.mjs --snapshots` passed with simulated
IPC: explicit read, exact selected target, mismatched target rejection,
empty/error distinction, metadata, escaped description, pending-read disconnect
and no mutation. The first browser attempt clicked the workspace button before
it became enabled; fixed that fixture synchronization, then the scenario passed.
Three native generated-HTTPS tests are added for fixed authenticated path,
malformed/authentication/path rejection and in-flight disconnect suppression.
`cargo test -p vmanyware-client --lib remote_snapshot_read -- --nocapture` stops in
webkit2gtk-sys because WebKitGTK 4.1 pkg-config metadata is absent; the new native
module/tests have not compiled or run here. Rust formatting, JS syntax and diff
checks pass. Prior dirty Android/server/client slices were preserved; no unrelated
suites were rerun. No new APK/native launch or real-provider evidence is claimed.
Next task: restore an owned native WebKitGTK build prefix and run the filtered
snapshot HTTPS tests, then validate the actual native list-to-server flow.

## Native snapshot command validation — 2026-10-20

Previous turn made progress: remote snapshot React UI and focused browser checks
passed, while native compilation stopped at a missing system library. Revalidated
that boundary, then recreated the seven-package WebKitGTK 4.1 library prefix in
ignored `.lab/native-deps`, using verified HTTPS, system public-keyring signature
verification and extraction without installation hooks. Keyring integrity: 17
files, zero alterations. Exact package versions/URLs/hashes/signature output and
setup helper remain in that owned directory. No system packages or personal
credential files were changed. See docs/native-linux-prefix.md for reuse.

Verification: actual `vmanyware-client` crate compilation passed, followed by all
three `remote_snapshot_read` tests (51 unrelated tests filtered out), proving the
fixed authenticated HTTPS path, unknown/incomplete metadata, invalid data/auth/
path rejection and in-flight disconnect suppression. Because the prior snapshot
slice modified shared ReadPath code, also ran the three `remote_workspace` tests
and the one fixed-health-path/token test; all four passed. No source correction
was needed. Native snapshot log: `.lab/native-deps/snapshot-native-tests.log`.
Documentation diff review and `git diff --check` passed; no frontend/server suites
were rerun. Earlier blocked-test notes are historical, superseded for this prefix.

Limitations: generated HTTPS peer, no WebView-driven snapshot IPC/server journey,
real provider, VM operation, packaged startup or mobile rebuild in this slice.
The prefix contains libraries only, not the previous Xvfb additions. Next task:
validate the real native WebView -> production command -> actual server snapshot
list with disposable generated infrastructure, then proceed to snapshot creation.

## Native snapshot-list vertical journey — 2026-10-20

Previous goal turn made progress by restoring a verified native library prefix
and passing the three snapshot plus four shared-read tests. This slice adds an
optional `--snapshots` mode to the existing isolated native fixture and a focused
real-UI driver, without changing normal application startup or injecting mock IPC.
The fixture registers production remote commands and connection state, uses an
actual authenticated server behind a verified generated HTTPS proxy, and reads
only a generated HTTPS provider. Lifecycle execution is disabled. The verified
owned prefix now also contains Xvfb/xauth/libxmu; all ten package signatures and
archive details are retained in `.lab/native-deps/packages.json`.

Acceptance met: explicit native reads show empty, incomplete and denied states,
retained target, unknown metadata, escaped description and backup wording; error
clears prior results and explicit disconnect succeeds. Runner asserts exactly
three fixed provider snapshot reads, zero provider writes and unchanged local
association. TLS, sandbox and X11 cookie checks were not disabled. Evidence:
`.lab/native-deps/snapshot-ui-check.log` ends with the full real-native journey pass.

Verification: built the affected example with `tauri/custom-protocol` and actual
server binary; native scenario passed. The initial imported production module
needed explicit child paths, now fixed for both create/snapshots; all three
filtered native snapshot tests passed again after that change. A plain initial
example build waited for Vite and expired without reaching the server; the
embedded-asset build corrected the invocation. JS/Python syntax, targeted Rust
formatting and `git diff --check` pass. No unrelated suite/frontend rebuild was
run. Owned display/process groups, proxy/provider and generated temporary data
were cleaned up by the runner. Minimal-session portal/accessibility warnings
remain; they did not prevent the asserted journey.

Limitations: isolated Linux/generated infrastructure only, not installed startup,
real Proxmox, mobile rebuild, snapshot creation or restoration. Roadmap list item
stays unchecked until real-provider evidence exists. Next task: implement bounded
validated snapshot creation and durable task/operation tracking, with explicit
target/action confirmation; keep real VM operations behind concrete approval.

## Snapshot creation prerequisites — 2026-10-20

Previous goal turn made progress: real native snapshot-list journey passed. This
slice prepares creation with `POST /api/v1/environments/{id}/snapshot-preflight`,
not a provider write. Validates exact 2–40 character Proxmox-compatible names,
reserved current, bounded UTF-8 descriptions and no unknown request options.
The fixed disk-only path reads actual provider feature support, existing snapshot
names/incomplete states, retained scope/target and lifecycle identity guards, then
rechecks pending/configuration identity. Complete reads are bounded to 20 seconds.
The report binds request/instance/scope/node/VMID/definition/digest and always says
`executionEnabled: false`. No reservation, approval, operation or VM write occurs.
ADR 0054 records disk-only semantics and the durable no-replay requirements for
the next slice. No dependencies, schema migration or client changes were added.

Verification: four filtered `snapshot_preflight` tests passed, then the added
actual HTTP bound-report test passed separately (five distinct tests total).
The HTTP test was repeated only after adjusting the body limit to 32 KiB and
adding a valid 4 KiB description whose escaped JSON exceeds 8 KiB; it passed.
Coverage includes request grammar/bounds, retained scope, fixed GET-only target,
provider support/malformed flags, duplicates, incomplete snapshot work, final
configuration changes, complete report and no operation record. Rust compilation,
targeted formatting and diff checks pass. Existing lifecycle/list/native/browser
suites were not rerun: their behavior was not modified. Generated HTTPS/local
SQLite only; no real provider, guest consistency or write permission was tested.

Creation itself remains incomplete: durable approval/expiry/hash, conflict guard,
no-replay dispatch marker, task persistence/reconciliation and client confirmation
must exist before exposing the mutation. Next task: add the confirmed snapshot
creation operation to the durable journal and worker, with crash/lost-reply tests;
real VM operations still require concrete target/action approval.

## Durable snapshot creation storage — 2026-10-20

Previous goal turn made progress: creation preflight and five targeted tests
passed. This slice adds schema 11/shared `vm_snapshot_create` operation storage,
full confirmed target/request hashing, retained same-key identity/age, common
mutation exclusion, incomplete-provisioning blocking, claim/expiry/exact-approval
guards, irreversible dispatch marker, qmsnapshot task binding, guarded completion
and existing exclusive-startup recovery semantics. Unknown submissions remain
uncertain and cannot be claimed; known tasks retain their marker for read-only
reconciliation/backoff. No provider POST, runtime worker, enqueue API or client
creation control is exposed; preflight stays disabled. ADR 0055 records choices.

Migration rebuilds the shared operations table, retaining provisioning children,
indexes and guards. It temporarily disables foreign keys on the unopened migration
connection, checks the graph inside the transaction and restores enforcement
before exposing the store. Invalid graphs roll back including schema version.
Normal schema-11 opens do not rescan the entire foreign-key graph. Existing tests'
expected latest schema versions were updated from 10 to 11; historical migrations
were retained unchanged. No dependencies or real infrastructure changes.

Verification: 53 existing affected storage tests passed (10 instance, 9 lifecycle,
34 provisioning-intent). Seven of the initial eight new integration tests passed;
the corruption fixture initially failed to inject an orphan because foreign keys
were enabled, so explicitly disabled them only in that owned fixture, then both
migration tests passed. Added concurrency, corruption/claim-rollback and actual
store-connection foreign-key enforcement checks; these passed. The expiry test
was repeated after adding proof that same-key retry cannot renew age. Total:
11 distinct new tests passed across filtered runs (10 integration, 1 unit), plus
53 existing affected tests. Targeted Rust formatting and diff checks passed.
No unrelated provider/native/browser suite was rerun; the broader storage coverage
was justified by rebuilding their shared table. SQLite/owned fixtures only; no
claim of provider dispatch, successful snapshot or restore is made.

Next task: implement fixed snapshot transport and runtime executor with explicit
capability, repeated preflight, dispatch-marker-before-POST ordering, retained task
polling and GET-only restart reconciliation; verify generated HTTPS crash/lost-
reply scenarios before exposing confirmed HTTP enqueue/client controls.

## Snapshot transport/executor and production queue — 2026-10-20

Previous goal turn made progress: schema-11 journal and affected storage tests
passed. This slice connects a fixed one-shot disk-only snapshot POST transport to
the durable executor. It rechecks exact target/preflight/digest, commits dispatch
before POST, persists only a matching qmsnapshot/node/VMID task before polling,
then requires successful task plus independently verified retained marker and
exact complete disk-only snapshot metadata. Snapshot creation changes digest;
completion verifies marker/template/lock rather than the old digest. Unknown
submission remains uncertain/unclaimed; known-task reopen/recovery is GET-only
and works with new dispatch disabled. Added safe snapshot-specific failure codes.

The main binary now validates independent VMANYWARE_SNAPSHOTS_ENABLED (absent/
false by default, exact true opt-in), and the sequential worker services one
snapshot claim after existing queues. Existing public worker entry points retain
default-disabled snapshot behavior. Confirmed HTTP enqueue/client submission are
not wired, and preflight still reports execution disabled. No write permission,
real VM operation, dependency or schema change was added in this slice.

Verification: six generated-HTTPS executor/transport tests passed, then the added
production-round test passed separately (seven executor tests total). They cover
encoded Unicode/ampersand/plus/newline, one fixed POST, pre-dispatch guards,
unknown/wrong task replies without replay after reopen, retained-task GET-only
reconciliation even when disabled, task failure and foreign/RAM metadata refusal.
The independent flag unit test and two actual-binary configuration/queue tests
passed (10 distinct new tests total). The binary tests use owned SQLite and no
provider: default-disabled fails without a marker; true reaches provider-not-
configured without a marker. Invalid flag stops before database creation and
never echoes its input. Actual native/UI/client/server-provider process journey
for creation is not claimed. Targeted Rust formatting and diff checks pass;
previously passing storage/native/browser suites were not rerun without changes.

Next task: add runtime-capability reporting and a confirmed idempotent snapshot
HTTP enqueue route bound to the whole reviewed target/request, with disabled,
stale, conflicting and same-key retry coverage; then add the client confirmation
and validate the real native creation journey against generated infrastructure.

## Confirmed snapshot HTTP submission — 2026-10-20

Previous goal turn made progress: one-shot transport, executor, opt-in runtime
queue and 10 targeted tests passed. This slice adds confirmed
`POST /api/v1/environments/{id}/snapshots`, capability-aware router/main wiring and
preflight runtime reporting. Existing router constructors default snapshots off.
Approval must match route ID, saved definition/scope/node/VMID and exact validated
snapshot request, with explicit consent and exactly one bounded idempotency key.
Configured bearer protection and origin refusal apply. Disabled/invalid/unconfigured/
changed/conflicting requests do not enqueue. Identical retries preserve operation
identity/age; accepted requests return 202/Location and never contact the provider.
Fresh preflight/digest checking stays in the worker; HTTP acceptance is not success.

Verification: seven new targeted HTTP tests passed across the initial run and
one corrected assertion rerun. They cover queue/read/retry identity, independent
flag, target/consent/unknown options, duplicate/missing keys, origin/size/auth guards,
active mutation conflicts and concurrent retries. Two new filtered HTTPS/API tests
also passed: actual enqueue -> production queue -> generated provider -> result
(success vs stale digest with zero POST), and capability independent of failed
prerequisites. Total nine distinct new tests passed. Two initial test assertions
assumed the wrong existing contract (unknown-field JSON rejection is 422, and
operation.failure is a string); corrected only those assertions and reran the
failed cases. No implementation failure was masked. Rust compilation, targeted
formatting and diff review pass. No schema/dependency/client changes or real VM
requests; unrelated suites were not rerun. docs/snapshots.md now describes the
current coherent contract rather than accumulating superseded pending statements.

Next task: native retained snapshot creation intent/preflight/confirm/status and
client review/consent, with same-key lost-reply resume and stale result suppression;
then exercise actual native creation through generated infrastructure. Real
provider snapshots, restore and complete roadmap acceptance remain open.

## Native snapshot creation intent — 2026-10-20

Previous goal turn made progress: confirmed HTTP enqueue and nine API/HTTPS tests
passed. This slice adds production native prepare/confirm/read/clear commands.
Preparation GETs the exact saved instance then POSTs read-only preflight, binding
all retained target/request fields, disk-only mode and the complete known check
set. Native revision checks reject superseded prepares before follow-up or state
replacement. Server check messages/unknown metadata are dropped. Confirmation
requires an eligible retained review, explicit consent and its existing key;
renderer cannot replace approval fields. It retains submitted/inflight/operation
identity, blocks duplicate concurrent POSTs, allows unchanged unknown-response
retry only, then GET-only status once an ID is known. Completion requires matching
recognized task-backed disk-only evidence. Explicit clear is limited to an
unsubmitted review or a verified terminal result; uncertain work stays retained.
Connection reset clears this memory, not the server operation.

Verification: eight new filtered `remote_snapshot_create` tests passed. Added a
security response test (authentication marker/status/JSON/256-KiB limit) and an
actual socket-abort test; both passed (ten distinct new tests total). The success
validator and happy HTTPS flow were repeated after requiring the provider-task
known flag, with a negative flag variant added and verified. Exact POST body/key
retention, scope/target/request/check mismatches, failed/disabled prerequisites,
invalid input, in-flight disconnect and stale preparation are covered. Actual
Tauri crate/command registration compile using `.lab/native-deps`; targeted Rust
formatting/diff checks pass. A test-only HTTPS sequence helper's visibility was
widened for reuse; its implementation is unchanged. No frontend/server suites,
APK builds or unrelated native tests were rerun without a behavior change.

Limitations: native commands + generated HTTPS only, no creation form, actual
creation WebView IPC/server journey, persisted native intent or restart recovery.
The existing snapshot list UI/real native listing evidence is unchanged. No new
dependency, server schema or real VM mutation. Next task: add the remote snapshot
review/consent/status form, same-key retry and explicit acknowledgement; then add
read-only server operation recovery for reconnect/restart and verify the complete
native creation journey against generated infrastructure.

## Remote snapshot creation form — 2026-10-20

Previous goal turn made progress: retained native snapshot commands compiled and
ten HTTPS tests passed. This slice adds the actual remote React creation form,
selected explicitly from an environment row. A single workspace review/operation
matches the native one-intent contract. It shows target, exact name/description,
known prerequisites and disk-only/backup/consistency limits. Review is bound to
the selected environment/request; consent is unchecked by default and unavailable
for disabled/failed prerequisites. Inputs/target stay locked until unsubmitted
cancel or verified terminal acknowledgement. Unknown submission exposes identical
same-key retry only; known operations expose explicit GET status refresh. Uncertain
work cannot be acknowledged/replaced. Completion checks reviewed task-backed
metadata before showing success. Disconnect resets selection and suppresses late
responses; it never claims server work was cancelled.

Verification: `npm --prefix apps/client run build` passed TypeScript and Vite.
`node experiments/instances-browser/check.mjs --snapshot-create` passed the focused
real Firefox/React walkthrough under simulated IPC: no automatic work on target
selection, disabled/failed/mismatched review refusal, escaped description, consent,
input/target exclusion, same-key retry, queued/running/uncertain/success read states,
terminal acknowledgement and stale preparation after disconnect. JS syntax and
diff review passed. No Rust/native/server tests were rerun: those sources were not
changed. No new dependency, real VM operation or credential change was made.

Limitations: this is frontend/browser evidence; no native creation window, APK
rebuild or real-provider creation has been tested. Intent/operation recovery after
closing/reconnecting is not implemented; operation ID is shown for preservation.
Next task: exercise the actual native WebView -> retained commands -> authenticated
server -> generated HTTPS provider creation path, then add explicit read-only
server operation recovery for reconnect/restart. Restore and real-provider gates
remain open; the full roadmap is still active.

## Actual native snapshot creation journey — 2026-10-20

Previous goal turn made progress: the React creation form/build and focused
Firefox scenario passed. This slice adds a distinct --snapshot-create mode to
the existing isolated native fixture and registers production creation commands
in the example. The owned server alone opts into snapshots. A verified generated
HTTPS proxy forwards the real preflight/confirmed request, token and idempotency
key to the actual server; its worker contacts only the generated HTTPS provider.
The example embeds the current frontend assets and invokes real native commands.
No application/product source change was needed for this integration proof.

Acceptance met: review/cancel, fresh unchecked review/consent, one confirmed
submission, explicit operation GET refresh, task-backed metadata success,
independent list, acknowledgement and disconnect. The provider accepts exactly
one fixed target/name/description/vmstate=0 POST. Unicode/ampersand/plus/newline
content is checked. The runner requires five snapshot reads, at least one task
poll, unchanged instance association and exactly one succeeded durable operation
with providerTaskKnown, successful task status and complete disk-only metadata.

Verification: targeted native example build with tauri/custom-protocol and actual
server binary build passed. First native scenario passed; repeated only after
adding independent task-poll and server-journal assertions, then passed again.
Evidence: .lab/native-deps/snapshot-create-ui-check.log. Targeted Rust formatting,
JS/Python syntax and git diff checks pass. Frontend/native/server unit suites were
not repeated without source changes. Fixture listeners, process groups and
generated temporary state were cleaned up. TLS, WebKit sandbox and X11 cookie
checks stayed enabled; minimal-session portal/accessibility warnings remain.

Limits: isolated Linux/generated provider only, not a real VM, Android rebuild,
physical device, normal packaged startup, application consistency or restore.
Next task: explicit read-only snapshot-operation recovery from the server after
reconnect/restart, without resubmitting the snapshot or inventing current VM state.
Full roadmap and real-provider snapshot/restore acceptance remain open.

## Scoped server snapshot recovery — 2026-10-20

Previous goal turn made progress: actual native snapshot creation passed with one
provider POST and independent task/journal evidence. This slice adds GET-only
per-instance operation history/detail recovery. It filters instance/kind before
reading another journal, returns recorded target and original request without
approval/grant/task-secret fields, checks retained approval/task identity, and
normalizes successful evidence against exact saved target/request/disk-only
metadata. Unexpected success fields are discarded. Reads are consistent SQLite
transactions; no provider, capability, prepared client intent, claim or enqueue
is needed. History filters before its newest-100 limit and omits large request/
result fields. Bearer protection remains common to all API routes.

Verification: six new HTTP/SQLite tests passed. Added a saved-success check after
introducing typed result normalization; it also passed (seven distinct new tests).
Coverage: database reopen with provider/capability absent, unchanged queued/
running/uncertain operations, scope/kind/ID isolation, old target visible despite
another target's large history, ordering/bounds, corrupted approval rejection
without cross-target disclosure, valid success/unknown-field removal, mismatched
success refusal and authentication. Targeted Rust formatting and git diff checks
pass. Added Deserialize only for existing normalized evidence types; no prior
write behavior, schema, dependency, frontend or native command changed. Existing
passing write/native/browser suites were not rerun without related changes.

Limits: API/journal recovery only. This does not contact a real provider or prove
current snapshot existence/restore, launch/restart the production server process,
or add client recovery controls. Next task: native fixed GET history/recovery
commands and an explicit client history/saved-ID flow that works after connection
reset, binds the selected instance, ignores stale replies and never submits the
saved snapshot again.

## Native saved snapshot recovery — 2026-10-20

Previous goal turn made progress: seven scoped server recovery tests passed.
This slice registers native read_remote_snapshot_history/recover_remote_snapshot
commands. Both use canonical instance/operation IDs, fixed GET paths and the
retained verified HTTPS/token profile with response/authentication bounds. History
checks target binding, 100-entry bound, uniqueness, names and status/failure/task
consistency. Saved detail validates requested identities, recorded target/request,
submission marker and exact task-backed complete disk-only success evidence.
Unknown fields are dropped and raw read failures are replaced by static messages.
Old connection replies are discarded after disconnect or reconnect. Reads do not
create/replace a prepared approval or retain new submission/retry state.

Refactored the existing creation operation validator/name grammar into shared
internal helpers for this working read use case. Creation still validates its
own retained Review; recovery supplies recorded target/request directly, without
inventing a preflight. No dependency, server schema or frontend change.

Verification: seven filtered remote_snapshot_recovery tests pass against generated
verified HTTPS: fixed authenticated GETs without intent, historical states,
identity/target/metadata mismatch refusal, bounded/unique history, disconnect and
reconnect suppression, unknown-field removal, private error handling and invalid
IDs before network access. Actual Tauri crate/command registration compiles with
.lab/native-deps. The initial test build exposed a local helper-name shadowing
error, corrected before execution. Re-ran only the two affected existing creation
validator/happy-path tests after sharing validation; both pass. Targeted Rust
formatting and git diff checks pass. No server/browser/Android or unrelated native
suite was rerun without changes.

Limits: native command/HTTPS evidence only. Recovery is not yet exposed in the
React UI and no fresh recovery WebView/process-restart journey is claimed. No
real provider contact, mutation, credential change or persisted client state.
Next task: per-instance read-only history and saved-operation-ID UI, with explicit
saved/current-state wording, stale result suppression and no submission controls;
then verify it through the actual native fixture after reconnect/restart.

## Snapshot recovery UI — 2026-10-20

Previous goal turn made progress: native GET-only recovery compiled and seven
HTTPS plus two affected creation tests passed. This slice adds a per-instance
React recovery panel below snapshot listing: explicit history, row inspect,
saved-operation-ID lookup and read-only refresh. Selected target/request identity
is checked before display; successful records need matching task-backed disk-only
evidence. The panel distinguishes recorded completion from current snapshot
existence/restore and handles unknown submission/failure without mutation controls.
ID editing invalidates pending results; disconnect/unmount ignores late replies.
A new connection starts empty but can read server records without prepared consent.
No client state persistence, approval restoration, write or automatic read occurs.

Verification: npm --prefix apps/client run build passed TypeScript/Vite.
node experiments/instances-browser/check.mjs --snapshot-recovery passed the focused
Firefox/React simulated-IPC scenario: explicit history/empty/error states, row/
saved-ID reads, saved/current wording, escaped description, unknown/failed states,
wrong-target refusal, stale edited-ID detail and disconnected history-error
suppression, fresh-connection recovery and zero creation commands. JS syntax and
git diff checks passed. Rust/server/native/Android suites were not repeated because
those sources did not change. No new dependency or real VM operation was used.

Limits: browser UI plus earlier native HTTPS/server evidence, not a completed
native recovery WebView/process restart journey. Next task: extend the owned native
fixture to reconnect/restart the app after snapshot creation, recover the saved
operation/history through real commands/server and prove no additional provider
POST; then continue snapshot restore with explicit destructive confirmation and
durable task tracking. Real-provider gates and the full roadmap remain open.

## Actual native snapshot recovery after app/server restart — 2026-10-20

Previous goal turn made progress: client read-only recovery built and passed the
focused Firefox scenario. This slice adds a --snapshot-recovery combined mode to
the owned native fixture and registers production recovery commands in the example.
It creates once through the real native UI, closes that application, saves the
operation, stops the actual server cleanly, then launches new server/app processes
on the same database. The new server explicitly disables snapshots and removes
all generated Proxmox configuration. New native connection uses the supplied
fixture HTTPS/token/CA; old token/operation ID/creation form are not restored.

Acceptance met: history lookup, copied saved-ID read and refresh show the correct
recorded target/request/task metadata and historical/current distinction, without
submission controls. Recovery proxy traffic is GET-only; provider read/POST counts
stay unchanged and the full succeeded operation equals its pre-restart value.
The creation phase still proves exactly one disk-only POST, independent task
polling and one succeeded journal entry. All infrastructure/certificates/tokens
are generated and disposable; no personal credentials or real VM are accessed.

Verification: targeted embedded-asset native example and actual server binary
builds passed. The combined native scenario passed on its first run; log:
.lab/native-deps/snapshot-recovery-ui-check.log. JS/Python syntax, targeted Rust
formatting and git diff checks pass. No product source, dependencies or schema
changed; unrelated frontend/unit/Android checks were not rerun. Owned display,
server/app process groups, proxy/provider and temporary data were cleaned up.
TLS, sandbox and X11 cookie verification stay enabled; minimal-session portal/
accessibility warnings remain without preventing the assertions.

Limits: succeeded saved operation after clean app/server restart only, in the
isolated Linux fixture. A live provider crash, physical/mobile/installed runtime,
current snapshot existence and actual restore are not proven here. Next task:
read-only restore preconditions and explicit destructive approval bound to the
selected complete disk-only snapshot and retained target, followed by durable
no-replay rollback task tracking and generated-provider/native restore checks.
Real-provider snapshot/restore and the full roadmap remain open.

## Read-only restore preconditions — 2026-10-20

Previous goal turn made progress: actual native app/server restart recovery
passed with GET-only traffic and unchanged journal. This slice adds bounded
snapshot-restore-preflight API/domain/provider reads. It binds the exact named
snapshot, live target digest and canonical SHA-256 of API-visible saved config,
checks stopped power before/after, metadata/config stability, marker/template/
lock/pending guards, complete disk-only snapshot work and provider snapshot feature
for the selected config. No dedicated snapshot revision digest is exposed by the
upstream config API, so hash canonical JSON locally and never forward the raw
configuration. Added recursive ordering and domain/name separation. ADR 0056
records limits, including RAM snapshots that may auto-start during rollback.

The report is explicitly destructive, startsVm=false and executionEnabled=false,
even with creation/lifecycle flags enabled. There is no rollback/power transport,
operation or approval in this slice. Shared identity flag decoding and snapshot
name grammar were extracted without changing existing contracts; current VM
digest still requires the provider's 40-character format. No schema/dependency/
client change or real provider operation was made.

Verification: six filtered snapshot_restore_preflight tests passed against owned
generated HTTPS/API fixtures, including fixed 12-GET positive sequence, stable
canonical fingerprint, 16 blocked conditions plus malformed RAM data, exact
retained target, independent disabled execution, invalid/scope/permission failures
and raw-data suppression. The initial new test build had a temporary borrow in
fixture setup; binding the scope string fixed it before running. Because identity/
name helpers are shared, reran five affected lifecycle_preflight tests and the
existing name/input-validation test; all six passed. Targeted formatting and
git diff checks pass. Unrelated storage/native/browser/Android suites were not run.

Next task: full destructive restore approval (including data-loss acknowledgement,
live digest and selected-config fingerprint), shared mutation exclusion and durable
no-replay rollback task storage. Then fixed rollback transport/executor, explicit
client confirmation and generated/native restore verification. Stopped/disk-only
preflight does not prove storage rollback, write permissions, application
consistency or recovered guest contents; real infrastructure acceptance stays open.


## Durable destructive restore journal — 2026-10-20

Previous goal turn made progress: six restore-preflight HTTPS/API tests and six
shared identity/name checks passed. This slice adds mandatory confirmation plus
explicit data-loss acknowledgement bound to the full target, current digest,
selected snapshot name and canonical saved-config hash. Unknown/start fields are
rejected. Restore has its own input-hash domain and operation kind. Same-key
retries keep the original identity/age; dispatch needs exact fresh approval and
an irreversible marker. Only qmrollback tasks with matching node/VMID can be
retained. Unknown submissions never reclaim; known-task recovery cannot dispatch
again. Storage success needs task success, matching snapshot/hash and a stopped
observation of the exact target; this does not prove guest contents.

Schema 12 adds restore to shared mutation exclusion and the incomplete-provisioning
trigger. Migration retains old snapshot/lifecycle/provisioning rows and child
journals, checks the reference graph atomically and restores FK enforcement before
returning a store. The slice adds no dependency, client control, provider transport or runtime
write opt-in. ADR 0057 records contract and limits.

Verification: eleven distinct new restore-storage tests passed, including consent,
idempotency/expiry, concurrent connections, target/snapshot creation exclusion,
crash windows, corrupt-input rollback, task identity, completion guards, actual
schema-11 snapshot/child-journal preservation and atomic migration refusal. The
corruption fixture initially failed to inject its orphan because its connection
correctly enforced foreign keys; explicitly disabled enforcement on that owned
fixture connection before injecting corruption, then the migration refusal test
passed. The eight initial tests passed first; reran only changed/new filters after
adding migration and stronger completion cases.

Because the operation-table schema and active-mutation constraints affect existing
callers, reran 9 lifecycle-storage, 34 provisioning-intent and 10 snapshot-storage
tests; all 53 passed. The affected v2 instance migration and store FK-enforcement
checks also passed. Latest-schema assertions changed from 11 to 12; historical
migration files remain unchanged. Targeted Rust formatting and diff checks pass.
No application/native/browser/Android or real-provider suites were repeated.

Next task: fixed one-shot disk-only rollback transport and an opt-in executor that
revalidates the reviewed target/snapshot before dispatch, retains the task before
polling and reconciles known tasks through GET only. Then HTTP/client explicit
confirmation and generated/native checks. Restore preflight remains disabled;
no actual rollback, guest-content recovery or real-infrastructure acceptance is
claimed. Full roadmap remains open.


## Opt-in durable rollback executor — 2026-10-20

Previous goal turn made progress: restore approval/storage, mutation exclusion and
schema-12 migration passed 11 new and 55 affected existing checks. This slice
connects fixed one-shot rollback transport and the durable restore executor to the
sequential production worker. It revalidates all stopped/disk-only target/snapshot
checks plus the confirmed live digest and saved-config hash before committing the
marker. POST has only start=0 at the fixed retained snapshot path. The matching
qmrollback task is persisted before polling. Task success then requires fresh full
preflight, original saved hash and stopped owned target; the pre-write live digest
is not required after rollback because rollback changes configuration. The final
power observation is included in the report/result. No raw saved config escapes.

VMANYWARE_RESTORES_ENABLED is a separate exact true/false flag, default false.
Existing creation/lifecycle/provisioning opt-ins do not grant restore. Invalid
configuration stops startup before database creation. Known tasks reconcile GET
only with all write flags off; lost/invalid submission responses stay uncertain
and never reclaim. No restore HTTP submission or client controls exist yet;
HTTP preflight stays execution-disabled until that submission route is added.
Updated ADR 0057 and snapshots documentation, preserving the open roadmap gates.

Verification: seven new filtered restore_execution tests passed against generated
verified HTTPS. Positive worker journey has exactly 26 requests: 12 preflight
GETs, one fixed POST, one task GET and 12 postcheck GETs. Covers 17 pre-dispatch
conditions, four lost/wrong node/VMID/kind response cases across reopen, 13-GET
known-task restart with writes off, 14 failed task/poll/postcheck conditions,
invalid names and default queue behavior. The final transport filter separately
checks 401/403/302/500/malformed responses, private error suppression and one POST
without redirects/retries. No prior passing executor filters were repeated after
adding this independent test.

One new exact opt-in unit test and two new actual-server process tests passed.
The two affected existing snapshot process tests and one existing restore
preflight positive test also passed. Targeted formatting and diff review pass.
No dependency/schema/storage implementation changed in this slice; earlier dirty
changes and unrelated native/browser/Android/storage suites were not repeated.
Owned fixtures/processes were cleaned up. No real VM, credentials or production
infrastructure were accessed or changed.

Next task: confirmed restore HTTP submission bound to path/target/snapshot and
one idempotency key, independent capability reporting and scoped safe responses;
then native retained review/data-loss confirmation/status and actual generated
native restore journey. Provider completion here does not prove restored guest
contents, write permissions on real storage or phase-0D/phase-4 acceptance. Real
modify/restore/readback and the rest of the roadmap remain open.


## Confirmed restore HTTP submission — 2026-10-20

Previous goal turn made progress: fixed rollback/executor and opt-in production
worker passed 13 targeted checks. This slice exposes POST environments/{id}/
snapshots/{snapshotName}/restore with a 4 KiB approval limit, both path identities
bound to the complete confirmed target/request and mandatory data-loss agreement.
Exactly one idempotency key is required; Origin-bearing requests are rejected.
The handler checks the independent restore capability, provider scope and saved
instance, then only enqueues. Same-key retries retain identity/age and return
terminal/uncertain records without replay. Location points to operation GET.
Static errors/diagnostic route templates omit selected names and raw request data.

The actual production router now receives the same restore flag as its worker.
Old router constructors default false; creation/lifecycle/provisioning flags do
not grant restore. Preflight reports this capability independently of eligibility;
true never bypasses stopped/disk-only checks. No schema/dependency/native/client
change, new runtime flag or real provider operation was introduced in this slice.
Earlier dirty source changes remain intact. ADR 0057/API/snapshots docs updated;
roadmap acceptance remains open.

Verification: eight new restore_api tests passed: exact approval/path binding,
mandatory confirmations/fingerprint, unknown options/malformed/oversized bodies,
Origin/key policy, independent capability, scope/target guards, saved operation
lookup, concurrent idempotency, changed-input and mutation conflict, bearer auth
and safe errors. The first test compilation found two incorrect format arguments
in the new adapted fixture; corrected before running. Two new filtered restore_http
verified-HTTPS tests passed: HTTP enqueue -> production round -> saved result for
success, stale live digest, stale selected hash and lost submission reply; same-key
terminal/uncertain retries do not write again, and query retargeting is ignored.
Preflight capability is independently checked with both eligible and failed checks,
exact 12 GETs, no journal/write, and true/false flag states.

Because shared router construction and production flag forwarding changed, reran
seven affected existing snapshot API tests, the existing restore-preflight target/
creation-flag test, and the actual restore process test enhanced to send a real
HTTP request. All nine pass (19 distinct checks in this slice). Actual server
returns disabled 403 or configured-provider-absent 503 as expected; generated
processes clean up. Targeted formatting/diff checks pass. Earlier executor/storage,
unrelated UI/native/Android and broad suites were not repeated.

Next task: native retained restore prepare/confirm/read/ack commands that bind the
reviewed snapshot/configuration, require separate data-loss consent, preserve an
unknown same-key retry, reject stale/disconnected replies and never replay a known
operation. Then explicit React restore review/status and generated/native journey.
Provider completion is not real guest-content proof; no actual rollback or phase
0D/phase-4 infrastructure acceptance is claimed. Full roadmap remains open.


## Native retained destructive restore commands — 2026-10-20

Previous goal turn made progress: confirmed restore HTTP and independent capability
passed 19 targeted checks. This slice registers native prepare/confirm/read/clear
restore commands and retains one restore review/key in the current connection.
Prepare reads the saved target then fixed preflight, binds target/request/revisions,
requires all 13 distinct check codes and validates destructive/stopped/disk-only
report consistency. Missing-snapshot failure can be reviewed but not submitted.
Confirm requires separate displayed-target and data-loss booleans; full approval
is constructed from the native retained review, never renderer revisions or URL.

Submitted intent is retained before network await. Unknown response only permits
same-key/body retry; known operation blocks POST and uses GET. Success validates
exact target/name/hash plus task-backed stopped observation. Inconsistent definite/
uncertain status combinations fail validation; uncertain state cannot be cleared.
Only unsubmitted or verified terminal intent can be acknowledged. Generation
changes clear local restore memory and reject late prepare/submission/status
replies without server cancellation. No persisted client consent or credential,
React controls, server/schema/dependency changes or real VM operation were added.

Reuse the creation name grammar and bounded authenticated snapshot HTTPS helper;
changed its denial text to generic snapshot action instead of creation. Added
Clone to normalized snapshot metadata for retained restore review. Existing
creation/recovery behavior remains otherwise unchanged. ADR 0057 and snapshot
workflow docs describe the native boundary and remaining gates.

Verification: actual Tauri/native command registration compiles using the signed
.lab/native-deps prefix. Fourteen filtered remote_snapshot_restore tests pass with
generated verified HTTPS: full prepare/dual-confirm/read/ack request sequence,
identical unknown-response retry, disabled/failed reports, 15 malformed target/
selection/check/power/fingerprint variants, exact task-backed success, status
uncertainty, auth/status/malformed/size limits, in-flight disconnect, stale prepare,
actual socket loss, missing-snapshot review, known uncertain GET-only completion
and late disconnected status refusal. Initial ten tests passed; strengthening the
shared new restore validator justified rerunning this slice's fourteen after
adding uncertainty cases. One initial unused test-helper warning was resolved by
asserting reset behavior in the disconnect test. Import cleanup compiled with the
subsequent checks, without warnings.

Because the existing HTTPS helper denial text is shared, reran only the affected
creation happy path and authentication/status/size response test; both pass.
Targeted Rust formatting and diff checks pass. No server/storage/browser/Android
or unrelated native suite was repeated. No real provider or infrastructure contact.
Next task: explicit React restore review showing VM/snapshot and current-data loss,
independent unchecked consent, same-key unknown retry and GET-only status, with
stale-reply suppression; then actual generated native restore journey. Native
command evidence alone does not prove WebView/mobile behavior or restored guest
contents. Full roadmap and real-provider gates remain open.


## Explicit remote client restore review — 2026-10-20

Previous goal turn made progress: native retained restore commands compiled and
passed 14 new plus two affected HTTPS tests. This UI slice adds restore selection
from the exact named snapshot row and a single retained review panel. Selection
makes no request until explicit review. The panel displays VM/name/node/VMID and
explains current-data/settings loss, stopped/disk-only requirements, power limits
and snapshot dependence on original storage. It binds reviewed target/name and
requires the full 13-check passed report, exact fingerprint, stopped observation
and disk-only selection before enabling consent.

Target confirmation and loss acknowledgement are separate initially unchecked
checkboxes. Both must be checked. Confirm sends only native connection/key/booleans;
revisions and approval remain native. Unknown response retains the same key/consent
and only offers same-request retry; known operation exposes GET refresh only.
Success evidence must match target/name/hash/task/stopped observation, with explicit
unverified guest-content wording. Uncertain work cannot be acknowledged. A parent
selection/lock prevents switching VM/snapshot or snapshot creation while retained.
Disconnect resets selection/consent and generation checks discard late replies.
No persistence, dependency, Rust/server/schema or real VM mutation was introduced.

Verification: npm --prefix apps/client run build passed TypeScript/Vite.
The new focused Firefox --snapshot-restore scenario passed first run using actual
React with simulated IPC. Covers no automatic request on list/selection, disabled/
running/mismatched review refusal, escaped description, both independent unchecked
consents, target/creation exclusion, same-key retry after response loss, queued/
running/uncertain GET refresh, invalid success fingerprint refusal, terminal ack,
fresh connection and late prepare/status suppression. It checks confirm arguments
cannot introduce URL/revision/hash fields and no creation command is invoked.
Because shared environment/list controls changed, reran the existing focused
--snapshot-create and --snapshots browser scenarios; both pass. JS syntax and
repository diff checks pass. Earlier native/server/storage/Android and unrelated
frontend suites were not repeated without source changes.

Updated snapshot workflow/ADR/browser-fixture docs; existing dirty work remains
intact. Next task: extend the isolated native fixture to exercise the actual
WebView -> retained native restore commands -> authenticated API/worker -> generated
HTTPS provider rollback, proving one fixed POST, independent completion evidence
and no replay. Browser/simulated IPC does not prove this native journey, deployed
or Android behavior, real storage rollback or restored guest contents. Real
phase-0D/phase-4 acceptance and full roadmap remain open.


## Actual native restore with lost HTTP submission response — 2026-10-20

Previous goal turn made progress: explicit restore UI built and focused Firefox
restore/creation/list scenarios passed. This slice adds a distinct mutually
exclusive --snapshot-restore native fixture mode and registers production restore
commands in the owned example. Actual WebView, native retained HTTPS state,
authenticated API/SQLite/worker and generated provider all participate; no IPC
mock or direct fixture submission bypass is used. Normal application startup does
not link fixture drivers. No product source, dependency or schema changed.

The driver selects the existing named disk-only fixture snapshot. First read-only
review observes a running VM and cannot consent; subsequent synthetic reads show
it stopped without power mutations. It cancels an eligible review, prepares again
with two unchecked consents, requires each independently and confirms. The HTTPS
proxy drops the first response only after real API 202/commit. Native retains the
key/review; exact retry returns the same operation. Provider receives exactly one
fixed rollback POST with start=0; creation/lifecycle flags are absent. Status/list,
terminal ack and disconnect work, without a new provider write.

Independent assertions pass: exactly two identical API path/key/body submissions;
one succeeded vm_snapshot_restore journal entry with known successful task; exact
instance/scope/node/VMID/name; canonical saved-config fingerprint independently
computed in Python; stopped observation; unchanged instance association. Counts:
12 snapshot-list GETs, 10 saved-config GETs, 10 power GETs and at least one task
poll. Live digest changes after rollback, while selected saved-config hash stays
stable. This is provider completion evidence, not restored guest-file contents.

Verification: current embedded-asset native example and actual server builds
passed. Initial scenario failed in its new driver before connecting because the
CA textarea was filled with an input-only setter; corrected element dispatch,
rebuilt only the affected example and reran the same scenario. It passed with all
independent assertions. Final log .lab/native-deps/snapshot-restore-ui-check.log;
earlier driver failure .lab/native-deps/snapshot-restore-ui-check.initial.log.
Python/JS syntax, targeted Rust formatting and diff review pass. Existing unit/
UI/Android suites were not repeated without product changes. TLS, sandbox and X11
cookie checks remain enabled; minimal-session portal/accessibility warnings remain
nonfatal. Owned processes, listeners and temporary state were cleaned up.

Next task: scoped read-only restore operation history/detail with recorded target/
request, so reconnect/restart recovery never submits a rollback or recreates
consent; then native/client saved-ID recovery and actual app/server restart proof.
No real VM/provider contact, mutation, credentials or production change occurred.
Real storage rollback, recovered guest state, physical/mobile/packaged runtime and
phase-0D/phase-4 infrastructure gates remain open. Full roadmap remains active.


## Scoped read-only saved restore recovery — 2026-10-20

Previous goal turn made progress: actual isolated native restore passed with lost
HTTP response, identical API retry and one provider rollback POST. This slice adds
GET-only per-instance restore operation history/detail. History filters instance/
kind before newest-100 ordering/limit and omits request/result bodies. Detail
returns original target/request/config fingerprint and dispatch marker, without
approval/grant flags, live digest, request key, confirmation age, input hash, raw
task ID or saved configuration. The existing immutable target DTO is reused.

Reads use consistent SQLite transactions, resolving scope/kind before decoding
other records. Validate saved approval/hash and qmrollback task, dispatch/status/
failure consistency and exact task-backed successful target/name/fingerprint/
stopped observation evidence. Normalize success through typed public evidence to
strip unknown nested fields. Refuse corrupt/inconsistent success or unrelated
workflow failures without changing records or disclosing their raw data. No
provider, preflight, claim, consent renewal, state recovery, write capability or
new submission is needed. Configured bearer protection remains common.

Verification: eleven distinct focused restore_recovery HTTP/SQLite tests passed.
Initial seven passed, then added/strengthened only affected filters for known
running/uncertain/interrupted/auth-refusal/task-failure states, wrong task/dispatch
combinations, same-instance creation isolation, saved success after reopen,
unknown nested field suppression and six mismatched completion fields. The final
failure-code whitelist change was checked with its new corruption test and the
two affected failed-record history/isolation tests, without repeating unrelated
passing filters. Coverage also includes queued reopen with no provider/flags,
unchanged journal, instance/kind/ID guards before corruption errors, independent
100-row history limit/order and bearer auth. All tests use owned SQLite; no real
infrastructure or credential files are accessed.

Targeted Rust formatting/diff checks pass. No dependency/schema, existing executor
or creation behavior, runtime configuration, native/frontend code changed; earlier
passing storage/native/browser/Android and broad suites were not rerun. This proves
read-only API/database reopen, not a new application/server-process restart test.
Updated API/snapshot/ADR docs and left roadmap gates open.

Next task: native fixed authenticated GET restore-history/detail commands with
recorded target/request/fingerprint validation, bounded private responses, current
connection checks and no prepared intent/submission; then explicit React saved-ID/
history recovery and actual app/server restart verification. Read completion is
historical provider evidence, not current VM state or real guest-content recovery.
Full roadmap and real phase-0D/phase-4 acceptance remain open.


## Native saved restore history/detail recovery — 2026-10-20

Previous goal turn made progress: eleven scoped restore-history/detail API/SQLite
tests passed. This slice registers native read_remote_restore_history and
recover_remote_restore commands. Canonical IDs select fixed GET paths through
the retained verified HTTPS/token profile, with bearer marker, exact status and
response size limits. History validates scope/count/unique IDs/names/timestamps
and status/failure/task consistency. Detail validates requested identities,
recorded target/request/config fingerprint, submission marker and exact task-backed
successful stopped observation. Unknown nested fields and raw error bodies are
not exposed. Read failures use static read-oriented text.

Connection generation is checked after await; disconnect/reconnect rejects late
reads. Recovery creates no review, grant, key, retry state or submission. Share
existing restore request/operation validation for recorded context; added a
dispatch-consistency check and tightened pre-dispatch failure codes to the server's
restore vocabulary. Historical success remains provider evidence, not current
power or guest recovery. No server/schema/dependency or frontend change occurred.
Earlier dirty changes remain intact.

Verification: actual Tauri/native command registration compiles with the isolated
.lab/native-deps prefix. Nine focused remote_restore_recovery tests pass with
generated verified HTTPS: two fixed authenticated GETs without prepared intent;
queued/running/unknown/terminal and known-task states; exact target/request/hash/
power/dispatch checks; history bounds/uniqueness; read auth/status/malformed/size/
private errors; nested unknown field removal; invalid IDs before network; old
connection reply suppression. Initial seven passed, then reran this slice after
extending restore-specific guards and adding two status/known-task checks.

The shared existing restore operation validator changed, so reran only its status
validation and retained prepare/confirm/status happy path; both pass. Targeted Rust
formatting and diff checks pass. No server/storage/React/browser/Android or unrelated
native suite was repeated. No real provider or credential files were accessed.

Next task: per-instance React saved-restore history/manual-ID/read-only refresh,
with recorded/current wording, target binding, stale ID/disconnect suppression and
no consent/submission controls; then actual app/server restart recovery fixture.
Native command evidence alone does not prove WebView/mobile/restart integration
or recovered guest contents. Full roadmap and real-provider gates remain open.


## Client saved restore history and manual-ID recovery — 2026-10-20

Previous goal turn made progress: native GET-only saved restore recovery compiled
and passed nine new plus two affected HTTPS tests. This UI slice adds a separate
per-instance saved-restore panel below creation recovery. Explicit history, row
inspection, manual saved ID and read-only refresh work without snapshot listing
or prepared consent. Detail binds selected instance/scope/node/VMID and operation
ID/kind; success also needs matching saved fingerprint/name/task/submission and
stopped target evidence. Saved completion is explicitly distinct from current VM,
snapshot existence or guest-file recovery. Uncertain/failed outcomes remain clear.

ID edits invalidate in-flight results and reset detail/error/busy state; unmount/
disconnect suppress stale responses or failures. Fresh connection starts empty but
can read the server record. Recovery exposes no consent, prepare, submit, retry
write, cancel or acknowledge action. Existing creation history remains separate.
No native/server/schema/dependency or real provider changes occurred in this slice;
earlier dirty work remains intact.

Verification: TypeScript/Vite build passed. New focused Firefox --restore-recovery
passed, then was extended with wrong-success fingerprint refusal and rerun after
small restore wording corrections; passes all cases. Actual React/simulated IPC
covers no automatic reads, history empty/error distinction, row/manual-ID lookup,
recorded/current/guest-state wording, unknown/failed states, wrong target/hash,
stale edited-ID detail, late disconnected history error and fresh-connection
lookup. Assertions prohibit any creation or restore mutation command. The shared
snapshot area now mounts another panel, so ran only the affected existing
--snapshot-recovery scenario too; it passes. JS syntax and diff review pass.
Earlier Rust/storage/native/Android or unrelated UI suites were not repeated.

Next task: actual isolated native restore followed by clean app/server restart,
provider configuration removed and write flags off, recovering history/saved ID
through real native commands with GET-only traffic and unchanged operation. This
UI/browser evidence does not prove that integration or restored real guest state.
Full roadmap and phase-0D/phase-4 infrastructure acceptance remain open.


## Actual native saved restore recovery after app/server restart — 2026-10-20

Previous goal turn made progress: client saved-restore lookup built and focused
restore/creation recovery browser checks passed. This slice adds mutually exclusive
--restore-recovery combined mode to the owned native fixture, registers production
GET history/detail commands in the example and adds a second-process recovery
driver. The initial actual native restore retains running-target refusal, cancelled
review, dual consent and same-key API retry after a deliberately lost response.
Exactly one rollback POST and independent completion evidence remain required.

The first app exits. Runner saves the succeeded operation, stops the actual server
cleanly, then starts a new server process on the same SQLite database with all four
write flags explicitly false and all Proxmox settings removed. A second native
app process starts empty and receives the generated connection explicitly. It
finds history/row detail, reads the copied saved ID and refreshes it through actual
commands/API. Old token/input/restore intent/consent are not restored automatically.

Acceptance passed: new recovery proxy traffic is GET only; provider read/POST
counts stay unchanged; the whole succeeded operation equals its saved value.
The initial restore still proves two identical API path/key/body submissions,
one provider rollback, exact selected-config fingerprint and stopped observation.
No product source, dependency/schema, real VM or credential change was needed.

Verification: targeted embedded native example and current server builds pass.
The combined actual native scenario passes on first run; log
.lab/native-deps/restore-recovery-ui-check.log. Python/JS syntax, targeted Rust
formatting and diff checks pass. Unrelated unit/browser/Android suites were not
repeated without changes. Owned listeners, process groups and temporary data were
cleaned up. TLS, sandbox and X11 cookie checks remained enabled; nonfatal minimal-
session portal/accessibility warnings remain.

Limits: clean succeeded-record recovery only, isolated Linux/generated provider.
No live rollback crash, real disk/guest restore, deployed/mobile/installed runtime
or phase-0D/phase-4 infrastructure acceptance is proven. Next task: read-only
retained-instance deletion prerequisites with ownership/power/lock/pending guards
and exact disk/deletion scope for later explicit delete/recreate approval. Roadmap
still requires delete/recreate, real-provider acceptance, remaining platform work
and security/packaging/release gates; full goal remains active.


## Read-only deletion target and candidate volume review — 2026-10-20

Previous goal turn made progress: actual native app/server restore recovery passed
with GET-only traffic/provider absent/writes off. This slice adds GET delete-preflight
for a retained instance. It reuses unchanged lifecycle guards and adds stopped
power, protection, live/unused/Cloud-Init/EFI/TPM and saved snapshot/RAM references,
storage-inventory owner VMID, canonical two-read scope stability and final target/
pending/power checks. Candidate volumes are deduplicated with source references;
recognized ISO CD-ROM media is separate. No raw configs/private inventory fields or
host paths escape. Current definition/association/journal remain unchanged.

Overall result remains unknown if all implemented checks pass: HA/replication
management has not yet been read. Existing flags never enable deletion. No delete,
power, purge, approval or new operation transport is added. ADR 0058 records this
explicit preparation boundary, primary upstream contracts and the need to review
backend cleanup effects/independent volume removal before execution. API-visible
scope is not full deletion authorization. Bounds: 20s, 64 snapshots, 256 images,
1024 inventory entries/storage and existing provider response limits.

Verification: five distinct targeted deletion_preflight generated-HTTPS/API tests
pass, including five owned candidate types and separate ISO, 15-GET positive
sequence, fourteen failed conditions, duplicate inventory/scope rejection, safe
errors, retained association/definition, zero journal and off capability despite
all existing flags on. The last error-fixture test initially failed compilation
on a temporary scope borrow; bound the owned scope string and its filtered rerun
passed. Earlier four passing filters were not repeated without related changes.
Rust formatting/diff checks pass; existing lifecycle/snapshot helpers are unchanged,
so unrelated storage/native/browser/Android and broad suites were not rerun.
No dependency/schema or real provider/VM/credential change occurred.

Next task: read-only HA/replication dependency checks and complete provider-effect
scope, then explicit data-loss deletion approval with durable mutation exclusion,
no-replay dispatch/task/independent absence checks and retained definition/history
for recreation. Actual deletion/recreation and real infrastructure acceptance remain
open; full roadmap remains active.


## Permission-backed deletion HA/replication review — 2026-10-20

Previous goal turn made progress: five read-only target/volume deletion checks
passed. This slice reads HA/replication membership before and after candidate scope,
blocks configured resources/jobs even if disabled or removal-pending, rejects
membership changes and binds normalized target membership into scopeHash. Report
adds target haManaged/replicationJobs without exposing unrelated cluster settings.
No permission, HA or replication configuration is changed.

Verified upstream contracts: HA index requires Sys.Audit on /; replication index
silently filters jobs by VM.Audit on the guest. Therefore require effective target
VM.Audit before/after each replication list. Permission values describe propagation:
0/false with a present key still grants audit. Missing/revoked/malformed visibility
is an error, not empty success. Target path is fixed from retained VMID; no userid
or token override. Validate canonical HA IDs and replication id/guest consistency,
duplicate IDs and 4096-entry bounds. Unknown fields and unrelated entries remain
private. Execution remains off and overall status unknown while remaining provider
cleanup effects are not fully reviewed; this does not grant deletion authorization.

Verification: all five affected deletion_preflight filters passed with the new
23-GET sequence. Three new generated-HTTPS management tests and one full-preflight
management blocker test passed: exact GETs, nonpropagated audit, missing/wrong-path/
revoked/malformed permissions, forbidden read, malformed/duplicate/oversized HA/jobs,
canonical numeric/string guests, disabled/pending jobs, newly appearing dependency
and private data suppression. Nine distinct checks passed. Targeted formatting and
diff review pass. No schema/dependency/native/frontend or existing non-deletion
behavior changed; unrelated suites were not repeated. No real VM/provider access,
credential file inspection, permission or production change occurred.

Next task: complete provider cleanup-effect/candidate scope review (including
unreferenced owned/fleecing artifacts and non-disk VM-specific cleanup), then bind
that explicit scope into durable destructive approval. Definition/association/
history retention, no-replay delete and independent target/volume absence evidence
must precede recreation. Actual deletion/recreation and real infrastructure gates
remain open; full roadmap remains active.


## Fixed deletion effects and recorded cleanup scope — 2026-10-20

Previous goal turn made progress: nine target/volume/permission-backed management
checks passed. This slice binds a fixed versioned deletion effect contract:
configuration/snapshot metadata, VM ACL/firewall, IPAM and recorded fleecing cleanup;
no job purge or bulk unreferenced-disk destruction, and definition/history retained.
Parse current special-section fleecing references and prove owner VMID before any
future write. Unknown sections or malformed/foreign cleanup block. Preserve
non-Cloud-Init CD-ROM media and report extra owned images observed on inspected
storages, without claiming global orphan discovery. Candidate/preserved images
are jointly bounded to 256. Scope v2 hashes effects and preservation/config evidence.

Upstream destroy does not promise removal of every snapshot-only disk reference;
classify it explicitly and block this fixed delete path unless a live/unused/RAM
reference covers its destruction. Moved the earlier positive old-snapshot-only
disk into an explicit refusal case; supported positive uses the live disk shared
by its snapshot. Cloud-Init filename recognition is only drive kind; storage
inventory still proves ownership independently. Fully checked read preconditions
can now pass, while execution is always false and no deletion consent, journal,
transport, power action or provider mutation exists.

Verification: six affected deletion_preflight tests passed with updated candidate/
preservation expectations. Four new effect-scope test groups pass: valid fleecing,
foreign/unsafe/changing cleanup, unknown sections, snapshot-only refusal, media
preservation, unsupported slot and aggregate bounds. After adding the aggregate
bound, reran only its new filter and the affected positive review. Ten distinct
checks passed. Formatting/diff checks pass. No schema/dependency/native/frontend
or non-deletion behavior changed, so unrelated suites were not repeated. Primary
upstream effect/config/drive contracts are linked in ADR 0058. No real infrastructure,
credential inspection/change or VM mutation occurred.

Next task: complete-target/current-digest/scope-hash deletion approval with explicit
loss acknowledgement and durable shared mutation exclusion/no-replay journal;
then fixed DELETE/task tracking and independently verified VM/volume absence while
retaining definition/history for recreation. Real storage cleanup, backend warnings,
platform/deployment/release gates and full roadmap remain open.


## Full-scope deletion approval and no-replay journal — 2026-10-20

Previous goal turn made progress: ten deletion-effect/preflight checks passed.
This slice retains complete target/digest/scope hash and normalized public plan
(volumes/effects/references, snapshots, preserved media/images and fixed effects)
in mandatory dual-confirmed deletion approval. The 64 KiB shape validator rejects
unsafe/unverified/ambiguous/duplicate/overlapping scope or changed backend effects.
A real generated-provider preflight plan is directly checked as valid approval.
Input hash has its own deletion domain; same-key identity/age never renews.

Schema 13 adds vm_delete to common mutation exclusion and retains existing operation
and provisioning child graphs. Active/uncertain provisioning blocks; failed
provisioning is retained but does not prevent explicit cleanup. Migration is atomic,
checks FKs before commit and restores enforcement before exposing the store. Exact
fresh approval/claim precede the irreversible dispatch marker; matching qmdestroy
task is retained once. Unknown submission never reclaims; known tasks only reconcile
with backoff and cannot dispatch again. A failed destroy task remains uncertain
and holds target exclusion because partial deletion is possible.

There is deliberately no success/retirement transition before independent absence
proof is implemented; generic operation completion cannot bypass the guard. Current
association, definition and history remain intact. No HTTP submission, runtime
capability/worker, provider DELETE or client controls are connected yet. ADR 0059
records the plan, failed-provision cleanup exception and partial-failure semantics.
No dependency or real provider/VM/credential change was made.

Verification: eight new deletion_storage tests passed, covering mandatory consent/
full-scope identity, mutated keys, lifetime/clock rollback, dispatch once, mutation
exclusion, reopen/crash windows, no-task uncertainty, known task/no redispatch,
partial task failure retention, concurrent identity/corrupt claim rollback, actual
schema-12 restore/task/child-graph preservation and atomic orphan refusal. Five
initial tests passed first; subsequent filtered migration/concurrency tests passed.

The operation-table rebuild affects existing callers: all 64 targeted lifecycle
(9), provisioning-intent (34), snapshot-storage (10) and restore-storage (11) tests
passed. Affected instance v2 migration and fresh/migrated/reopened FK enforcement
checks passed, including v12/v13. The affected generated-provider public-plan
preflight check passed. Total: eight new plus 67 affected checks, no unrelated
native/browser/Android or full suite. Latest-version assertions changed to 13;
historical migration files are unchanged. Formatting and diff checks pass.

Next task: provider VM/volume absence observation with permission/identity-safe
semantics and exact plan coverage, then atomic completion/retained-association
retirement without deleting definition/history. Follow with fixed one-shot DELETE,
opt-in worker, HTTP/native/UI confirmation and recreation from retained inputs.
Actual deletion, partial-cleanup reconciliation and real infrastructure/platform/
release acceptance remain open; full roadmap remains active.


## Independent deletion absence observation — 2026-10-20

Added a GET-only observer bound to validated retained approval/provider scope.
Explicit cluster-wide VMID availability must return the exact integer before and
after storage reads. Each candidate storage requires effective Datastore.Allocate
before/after its unfiltered-by-VMID image list, because Audit alone can hide images
after VM ACL removal. Permission values are propagation flags, including false/0.
No permissions are changed. Remaining candidate IDs are retained even when owner
VMID changes. Errors never mean absence; malformed/duplicate/wrong-storage entries,
4096-entry inventory bounds and an overall 20-second deadline fail closed.

Verification: three targeted generated-HTTPS tests pass, covering exact five-GET
positive/partial-cleanup traffic, approval hash, owner changes, missing/Audit-only/
revoked/malformed grants, unsafe/duplicate/wrong-storage/oversized inventories,
changed or mistyped VMID, 401/403/404/500, invalid scope and missing consent before
network access. Targeted formatting and diff checks pass. No existing helper,
schema, dependency, native or frontend behavior changed; unrelated tests were not
repeated. Official cluster/content/volume visibility contracts are linked in
docs/deletion.md. No real provider, credential or VM mutation occurred.

Limitations: point-in-time observation only; no preservation/auxiliary-cleanup
proof, success transition, retirement, DELETE, worker or client capability yet.
Next task: atomically retire the retained association after separately verified
task and approved VM/volume absence, preserving definition/history and mutation
exclusion on partial cleanup. Real infrastructure and full roadmap remain open.


## Atomic deletion completion and retained identity retirement — 2026-10-20

Previous goal turn made progress: GET-only VM/candidate-volume absence observation
passed three HTTPS tests. This slice requires a provider-created non-deserializable
completion proof: matching qmdestroy task independently succeeded, then complete
approved VM/candidate-volume absence. Full approval hash/exact linked task and
30-second monotonic freshness prevent rebinding/stale completion. No task status
boolean or caller-supplied empty-volume list can authorize retirement.

Schema 14 retains the whole instance/foreign-key graph and adds deleted status.
An immediate transaction writes succeeded evidence and retires the association
atomically; injected failure rolls both back. YAML/definition/target/history remain.
Deleted identities reject new operations and live target resolution. Registered
scope/VMID uniqueness allows a later new identity on that VMID without letting
old associations read or mutate it. Local/remote native list decoders and both
lists accept/display deleted rows without active VM controls. ADR 0060 records
these consequential schema/status/proof choices.

Verification: four generated-HTTPS/storage completion tests pass (positive, partial/
failed/running task refusal, mismatched approval/task, missing marker, injected
rollback, reopen, retained YAML/history, same-key replay and reused-VMID identity).
The proof expiry/rebinding unit check passes. Actual schema-13 marked-delete upgrade
and atomic orphan refusal pass one new storage test. Shared instance-table rebuild
justified all 82 affected association/lifecycle/provisioning/snapshot/restore/delete
storage tests; all pass, plus the fresh/migrated/reopened FK enforcement check.
Historical migration files are unchanged; latest-version assertions now say 14.

Both client native enums compile; three affected remote workspace HTTPS tests and
one new local HTTP retained-status reader test pass. Frontend build passes. Both
local and remote Firefox simulated-IPC fixtures pass, including retained deleted
rows and no active controls for those identities. These are generated/local
checks, not real infrastructure, mobile or native end-to-end deletion acceptance.
Initial added tests needed ownership/SQLite-type fixes and explicit FK disabling
only to inject a corrupt historical fixture; corrected targeted reruns pass.
Targeted formatting/diff checks pass; unrelated native/browser/Android/full suites
were not repeated. No dependency, credential, real provider/VM or production change.

Limits: absence is point-in-time, not a reservation; preservation/auxiliary cleanup
are not independently certified. No DELETE, runtime capability/worker, HTTP deletion
submission or client deletion confirmation exists yet. Next task: fixed one-shot
DELETE with purge/unreferenced destruction disabled and no-replay executor using
revalidated approval, durable dispatch/task linkage and independent completion.
Then opt-in worker/API/native/UI, recreation and real/platform/release gates remain.
Full roadmap remains active.


## Fixed deletion transport and standalone executor — 2026-10-20

Previous goal turn made progress: schema-14 atomic retirement and retained deleted
identities passed local task/absence/storage/client checks. This slice adds one
fixed retained-target DELETE with query purge=0/destroy-unreferenced-disks=0, empty
body, no skiplock/power/extra-volume operations, verified TLS, bounded response,
redirect refusal and no retry. Returned node/qmdestroy/VMID must match exactly.
Only explicit 401/403 refusal is definite after the durable dispatch marker;
missing/invalid replies remain unknown and non-reclaimable.

The standalone executor requires explicit capability for a fresh dispatch. It
checks retained identity/scope and permission-backed future absence visibility,
then repeats full stopped/protection/pending/lock/owned-volume/snapshot/effect/HA/
replication review. Exact digest and full public plan must match recorded consent;
the original approval expiry is checked before committing the once-only marker.
Known tasks only read, even after writes are disabled. Failed tasks, read failures,
remaining volumes/VM or missing/changed provider stay uncertain and hold exclusion.
A later full absence proof can retire the same association through GET-only recovery.
No runtime flag/worker, HTTP submission or native/client deletion controls are wired.

Verification: server library cargo check passes. Eight new generated-HTTPS tests
pass: exact one DELETE/32-request executor sequence and independent retirement;
18 before-dispatch guards with exact failure codes; unknown/lost/wrong replies
across reopen; known-task restart with dispatch disabled; seven partial/failure
outcomes with retained identity/exclusion and no private error text; later cleanup
reconciliation with the same task and seven GETs; missing/changed provider recovery;
explicit auth/server errors and relative redirect refusal without retry.

One new reconciliation fixture initially omitted its intended remaining-volume
variant and completed immediately; fixed its variant selector, and the targeted
reconciliation rerun now proves uncertainty then GET-only success. The affected
public-preflight positive check passes after extracting its unchanged route builder.
Total nine distinct targeted tests; unrelated storage/native/browser/Android/full
suites were not repeated. Targeted formatting/diff checks pass. No schema/dependency,
credential, production or real provider/VM mutation occurred.

Primary upstream destroy API and HTTP DELETE query parser were reviewed and linked
in ADR 0061. The provider offers no digest compare-and-delete: external actors can
change the target between our review and its mutation. Local operation exclusion
is not an external lock; no atomic infrastructure consent claim or real acceptance
is made. Absence remains point-in-time and auxiliary cleanup/preserved media are
not independently certified.

Next task: independently default-off deletion runtime config/capability and fair
worker integration, with actual isolated server process checks. Follow with exact
HTTP/native/client consent and recovery, recreation, real disposable-target review
and remaining platform/deployment/release gates. Full roadmap remains active.
