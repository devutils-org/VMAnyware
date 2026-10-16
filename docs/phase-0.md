# Phase 0 — Implementation brief

The purpose is to retire feasibility risks, not implement the full roadmap.
Each row is one bounded task; update `STATE.md` with its evidence before moving on.

| Order | Task | Acceptance evidence |
| --- | --- | --- |
| 0A | Resolve dependencies and run the client shell on macOS | Commit both dependency locks and a tested Rust toolchain pin; frontend build, Rust fmt/clippy/tests pass; native window opens and bridge reports `macos` |
| 0B | Read-only Proxmox access from a Rust prototype | Verified TLS with a trusted CA; dedicated token; read version, nodes, known template metadata; mock tests for authentication/TLS/timeout/provider failures |
| 0C | Provision one disposable VM | Explicit approval of template, node, storage, network and VMID; clone, apply resources and public SSH key through Cloud-Init, start, poll task and separately verify guest readiness |
| 0D | Snapshot on disposable storage | Record backend/snapshot constraints; create/list; explicitly approve restore; verify restored guest state, not just task acceptance |
| 0E | Desktop SSH/PTY prototype | Real interactive session with verified host key, authentication, terminal resize, Ctrl-C, disconnect/reconnect and changed-key rejection |
| 0F | Mobile SSH/PTY feasibility | Repeat essential terminal checks on iOS and Android; record OS/device/toolchain versions, keyboard/special keys and suspend/resume behavior |
| 0G | Risk report and next architecture decision | Identify proven behavior, remaining unknowns and supported MVP platforms; record any deliberate mobile limitation before Phase 1 |

## First slice: 0A

Prepared source: a Cargo workspace with one Tauri client, React/Vite frontend,
one Rust command, local tool doctor and development commands. The native check
does not access the network or a VM.

Verification procedure:

1. Run `make doctor`; install missing prerequisites outside this restricted
   environment as needed. Availability is not proof of full SDK readiness.
2. Run `make install-client` and `make check-client`.
3. Run `make dev`, click **Check native bridge**, and record its actual result.
4. Run `make check-rust` after the frontend assets exist.
5. Commit resolved lockfiles, pin the working compiler, and record exact versions.

Do not mark the roadmap's Tauri development/build task complete on source review
alone. A native launch and successful check/build are required. Browser preview
does not demonstrate mobile or desktop-native feasibility.

## Proxmox inputs to establish before 0B/0C

- Test endpoint and Proxmox VE version; trusted certificate/CA and routing.
- Dedicated account/token and exact ACLs needed for each proven operation.
  Do not assume admin privileges or guess an untested minimum ACL list.
- Test node, Debian template VMID and template version/preparation steps.
- Target storage, bridge, DHCP/static addressing and available disk capacity.
- Cloud-Init drive, disk resize support and expected readiness mechanism.
- Disposable VMID allocation and ownership tagging to prevent collisions.
- Explicit approved mutation/cleanup boundaries. Do not enumerate or alter a
  real host merely because its credentials are available on the machine.

For the read-only prototype, keep a concrete Proxmox HTTP client with injectable
endpoint/transport and mock responses. Do not create a generic provider framework.
Tokens must come from runtime configuration, be redacted from errors/logs, and
never appear in frontend code. An unavailable host must remain an explicit
untested condition, not be replaced with fake success.

## SSH decisions to establish through 0E/0F

- Rust SSH library support on desktop, iOS and Android, including crypto builds.
- PTY allocation, binary stream transport to the WebView and resize propagation.
- Known-host persistence, first-use confirmation and changed-key rejection.
- Key access via platform facilities without sending private keys to the server.
- Terminal renderer compatibility, mobile keyboard input and lifecycle handling.

Choose the library/renderer after a working prototype. A browser terminal mock
or a successful TCP connection does not prove interactive SSH support.

## Reference entry points

- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
- [Tauri and Vite](https://v2.tauri.app/start/frontend/vite/)
- [Proxmox API viewer](https://pve.proxmox.com/pve-docs/api-viewer/)
- [Proxmox administration guide](https://pve.proxmox.com/pve-docs/pve-admin-guide.html)

Verify API/library details against official documentation during each spike.
