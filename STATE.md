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
- Debian Cloud-Init template guide and configuration/API/operation/task/diagnostic
  documentation. No real template preparation commands have been executed.

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
- CI YAML triggers/jobs parsed locally. Its commands have passed locally; no
  GitHub runner, native packaging or mobile CI result is claimed.
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

Run an isolated local SSH/PTY feasibility spike with generated ephemeral fixture
keys, explicit host-key trust, changed-key rejection, authentication, data,
resize and reconnect. Keep experimental dependencies outside the root workspace
until working evidence supports an architecture decision. A protocol fixture
alone must not be presented as a real interactive shell or mobile validation.

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
