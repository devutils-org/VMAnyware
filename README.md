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

In development: VM management is not implemented in this revision. Real-host
provisioning and mobile support remain unverified. See [current evidence](STATE.md).

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

Choose **Check local service** to try the app without Proxmox. Edit and save a
YAML definition locally; configure [Proxmox access](docs/proxmox-setup.md)
separately for read-only inventory. `make web` is a browser preview, used instead
of `make dev`; it does not provide native SSH functionality.

## Documentation

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
