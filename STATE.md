# PocketLab — current state

Updated: 2026-10-16. Historical evidence and decisions are in Git and docs.

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
  SIGTERM request graceful shutdown; startup failures emit fixed stage codes.
- Prepared GitHub Actions server/frontend checks with pinned actions and locks;
  workflow not pushed or executed, Linux success remains unverified.
- Debian Cloud-Init template guide, platform/toolchain evidence matrix and
  interim feasibility report. No real template preparation was executed.
- Isolated opt-in desktop SSH/PTY experiment, outside the root workspace and
  application dependency graph. Generated in-memory keys and localhost fixture
  prove strict host-key checks, signed authentication, real local shell output,
  resize, Ctrl-C foreground interruption and fresh reconnect. See ADR 0008.
- Experimental persistent endpoint-scoped host trust with fingerprint-bound
  explicit confirmation, reopen, changed-key refusal and concurrent update
  protection. Store failures reject trust; no personal SSH files are read.
  Confirmation is an experimental API, not an implemented native dialog.

## Current verification

Environment: macOS arm64, Rust/Cargo 1.99.1, Node 26.11.1, npm 11.20.0,
Apple CommandLineTools. Dependency locks and compiler pin are committed.

- Latest `make check` passed frontend TypeScript/Vite build, workspace formatting,
  all-target Clippy with warnings denied and all 60 tests: 49 server, 11 native.
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
- Latest `make check-ssh` passed isolated formatting, locked Clippy and all ten SSH
  tests on macOS arm64. The real PTY test uses two fresh connections, observes
  stty size changes and requires foreground sleep interruption/status 130 within
  two seconds. Fixture cleanup leaves no owned shell/sleep processes. An early
  development timeout remains undiagnosed and is documented honestly; repeated
  final checks passed. The experiment has no existing credentials or config.
- Trust tests cover exact confirmation/reopen/authentication, rejection before
  authentication, changed keys, port/DNS/IP scope, corrupted/missing/insecure
  data, bounded 250 ms locking, stale/concurrent claims and endpoint validation
  after deserialization. Generated public keys persist only in temporary stores.
  Only the isolated crate/docs changed; application tests were not rerun.
- The running local service was rebuilt/restarted with the current code; a real
  validation POST returned HTTP 200, canonical hash and generated request ID.
  Current UI features compile; their manual native walkthrough remains pending.
- Template guide: seven shell blocks passed syntax checks without execution;
  links/catalog sample reviewed; official dated image/manifest/build metadata
  exist. No cloud image was downloaded and no VM operation was executed.

Earlier runtime evidence: `make doctor`, actual local HTTP/SQLite restart smokes,
`make dev` native launch and user-confirmed `Native bridge connected (macos)`.
`tauri build -- --no-bundle` produced an optimized binary before the latest
features; current native frontend changes compile but await manual walkthrough.
Windows shutdown code is source-prepared, not compiled or tested on Windows.
No Proxmox infrastructure has been contacted. Phase 0 remains incomplete.

## Next single task

Prepare native host-key inspection/confirmation through Rust-owned, expiring
IPC challenges bound to the exact endpoint/key, with an explicit first-use
confirmation view and distinct changed-key errors. Keep authentication and
terminal commands separate until platform credential access is chosen. See
`docs/ssh-host-trust.md`; the isolated candidate is not a production/mobile
library decision. Live Proxmox checks still await the approved PC/catalog.

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

Application provisioning, VM instances/lifecycle, snapshots, desktop/mobile SSH,
authenticated remote control planes and signed distribution are not implemented.
Default tests continue to work without Proxmox, credentials or mutable VMs.
