# Platform and toolchain evidence

PocketLab has one validated development baseline as of 2026-10-17. No signed
release or broad minimum-OS support guarantee is available yet. Future roadmap
targets must be proven separately; upstream Tauri support alone is insufficient.

| Target | Current evidence | Still required |
| --- | --- | --- |
| macOS 27.0.1 arm64, Apple M1 | Native dev launch/bridge; workspace tests/release executable; real Tauri/IPC/bundled-terminal/disposable-agent/shell smoke | Normal startup with deliberately selected real agent/guest, physical keyboard, signing/installers |
| Linux | Ubuntu 24.04 server/frontend CI prepared only | First CI run; native WebKit dependencies, desktop launch and terminal tests |
| Windows | Portable source plus Ctrl-C handling prepared | MSVC/WebView2 setup, compilation, launch, process/terminal tests |
| iOS | No application build/runtime evidence | Full Xcode, signing, simulator/device and SSH/terminal/lifecycle proof |
| Android | No application build/runtime evidence | SDK/NDK, Rust targets, emulator/device and SSH/terminal/lifecycle proof |
| Browser | Vite build/preview | Native bridge intentionally unavailable; this is not the product's native runtime |

The current Mac reports macOS build `26A434`. Apple developer directory is
`/Library/Developer/CommandLineTools`, not full Xcode. `adb` and `sdkmanager`
were not found on PATH during the check; their absence on PATH is not proof
that no SDK exists elsewhere. No personal SDK/configuration files were inspected.

## Reproduce the current development checks

| Tool | Tested version/configuration |
| --- | --- |
| Rust/Cargo | 1.99.1, pinned in `rust-toolchain.toml`; rustfmt and Clippy |
| Node | 26.11.1 |
| npm | 11.20.0; use committed package lock with `npm ci` |
| Rust dependencies | Committed root Cargo lock |
| Native client | Tauri 2 with React/TypeScript/Vite; exact versions in locks |

Run `make doctor`, `npm --prefix apps/client ci`, then `make check` on the
validated Mac. `make check-server` avoids native client builds. Browser and
native development commands share the Vite port and should run separately.
The isolated SSH experiment has its own Cargo manifest/lock and optional check
command; it is not part of the application dependency graph or mobile proof.

Native packaging, cross-compilation and on-device tests need the corresponding
platform tools. Do not infer a lower macOS minimum or universal architecture
support from a successful build on this machine. The Mac's lack of nested
virtualization does not block the local service or localhost SSH tests; real
Proxmox tests will use the user's separate PC.

For platform setup, consult [official Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).
Use upstream instructions as prerequisites and record actual project results
before changing the table. See [interim feasibility](feasibility.md) for open risks.

## Isolated Linux server check

`make check-linux-server` checks committed HEAD, not working-tree edits, using
an anonymous Docker configuration and the explicit local Unix socket. It archives
tracked source into a disposable directory and runs locked server formatting,
Clippy and tests in the digest-pinned Rust 1.99.1 Debian 12 arm64 image. The
container has two CPUs, 2 GiB RAM, a PID limit, no extra capabilities and no
published ports. No personal configuration, credentials or Docker socket is
mounted. Public image/component/crate downloads require network access.
The container and temporary source are removed afterward; the image remains in
the local Docker cache. This opt-in check is separate from default checks and
provides neither GitHub Actions nor Linux desktop evidence.
