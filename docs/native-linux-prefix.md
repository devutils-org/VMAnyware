# Linux native command validation with a temporary package prefix

Date: 2026-10-18. Initial evidence was compilation/headless tests. A later
isolated native lifecycle GUI/IPC check now passes as recorded below; installed
runtime, SSH terminal and packaged release claims remain separate.

The Arch Linux x86_64 host has GTK 3/libsoup but lacks globally installed
WebKitGTK/JavaScriptCoreGTK 4.1. A temporary user-owned prefix allowed the actual
Tauri crate and its HTTP contract tests to compile without root/system changes.

## What was done

Read the system package database's missing dependency plan with `pacman -Sp`.
Downloaded each exact package and detached signature through verified HTTPS from
the Arch package mirror. Verified signatures with the public keyring shipped by
`archlinux-keyring`; `pacman -Qkk archlinux-keyring` reported 17 files, zero altered.
The packaged keyring is armored, so an initial gpgv attempt could not read it;
a dearmored copy in the temporary directory fixed the format. No key was fetched
from a keyserver, trust exception applied or personal GPG configuration accessed.

Only after successful verification, extracted each package's `usr` tree into the
owned prefix without executing installation hooks. Adjusted only those copied
pkg-config files to reference the temporary prefix. Binaries/signatures and the
system package database were not modified.

| Package | Version |
| --- | --- |
| enchant | 2.8.21-1 |
| harfbuzz-icu | 14.6.0-1 |
| hyphen | 2.8.9-1 |
| libmanette | 0.2.13-2 |
| woff2 | 1.0.2-6 |
| xdg-dbus-proxy | 0.1.9-1 |
| webkit2gtk-4.1 | 2.54.1-1 |

The [official Arch package page](https://archlinux.org/packages/extra/x86_64/webkit2gtk-4.1/)
identifies the WebKitGTK package/version and dependencies. This was a host-specific
validation setup, not a new project dependency installer or portable SDK.

## Actual result

With the temporary `usr/lib/pkgconfig` in PKG_CONFIG_PATH and its `usr/lib` in
LD_LIBRARY_PATH/LIBRARY_PATH, `make check-rust` passed workspace formatting,
all-target Clippy with warnings denied and all 95 tests: 80 server and 15 native
client HTTP tests. The latter include history, instance observation and lifecycle
preflight commands previously left source-prepared. A standalone client test-list
build also succeeds without relying on the server's dependency feature graph.
Frontend build, 15 Node controller tests and the Firefox simulated-IPC walkthrough
passed separately on the same source. No native GUI or real IPC-to-WebView launch
was performed by these commands.

## Resume and limits

The retained temporary directory is `/tmp/vmanyware-native-deps-61ktnqj7`.
`packages.json` records exact package URLs, SHA-256 values and signature outputs.
Its `prepare.py` and `check.py` are local experimental helpers, not repository
runtime code. While it still exists, from the repository run:

```sh
python3 /tmp/vmanyware-native-deps-61ktnqj7/check.py
```

The prefix is disposable and may vanish on reboot; inspect its existence before
relying on it. Default checks on an unconfigured host still need proper native
system prerequisites. WebKit helper/resource paths were not relocated or tested
for application launch. A deployment should use supported installed dependencies,
not this temporary validation prefix. macOS native integration, physical keyboard,
Linux SSH support, signing/installers and mobile evidence remain separate gates.


## Later real native lifecycle fixture

Added verified/extracted libxmu 1.3.1-1, xorg-xauth 1.1.5-1 and
xorg-server-xvfb 21.1.25-2 to the same owned prefix, without hooks or system
installation. `display-packages.json` retains exact URLs/hashes/signature evidence.
A private bubblewrap read-only `/usr` overlay makes original WebKit helper paths
available for this process tree only; temporary X11/XDG mounts and a separate
D-Bus session isolate GUI state. Existing home directories are hidden.

`make check-lifecycle-native NATIVE_TEST_PREFIX=/tmp/vmanyware-native-deps-61ktnqj7`
passes the build/23 Node tests and real React/WebKit -> Tauri Rust IPC -> actual
Axum -> generated HTTPS start/shutdown assertions. It requires exactly one POST
per action and separate task/power results; no invoke is mocked. The example uses
a separate fixture identifier, embedded assets and ephemeral WebView storage.

The final prefix `make check-rust` also passes fmt, all-target warnings-denied
Clippy and all 122 workspace tests (104 server, 18 native). Rustfmt initially
resolved the imported command module's test path incorrectly; an explicit
repository-relative test module path fixes formatting/all-target compilation.

Software Mesa rendering avoids this host's Xvfb/NVIDIA EGL crash. Minimal-session
D-Bus/portal/accessibility warnings remain; asserted interactions still pass.
WebKit sandbox, X11 cookie authentication and TLS verification were not disabled.
Normal packaged startup, hardware GPU rendering, real Proxmox, macOS/mobile and
SSH remain outside this result. See the [native harness](../experiments/lifecycle-native/README.md).
