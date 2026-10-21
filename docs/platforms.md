# Platform and toolchain evidence

VMAnyware has one validated development baseline as of 2026-10-17. No signed
release or broad minimum-OS support guarantee is available yet. Future roadmap
targets must be proven separately; upstream Tauri support alone is insufficient.

| Target | Current evidence | Still required |
| --- | --- | --- |
| macOS 27.0.1 arm64, Apple M1 | Native dev launch/bridge; workspace tests/release executable; real Tauri/IPC/bundled-terminal/disposable-agent/shell smoke | Normal startup with deliberately selected real agent/guest, physical keyboard, signing/installers |
| Linux | Arch Linux x86_64: frontend build/23 Node tests; workspace fmt/Clippy and 122 tests (104 server, 18 native HTTP) using a temporary signed-package WebKitGTK prefix. Isolated software-rendered native React/WebKit/Tauri IPC -> actual Axum -> generated-HTTPS start/shutdown passes. Earlier Debian 12 arm64 container checks passed; Ubuntu CI prepared | Normal packaged/native startup, GPU rendering, Linux SSH terminal and first CI run |
| Windows | Portable source plus Ctrl-C handling prepared | MSVC/WebView2 setup, compilation, launch, process/terminal tests |
| iOS | No application build/runtime evidence | Full Xcode, signing, simulator/device and SSH/terminal/lifecycle proof |
| Android | ARM64/x86_64 debug APKs build; Android 15/API 35 x86_64 emulator launches the packaged app with actual Tauri bridge, encrypted identity/restart, native guard and painted-layout proof; current APK SSH/PTY/input/rotation/reconnect pass via owned ADB tunnel | Direct-network remote create journey, full software IME/suspend-resume, physical ARM64, current WebView parity, iOS-independent proof and release packaging |
| Browser | Vite build/preview | Native bridge intentionally unavailable; this is not the product's native runtime |

The current Mac reports macOS build `26A434`. Apple developer directory is
`/Library/Developer/CommandLineTools`, not full Xcode. `adb` and `sdkmanager`
were not found on PATH during the check; their absence on PATH is not proof
that no SDK exists elsewhere. No personal SDK/configuration files were inspected.

## Current Android artifacts — 2026-10-21

Both current-source debug APKs now compile and are copied to `.lab/packages/android/`
with verified signatures, package/version/ABI, API 24/36 declarations and exact
embedded-native-library SHA-256 matches. See [installation](install-android.md).
Their manifest records `runtimeValidatedForTheseApks:false`: earlier emulator/SSH
reports below belong to historical APKs and do not prove these new binaries run.
Physical ARM64, fresh runtime/input/network behavior and release signing remain open.

## Optional Linux identity key store — 2026-10-21

Source now offers explicit save/unlock/forget of the application SSH unlock
passphrase through Secret Service, plus an SSH identity-source choice. A real
isolated GNOME Keyring/private D-Bus fixture passes roundtrip/failure and native
SSH authentication checks; React/Firefox controls pass separate simulated-IPC
checks. Joined WebView/IPC proof is next. This does not store a non-exportable SSH
key, implement macOS Keychain/Android Keystore or update existing delivery files.
See [ADR 0087](decisions/0087-linux-system-identity-unlock.md).

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

`make check-linux-client` uses the same isolation and cleanup, with the pinned
Node 26.11.1 Debian 12 arm64 image. It runs `npm ci --no-audit --no-fund`, the
TypeScript/Vite build and seven controller tests in the disposable source copy.
Node 26.11.1/npm 11.20.0 passed these checks on 2026-10-17. Native Tauri is not
built or launched by this target. Both Linux targets share `scripts/check-linux.mjs`.

## Linux native HTTP command validation

The 2026-10-18 [temporary signed-package prefix](native-linux-prefix.md) unblocked
workspace compilation/Clippy and all 95 Linux tests (80 server, 15 native HTTP).
The frontend build, 15 Node tests and disposable Firefox simulated-IPC walkthrough
also pass. No global package was installed, no native graphical window launched
and no Linux SSH terminal support inferred. Standard native checks still require
installed native prerequisites or the retained temporary build prefix.

## Initial terminal support boundary

[ADR 0019](decisions/0019-macos-first-terminal.md) limits the initial MVP terminal
to macOS. iOS, Android, Linux and Windows native terminals are unsupported until
separately validated. This is a scope decision, not mobile feasibility evidence
or a release-readiness claim for macOS.


## Isolated Linux native lifecycle GUI

The [native lifecycle harness](../experiments/lifecycle-native/README.md) now passes
real WebKit/Tauri IPC and actual control-plane/provider transport in owned
Xvfb/Mesa/XDG/D-Bus namespaces. It drives explicit start/graceful-shutdown
confirmation against generated HTTPS only. This updates the earlier compile-only
HTTP evidence; it does not establish normal packaged startup, native profile
restart, hardware GPU rendering, SSH or real Proxmox support.


## Application SSH identity preparation

ADR [0046](decisions/0046-encrypted-application-ssh-identity.md) adds an explicit
passphrase-encrypted Ed25519 identity and public-key insertion in remote creation.
Unix source is enabled for Linux/macOS/Android/iOS, but only Linux compilation and
owned local fixtures are proven. It is encrypted app-local storage, not an OS
keystore or mobile SSH support claim. Existing agent/PTY still requires macOS.
Mobile storage/backup policy, native command invocation and on-device
authentication/terminal runtime remain open. Android/iOS SDK commands are absent from this Linux PATH and
only the host Rust target is installed; this does not rule out SDKs elsewhere.


## Application-key SSH protocol validation on Linux

[ADR 0047](decisions/0047-application-key-ssh-terminal.md) enables the shared Unix
host/authentication/PTY source with the encrypted app key. Generated localhost SSH
and a real disposable /bin/sh prove authentication, strict fresh host verification,
output, resize, Ctrl-C, close and reconnect on Linux. React/Firefox simulated IPC
proves the visible application-key flow and stale cleanup separately. This updates
the initial macOS-only source boundary; it is not packaged Linux/native GUI or mobile
support. No current macOS-agent regression run is claimed on Linux. Android/iOS
build/runtime, native command dispatch, key-storage backup policy, software keyboard
and suspension remain required. Next build/launch attempt targets Android on Linux
provisionally, pending the user's platform preference; iOS remains in the roadmap.


## First Android package and native launch

[ADR 0048](decisions/0048-android-build-and-native-smoke.md) and the
[owned emulator procedure](../experiments/android-native/README.md) record actual
SDK/NDK installation, two debug APKs and Android 15 x86_64 native evidence. The
editable generated Android project is retained in Git. Min API 24/target 36 are
configuration values, not tested OS bounds. Real WebView/Tauri commands create an
encrypted identity and reopen the same key after process restart; endpoint/session
guards reject invalid inputs. A framebuffer check plus reviewed screenshot proves
actual painted launch, distinct from DOM or native protocol compilation. ARM64 was
packaged but not executed. APK signatures are debug signatures; no release/Play,
physical phone, Android SSH session, keyboard/lifecycle or iOS claim is made.


## Current Android SSH input evidence — 2026-10-19

The current x86_64 debug APK passes actual native SSH trust/authentication, typed
shell commands, rotation/OS PTY size, foreground interrupt, reconnect and process
death cleanup on the owned Android 15 emulator. The emulator host alias returned
`Network is unreachable` in an independent TCP probe, so this evidence explicitly
uses an owned ADB reverse tunnel. Direct remote network access remains unverified.
The software keyboard is visible; command input uses ADB events, not physical or
complete software-IME interaction. Controls wrap with 44px targets; a current
terminal screenshot was reviewed. ADR 0049 and the Android harness README record
scope and remaining limits. Historical statements above describe earlier slices.

## Android native HTTPS and retained-operation recovery — 2026-10-19

The current x86_64 APK now initializes the Android TLS verifier and packages its
matching Kotlin component. A per-connection public CA supports privately issued
server certificates without changing device roots or disabling verification.
The owned native HTTPS harness uses the production server with an isolated DB,
a generated HTTPS proxy and explicit ADB reverse mappings. It verifies issuer/
hostname refusal before HTTP, actual bearer rejection, authenticated connection,
retained failed-operation/target recovery and explicit reconnection after process
restart without token/profile persistence or request replay. No writes are sent.
The absent template catalog remains an actionable error. These are native Android
checks, not browser IPC simulation. Direct deployed access, complete create-to-SSH,
physical devices and release/iOS support remain separate gates (ADR 0050).


## Android generated-provider creation and SSH integration — 2026-10-19

The current x86_64 APK now completes real native confirmation/API/production-worker
creation and strict readiness against an owned HTTPS provider fixture, followed
by explicit exact-address/account handoff. The same app key is asserted in
Cloud-Init and authenticates to a real localhost OS-shell SSH peer after explicitly
changing the endpoint to the owned ADB tunnel. Exactly one creation and expected
provider mutations remain unchanged across SSH/restart. No native IPC is mocked.
The guest/IP and readiness evidence are simulated; this is not real VM or direct
network proof. ADR 0052 and .lab/android-create records separate these boundaries.


## Direct emulator transport supersedes the tunnel boundary — 2026-10-19

The same current x86_64 APK now passes native verified HTTPS/create/API/worker
and strict SSH/PTY using 10.0.2.2 directly, including a cold-boot repeat with an
owned TCP readiness probe. No data-plane ADB reverse mapping exists before or
after the journey; ADB/CDP remains instrumentation only. No route/firewall/trust
modification was required. The older intermittent failures were not reproduced;
their precise cause remains unproven. Reports label direct and tunnel transports.
ADR 0053 supersedes the historical host-alias limitation, while synthetic guest,
physical/deployed network, full IME/lifecycle and real VM limits still apply.
