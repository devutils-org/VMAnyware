# PocketLab — current state

Updated: 2026-10-17. Historical evidence and decisions are in Git and docs.

## Implemented

- Tauri 2/React/TypeScript native shell; user confirmed the macOS native bridge.
- Loopback-only Axum service, validated runtime configuration, verified-TLS
  Proxmox reads and safe errors. No provider mutation methods are implemented.
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
  workflow not pushed or executed, Linux success remains unverified.
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
  Its commands passed locally; no GitHub/Linux/native/mobile CI result is claimed.
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
produced the current optimized `target/release/pocketlab-client` (1m02s Rust
release build), confirmed Mach-O arm64, including stall handling/lazy rendering. It was not launched for an integrated SSH walkthrough. The
older renderer smoke uses an isolated WebKit host. The new optimized Cargo
example now proves real Tauri IPC/custom-protocol/CSP integration with disposable
resources; normal app startup/distribution and real guest tests remain open.
Windows shutdown code is source-prepared, not compiled or tested on Windows.
No Proxmox infrastructure has been contacted. Phase 0 remains incomplete.

- Linux preparation: the pinned Debian 12 arm64 container compiled the server
  and passed Clippy, but diagnostics tests failed. The image has a POSIX shell
  kill builtin and no external kill executable; tests now send signals through
  the shell builtin with separately passed arguments. All 50 server tests and
  server formatting/Clippy pass again on macOS. Linux rerun remains pending.
  The opt-in runner checks committed HEAD with an isolated local Docker client.

## Next single task

Check Linux server feasibility using the explicitly local Docker socket and an
isolated client configuration, never personal Docker credentials/remote contexts.
Verify availability of the exact pinned compiler image before any test; record
actual Linux evidence or the external toolchain blocker without claiming CI ran.

## External inputs and limits

- User will install Proxmox on another PC. The current Apple M1 Mac supports
  native virtualization but reports nested virtualization unsupported; no local
  VM was installed or changed.
- Live inventory/preflight needs an explicitly approved test endpoint, trusted
  CA, dedicated token and prepared catalog. Follow `docs/debian-template.md`.
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
