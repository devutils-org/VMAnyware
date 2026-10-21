<p align="center"><code>devutils / PROVISION</code></p>

<h1 align="center">VMAnyware</h1>

<p align="center">Reproducible environments on your own Proxmox homelab.</p>

<p align="center">
  <a href="https://devutils-org.github.io/tools/vmanyware/">Website</a> ·
  <a href="#documentation">Documentation</a> ·
  <a href="https://github.com/devutils-org">devutils</a>
</p>

## Linux server package

`make package-server` creates a local release-binary archive with a systemd unit,
root-only configuration example, template example, checksums and staging installer
in `.lab/packages/`. It never installs or starts a host service. See the
[installation, HTTPS, upgrade and recovery guide](docs/install-server.md).
A packaged-process auth/restart check is available with
`python3 scripts/check-server-package.py <archive>`; actual systemd host acceptance
and mobile/release gates remain open.

Current Android test APKs can be produced with `make package-android` after
configuring an owned SDK/JDK/NDK and isolated Android/Gradle caches. The packager
verifies signatures, ABI/version and current native-library content; it installs
nothing. See [Android installation and test instructions](docs/install-android.md).
These debug artifacts still require fresh runtime/device acceptance.

## Overview

VMAnyware combines a native Tauri client, a self-hosted Rust API and an explicit
Proxmox template catalog. Environment definitions outlive individual VMs.

Implemented, with local/generated-provider evidence:

- Strict YAML definitions, canonical hashes and SQLite persistence.
- Verified remote HTTPS, bearer authentication and read-only inventory/preflight.
- Confirmed clone, CPU/memory/disk configuration, Cloud-Init and guest readiness.
- Remote start/shutdown, explicit power reads and retained definition inspection.
- Snapshot creation/restore, deletion and retained-definition recreation.
- Durable operations, idempotent requests and read-only restart recovery.
- SSH host trust, encrypted application identity and an interactive native terminal.
- Optional Linux Secret Service and Android Keystore protection for the identity
  unlock passphrase (Android runtime acceptance pending).
- API/worker execution of declared packages and an optional final disk-only snapshot.

The package/final-snapshot workflow passes all create/recreate combinations against
an owned generated HTTPS provider. Native plan validation passes protocol checks, and explicit package/final-snapshot
inputs/reviews and joined Linux native create/recreate journeys now also pass against
the owned provider, including lost-reply retry and final stage evidence. Refreshed Linux
server/Android debug artifacts pass packaging checks; actual platform/guest
acceptance remains open.

**Development status:** this is not a release candidate. Linux native integration,
server package checks and Android compilation/signature/content checks have evidence.
The exact-current APK journey, real guest provisioning/packages/snapshot restoration,
real deployment, OS credential-store integration and full platform acceptance remain
open. Fixtures are not proof of a working home Proxmox deployment.
See [current state](STATE.md), [remaining work](docs/roadmap-status.md) and the
[platform matrix](docs/platforms.md); no audited completion percentage is claimed.

Safe package/snapshot/provider failure reasons and explicit recorded stage reads are
available in creation, recreation and saved recovery. Native readers validate the
operation/target/plan; private journal and guest/provider output stay hidden. Joined
Linux native journeys, including the exact extracted server package, pass against
an owned generated provider. Refreshed server and both Android debug artifacts now
include this workflow; current Android execution and real guest acceptance remain
open. A home Proxmox host will need an explicitly mapped Debian template,
storage/network, verified TLS, least-privilege token and an approved disposable
target. Start with the [template preparation guide](docs/debian-template.md) and
[Proxmox setup](docs/proxmox-setup.md).

The optional Linux Secret Service controls pass isolated real-daemon/native SSH,
browser and joined Linux WebView/production IPC/terminal checks. The private key
remains encrypted in application storage. Android Keystore now compiles/packages
for both ABIs, with scoped ciphertext and JNI checks; actual device behavior, macOS
Keychain and normal deployment/platform acceptance remain open. Current Android
APKs include the vault commands but have not been executed. See
[Linux decision](docs/decisions/0087-linux-system-identity-unlock.md) and
[Android decision and limits](docs/decisions/0088-android-keystore-identity-unlock.md).

## Install

For a development checkout, use Node.js 22.12+,
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

Historical owned Android integration is recorded separately from current APKs.
See the [Android build and owned-emulator procedure](experiments/android-native/README.md).
Current debug artifacts require their own runtime acceptance; physical devices,
iOS/Windows parity and production release packaging remain open.

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

The workspace contains the client and server crates, migrations, owned fixtures,
operator documentation and packaging scripts. See
[API behavior and verification](docs/api.md). Linux server archives and signed debug
Android APKs can be built locally; production signing/distribution and supported
installation/upgrade environments remain to be validated.

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
