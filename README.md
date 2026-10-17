# PocketLab

A native client for reproducible Linux environments on a self-managed Proxmox
homelab. The repository directory is currently named `VMAnyware`; PocketLab is
the working product name used in the roadmap.

**Status: Phase 0.** The frontend build, workspace Rust checks/tests and local
control-plane HTTP smoke check pass on macOS arm64. The native development
process starts; the user confirmed the UI bridge returns `macos`.
VM management is not implemented. A macOS SSH terminal prototype is wired to
the native transport; an isolated real-Tauri/local-shell walkthrough passes,
while real guest and normal application walkthroughs remain pending.

The native client can now connect to the local control plane and list Proxmox
nodes/templates in read-only mode. Configure the server using
[the Proxmox setup guide](docs/proxmox-setup.md), run `make server`, then click
**Connect** in the native app. Missing configuration is reported explicitly.
Real Proxmox verification is pending; default tests use localhost TLS fixtures.
Inventory refreshes run as persisted background operations, with status polling,
idempotent requests and restart recovery. See [operation behavior](docs/operations.md).

For a local demo without Proxmox, run `make server` in one terminal and
`make dev` in another. Click **Check local service**: availability is checked
independently of provider configuration. If it is unavailable, check the port,
start the service and click **Retry service check**. Changing the port clears the
previous result. **Connect** separately requests read-only Proxmox inventory.

You can also edit, save and reload reusable YAML definitions without Proxmox.
The server validates the schema and persists definitions in SQLite, without
creating VMs. See [definition format and storage](docs/definitions.md).

To preview provisioning, configure `POCKETLAB_TEMPLATES` with your explicit
[image-to-template catalog](docs/provisioning-plan.md), enter a candidate VMID
and click **Preview provisioning**. The offline plan never reserves or creates
a VM. **Check against Proxmox** performs read-only preflight checks when a
provider is configured, reporting passed, failed and unknown observations.
Actual provisioning and real-host validation remain pending. For the upcoming
test PC, follow the [Debian Cloud-Init template guide](docs/debian-template.md).

## Start development

Install Node.js (22.12+ or a compatible newer release), npm, Rust 1.99.1 with
rustfmt/clippy, and the native dependencies for your operating system. See the
[official Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).
iOS additionally requires full Xcode; Android requires its SDK/NDK toolchain.

From the repository root:

```sh
make doctor
make install-client
make web                 # browser preview only
make dev                 # native Tauri window, instead of make web
make check               # frontend build/tests, Rust fmt/clippy/tests
make server              # local Axum API on 127.0.0.1:8080
make check-server        # scoped server checks
```

Run the development commands separately: they share port 1420. In the native
window, click **Check native bridge**. Success must report the actual platform
returned by Rust. Browser preview deliberately disables that check.

If `make dev` reports `Port 1420 is already in use`, check whether an existing
native development session or `make web` is running. Reuse its window, or stop
that session with Ctrl-C in its terminal before starting another. To identify
the listener on macOS, run `lsof -nP -iTCP:1420 -sTCP:LISTEN`. Stop only the
identified project session; the API on port 8080 runs independently.

Resolved dependencies are recorded in `apps/client/package-lock.json` and
`Cargo.lock`. Rust is pinned to `1.99.1` in `rust-toolchain.toml`.
Use `npm --prefix apps/client ci` to reproduce the frontend installation.
See `STATE.md` for actual build and runtime verification and the
[platform/toolchain matrix](docs/platforms.md) for tested targets. The prepared
[CI workflow](docs/ci.md) runs server and frontend checks without Proxmox;
GitHub execution and native/mobile CI remain unverified.

The native editor also provides **Validate YAML**, which checks the current
definition without saving. Server diagnostics and shutdown behavior are
documented in [diagnostics](docs/diagnostics.md), and the read-only task helper
in [provider task observation](docs/provider-tasks.md). The
[interim feasibility report](docs/feasibility.md) separates local evidence from
remaining infrastructure and platform tests. The opt-in
[SSH/PTY experiment](experiments/ssh/README.md) exercises a disposable localhost
server and real local shell; run it separately with `make check-ssh`. Its
[persistent host-trust policy](docs/ssh-host-trust.md) is shared with the
[native macOS inspection/confirmation prototype](docs/ssh-host-inspection.md).
Inspection never logs in or opens a shell; manual UI verification remains pending.
The [macOS agent authentication check](docs/ssh-agent.md) lets you explicitly
list identities and test one selected fingerprint after host verification. It
closes the connection without opening a terminal. Default macOS Rust tests use
disposable system agents; they never contact your configured agent. The
[bounded native PTY transport](docs/ssh-sessions.md) now powers a
[graphical terminal prototype](docs/ssh-terminal.md). `make check-terminal-webkit`
optionally tests actual macOS WebKit rendering with isolated simulated IPC; it
never connects to an agent or SSH host.
`make check-terminal-native` runs a separate macOS Tauri example with real native
IPC, a disposable agent and an actual local shell; see its
[fixture boundaries](experiments/terminal-native/README.md).

The native build requires `apps/client/src-tauri/icons/icon.png`. Its editable
source is `icons/source.svg` in the same directory. Regenerate it from the root:

```sh
npm --prefix apps/client run tauri icon -- src-tauri/icons/source.svg --output src-tauri/icons --png 128
mv apps/client/src-tauri/icons/128x128.png apps/client/src-tauri/icons/icon.png
```

## Repository

| Path | Purpose |
| --- | --- |
| `VMAnyware.md` | Product scope, roadmap and acceptance criteria |
| `AGENTS.md` | Instructions for sequential agent sessions |
| `STATE.md` | Current evidence, blockers and next task |
| `docs/phase-0.md` | Ordered feasibility tasks and exit criteria |
| `docs/decisions/` | Architecture decisions and their reasons |
| `apps/client/` | React/TypeScript interface and Tauri 2 native entry point |
| `server/` | Local Axum API and HTTP/configuration tests |
| `migrations/` | Transactional SQLite schema migrations |
| `examples/` | Reusable YAML examples with public demonstration keys |
| `scripts/doctor.mjs` | Local tool availability check, without credentials/network |

The workspace contains the client and server crates. The server implements
local liveness, read-only provider inventory and definition storage; see
[API behavior and verification](docs/api.md). Do not create
empty domain, storage or SSH crates in advance. No app signing, installers or
mobile projects are generated yet. The local bundle identifier is provisional.

## Working method

One small task per iteration, targeted checks, then an evidence-based update to
`STATE.md`. Preserve reusable definitions independently of live VMs. Keep
Proxmox credentials on the future control plane, and SSH connections on clients
over LAN/VPN. See the roadmap before implementing either path.

The scaffold follows the official
[Tauri project setup](https://v2.tauri.app/start/create-project/) and
[Vite integration](https://v2.tauri.app/start/frontend/vite/) conventions.
