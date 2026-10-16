# PocketLab — current state

Updated: 2026-10-16. Keep this file short; historical decisions belong in ADRs.

## Implemented

- Tauri 2/React/TypeScript/Vite shell with a `runtime_platform` native command.
  Browser preview disables the native bridge check.
- Local Axum service: health endpoint, safe errors, loopback-only binding,
  validated port and graceful Ctrl-C shutdown.
- Verified-TLS, read-only Proxmox client and local inventory endpoint/UI.
- Strict versioned YAML definitions, deterministic exports/hashes, SQLite
  migration and persistent create/list/read APIs; native YAML editor/catalog.
- Durable inventory jobs: queued/running/terminal states, idempotent enqueue,
  background worker, status API, native polling/resume cache and startup recovery.
- SQLite migration 2 and an exclusive database service lock prevent duplicate
  process ownership. No VM mutation jobs are implemented yet.
- Explicit server-owned image/template catalog and stateless offline provisioning
  plans. Native preview shows source/target/resources/storage/network/access,
  binds inputs with deterministic hashes, and hides stale results after edits.
- README, scoped Makefile checks, tool doctor, API contract, Phase 0 brief,
  agent instructions and architecture decisions. Original roadmap retained.

## Fixes from the first real build

- Formatted the server HTTP contract test with rustfmt.
- Added the required RGBA `icons/icon.png` and editable SVG source. The missing
  icon previously made `tauri::generate_context!()` panic during compilation.
- Changed the provisional bundle ID to `dev.pocketlab.client`, removing the
  warning about the `.app` suffix on macOS.
- Pinned Rust to the tested `1.99.1`. Both Cargo and npm lockfiles now exist.

## Verification

Environment: macOS arm64, Rust/Cargo 1.99.1, Node v26.11.1, npm 11.20.0.
Apple developer directory: `/Library/Developer/CommandLineTools`.

- `make doctor`: passed. The old missing-tool/network restrictions are resolved.
- `make check-client`: passed TypeScript checking and Vite production build.
- `make check-server`: passed fmt, Clippy with warnings denied and 36 server tests.
- `make check-rust`: passed workspace fmt, Clippy with warnings denied and all
  42 tests: 6 native HTTP, 11 configuration/provider/TLS, 8 definitions,
  3 service HTTP, 7 operations and 7 provisioning/catalog tests.
- Live server started on an unused loopback port, returned HTTP 200 with
  `{"status":"ok"}`, and shut down on SIGINT with exit code 0.
- Live HTTP/SQLite restart check passed using a temporary database: validation,
  save/list/read, duplicate HTTP 409, invalid definition HTTP 422, unconfigured
  provider HTTP 503, and unchanged saved record/hash after server restart.
- Live operation smoke passed: HTTP 202, repeated request key returns the same
  operation, failed result survives restart, second process cannot acquire the
  same database, and a simulated pre-crash running record becomes interrupted
  on actual service startup. No provider was configured for this process test.
- Live provisioning-preview smoke passed with the example catalog, temporary
  database and no provider: explicit mapping loaded, stable plan hash, requested
  resources/VMID returned, disk shrink/source VMID/unknown fields rejected,
  no definitions or operations written. Invalid catalog refused binary startup
  without echoing configuration values. Plans explicitly remain unverified and
  do not reserve VMIDs or enable creation.
- `make dev`: compiled successfully and launched `target/debug/pocketlab-client`.
  The user confirmed the native window and button result:
  `Native bridge connected (macos)`. The desktop UI-to-Rust smoke check passes.
- `npm --prefix apps/client run tauri build -- --no-bundle`: passed; optimized
  native executable generated at `target/release/pocketlab-client`. Installers,
  signing and distribution bundles were not built.
- The new preview UI is compiled; a manual walkthrough with the user's real
  template catalog and live host validation are still pending.

The initial implementation is versioned in Git; see `git log` for the commit.
No Proxmox infrastructure has been contacted. Mobile and SSH remain untested.
Phase 0 is not complete; no unsupported roadmap acceptance claims are made.

## Next single task

Implement read-only preflight validation of a provisioning plan against actual
template metadata, target VMID availability, storage and network. Use local TLS
fixtures by default. Live verification awaits the user's test endpoint/catalog;
real VM mutations require a concrete approved disposable target.

## Open inputs

- `../omniscient` and `../atfq` were inspected; `../distroproof` was not found.
- Test Proxmox endpoint, template, storage/network and approved mutation scope.
- iOS/Android build and test devices/simulators for terminal feasibility.

## Remaining scope

Real Proxmox, provisioning, snapshots and desktop/mobile SSH remain untested.
Inventory operation orchestration is implemented; VM orchestration/instances,
authenticated remote control planes and release packaging are not implemented.
Autonomous source work has not been
treated as evidence for those milestones. Phase 0 is still open; independent
definition work is recorded in ADR 0004 rather than marking feasibility complete.
