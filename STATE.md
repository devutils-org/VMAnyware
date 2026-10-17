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
produced the current optimized `target/release/pocketlab-client` (1m02s Rust
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
  target/debug/pocketlab-client. README documents single-session recovery.
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

Prepare a concrete Debian template plan (dated image/checksum, VMID, storage
and network) for approval on pve-lab, including scoped VM.Audit for its account
and token. The Rust version/node/empty-inventory probe now passes with verified
TLS. No inner guest exists. The current read-only identity expires on
2026-10-18 at 21:22:17 UTC; replacement requires approval. Normal macOS startup
remains deferred to a Mac.

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
  `pocketlab-pve-lab` with the resources/network/disk in `docs/x86-lab.md`.
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
- Started `pocketlab-pve-lab` as UID 1000, PID 171988, with q35/KVM, host CPU,
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
- Proposed hostname: pve-lab.pocketlab.test; address 10.0.2.15/24; gateway
  10.0.2.2; DNS 10.0.2.3; ext4/LVM on the sole 64 GiB virtual disk.
  Existing credential files were never read. VM remains running.

## Installed outer Proxmox lab — 2026-10-17

- User explicitly approved the proposed installation settings and generation
  of the local administrator password. Installed only on `/dev/vda` (the new
  64 GiB qcow2), ext4/LVM. Country France, Europe/Paris, US English keyboard;
  hostname pve-lab.pocketlab.test; nic0 bridged by installer vmbr0 with
  10.0.2.15/24, gateway 10.0.2.2 and DNS 10.0.2.3. Notification address
  root@pocketlab.test is a lab placeholder, not working email delivery.
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
  curl with this explicit CA and hostname pve-lab.pocketlab.test resolved to
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
  passwordless pocketlab-reader@pve, privsep inventory token, no ACLs, both
  expiring in 24 hours. Secret output is exclusive/0600 under ignored .lab.
  Script syntax parsed successfully; it has NOT been executed. Concrete
  credential approval was requested and remains pending.
- `cargo build --locked -p pocketlab-server --example proxmox_probe` passed.
  Bubblewrap read-only hosts-file override resolved pve-lab.pocketlab.test to
  127.0.0.1 without changing host DNS/hosts. No new dependency or production
  code was introduced. Live authenticated inventory remains untested.
- Next: after approval, run the prepared script in the isolated hosts namespace,
  then pass the generated token only in the probe child environment and record
  actual version/node/empty-template evidence. No template or VM operation is
  part of this slice. `git diff --check` passed.

## Live Rust inventory slice — 2026-10-17

- User explicitly approved the prepared account/token operation. Created only
  pocketlab-reader@pve (no password/group/ACL) and privsep inventory token on
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
- `cargo build --locked -p pocketlab-server --example proxmox_probe` passed.
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
