# Continuous checks

`.github/workflows/check.yml` prepares two independent GitHub Actions jobs for
pushes, pull requests and manual dispatch:

- Ubuntu 24.04 server: repository-pinned Rust, formatting, locked all-target
  Clippy and locked server tests, including localhost TLS and subprocess tests.
- Ubuntu 24.04 frontend: Node 26.11.1, `npm ci` using the committed lockfile,
  TypeScript checks and Vite production build.

The actions are pinned to reviewed commit IDs, checkout credentials are not
persisted, token permissions are read-only, and superseded workflow runs are
cancelled. No credentials, Proxmox endpoint, VM mutations or release publishing
are configured. The jobs have explicit time limits. Dependencies are still
fetched over the network; this is not an offline build.

Rustup reads `rust-toolchain.toml`, so changing the pinned compiler does not
require copying its version into the workflow. Update the Node version and the
action commit pins deliberately when updating the tested toolchain.

The YAML structure passed actionlint 1.7.12 on macOS (temporary binary checked
against its published checksum manifest), and the configured commands passed
locally. The workflow
has not been pushed or executed on GitHub, and Linux test success has not been
claimed. Native Tauri builds, signed installers, mobile builds, live Proxmox and
SSH/PTY validation are outside these jobs. Run `make check` locally for the
native Rust tests and frontend checks on the current supported development Mac.

References: [checkout action](https://github.com/actions/checkout),
[setup-node action](https://github.com/actions/setup-node).
