<p align="center"><code>devutils / PROVISION</code></p>

<h1 align="center">VMAnyware</h1>

<p align="center">Reproducible environments on your own Proxmox homelab.</p>

<p align="center">
  <a href="https://devutils-org.github.io/tools/vmanyware/">Website</a> ·
  <a href="#documentation">Documentation</a> ·
  <a href="https://github.com/devutils-org">devutils</a>
</p>

## Overview

- Edit, validate and store reusable YAML environment definitions locally.
- Inspect Proxmox nodes/templates and preview provisioning without creating VMs.
- Try the native macOS SSH terminal prototype with verified host trust.

In development: confirmed provisioning and lifecycle workflows pass local fixture
checks. Android debug builds and native emulator smoke checks pass; the latest
mobile keyboard adapter still needs runtime validation. Real-host provisioning
and the complete mobile journey remain unverified. See [current evidence](STATE.md).

To preview provisioning, configure `VMANYWARE_TEMPLATES` with your explicit
[image-to-template catalog](docs/provisioning-plan.md), enter a candidate VMID
and click **Preview provisioning**. The offline plan never reserves or creates
a VM. **Check against Proxmox** performs read-only preflight checks when a
provider is configured, reporting passed, failed and unknown observations.
The opt-in confirmed create API and sequential worker now pass a complete local
HTTPS fixture journey, including disk expansion and guest checks. The remote form now has creation review, explicit confirmation and status refresh;
its native HTTPS and browser checks pass separately. Native/mobile runtime and
real-host validation remain pending. See the
[create API contract](docs/provisioning-plan.md#confirmed-minimum-create-api). For the upcoming
test PC, follow the [Debian Cloud-Init template guide](docs/debian-template.md).

## Install

Development checkout only; no packaged installers yet. Requires Node.js 22.12+,
Rust 1.99.1 and [native prerequisites](https://v2.tauri.app/start/prerequisites/).

```sh
make doctor
make install-client
```

See the [platform matrix](docs/platforms.md) for tested targets.

## Use

Run in separate terminals from the repository root:

```sh
make server     # local API on 127.0.0.1:8080
make dev        # native Tauri client
```

Android debug APKs and an Android 15 emulator native bridge/key-persistence smoke
now pass. See the [Android build and owned-emulator procedure](experiments/android-native/README.md).
Actual mobile SSH/create journey, physical devices and release packaging remain open.

## Repository

Choose **Check local service** to try the app without Proxmox. Edit and save a
YAML definition locally; configure [Proxmox access](docs/proxmox-setup.md)
separately for read-only inventory. `make web` is a browser preview, used instead
of `make dev`; it does not provide native SSH functionality.

## Documentation

Registered definition/VM associations and read-only monitoring are described in
[instance registration and observation](docs/instances.md). The optional Linux
`make check-instances-browser` target exercises React with disposable Firefox and
simulated IPC; it does not replace native Tauri validation.

The workspace contains the client and server crates. The server implements
local liveness, read-only provider inventory and definition storage; see
[API behavior and verification](docs/api.md). Do not create
empty domain, storage or SSH crates in advance. No app signing, installers or
mobile projects are generated yet. The local bundle identifier is provisional.

[Setup and workflows](REFERENCE.md) · [Definitions](docs/definitions.md) ·
[Provisioning previews](docs/provisioning-plan.md) ·
[SSH terminal](docs/ssh-terminal.md) · [Roadmap](VMAnyware.md)

## Development

```sh
make check      # frontend and Rust checks
```

See [working instructions](AGENTS.md) and [verification evidence](STATE.md).
Native launches and real infrastructure checks are separate from local tests.

## License

[MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
