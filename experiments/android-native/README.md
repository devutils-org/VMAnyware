# Owned Android native smoke

This opt-in check runs the real packaged app and Tauri commands in a dedicated
Android emulator. No IPC simulation, real device, control-plane server, personal
key, Proxmox endpoint or VM operation is used. It intentionally resets only
`dev.vmanyware.client` in the AVD named `vmanyware-native-smoke`, on the owned ADB
server port 5047 and emulator serial `emulator-5580`. Both the AVD name and QEMU
property are checked before any app mutation. Never point it at another target.

## Build

Install the [Tauri Android prerequisites](https://v2.tauri.app/start/prerequisites/)
in an owned development prefix. Set JAVA_HOME, ANDROID_HOME, NDK_HOME,
ANDROID_USER_HOME, ANDROID_AVD_HOME and GRADLE_USER_HOME. Keep SDK/AVD/signing caches
outside the repository. Do not change HOME or load a personal signing identity.
Install Rust targets `aarch64-linux-android` and `x86_64-linux-android`.
The Android project is already prepared in `apps/client/src-tauri/gen/android`;
do not regenerate it over the checked-in manifest/activity/Gradle configuration.

```sh
npm --prefix apps/client run tauri -- android build --debug --apk \
  --target x86_64 aarch64 --split-per-abi --ci
```

Debug APKs are under `app/build/outputs/apk/{x86_64,arm64}/debug/` in that project.
They are large because native debug symbols are retained; they are development
artifacts, not release packages or Play Store evidence. The local initial build
reused the already verified current frontend assets with a beforeBuildCommand
configuration override; the ordinary command above builds the frontend normally.
No server or unrelated workspace suite is necessary for this Android-only slice.

The observed environment was Linux x86_64/KVM, Rust 1.99.1, CLI 2.12.3, JDK 21.0.2,
SDK/build-tools 36, NDK 28.2.13676358, Gradle 9.6.1, AGP 9.3.1 and Kotlin 2.2.10.
The Gradle distribution URL/checksum and Android project inputs are committed;
local.properties, tauri-generated Kotlin/Gradle, JNI libraries and build outputs
are ignored. SDK archives came from Google's HTTPS repository metadata and the
JDK archive was checked against its published SHA256. These are observed tooling
versions, not a release security/minimum-platform guarantee.

## Start the dedicated emulator

Use the Android SDK tools to create `vmanyware-native-smoke` from
`system-images;android-35;google_apis;x86_64` with the Pixel 6 device definition.
Verify where avdmanager writes it: this tool version uses ANDROID_USER_HOME/avd;
point ANDROID_AVD_HOME there too. The local fixture uses a 2 GiB data partition
because /tmp is bounded; the device template's default wanted 12 GiB free.

```sh
adb -L tcp:127.0.0.1:5047 start-server
ANDROID_ADB_SERVER_PORT=5047 ADB_SERVER_SOCKET=tcp:127.0.0.1:5047 \
  emulator -avd vmanyware-native-smoke -ports 5580,5581 \
  -grpc 8558 -grpc-use-jwt -no-window -gpu swiftshader -no-audio \
  -no-boot-anim -no-snapshot -accel on -cores 2 -memory 2048
```

Wait for sys.boot_completed=1 on that serial, then install the x86_64 APK with
`adb -P 5047 -s emulator-5580 install -r <apk>`. Do not use an unqualified adb
install or another emulator/device. No gRPC token is read by this check.

## Run and inspect

```sh
VMANYWARE_ANDROID_ADB=/absolute/owned/sdk/platform-tools/adb \
  node experiments/android-native/check.mjs .lab/android-native --reset-owned-app
```

The harness checks the actual native platform button, SSH support flags, encrypted
identity creation, wrong-passphrase/no-replacement refusal and the same public
identity after force-stop/relaunch. It verifies native HTTPS/SSH input and session
ownership guards without opening an external connection. It checks a narrow launch
layout, the actual device framebuffer (a mounted DOM alone can still be blank),
and records report.json/launch.png. Passphrases stay in fixture memory; only public
fingerprints enter the report. CDP forwards and the app are closed in finally.
The emulator/ADB server are separately owned and may be stopped after the check:
`adb -P 5047 -s emulator-5580 emu kill`, then `adb -P 5047 kill-server`.

Current evidence is Android 15/API 35 x86_64 with WebView 124.0.6367.219. ARM64 was
compiled/packaged, not launched. Actual mobile SSH/PTY, authenticated remote TLS,
create/readiness/SSH integration, keyboard/IME, suspend/resume, physical devices,
iOS, real infrastructure and signed release support remain open. Backup exclusions
are packaged/inspected, not an executed OEM backup/restore test. See ADR 0048.


## Owned SSH and keyboard journey

After rebuilding and installing the APK from the current source, build the owned
Unix host fixture with `cargo build -p vmanyware-client --example
android_ssh_fixture` (the host's native build prerequisites still apply). Then run:

```sh
VMANYWARE_ANDROID_ADB=/absolute/owned/sdk/platform-tools/adb \
VMANYWARE_ANDROID_SSH_FIXTURE="$PWD/target/debug/examples/android_ssh_fixture" \
  node experiments/android-native/check.mjs .lab/android-ssh --reset-owned-app --ssh
```

The same exact AVD/serial/reset guards apply. The host example requires the
harness's explicit opt-in argument, accepts only the disposable public key on
stdin, generates its host identity, and binds an ephemeral loopback port. The
emulator reaches it through its host alias. The real shell runs in a temporary
directory with a cleared environment; no personal private key is read or exported.
The check compares the generated fingerprint before confirming trust, checks
refusal/authentication-only behavior, types fixed commands into the Android input
field, verifies actual shell output and PTY size after rotation, interrupts a
foreground process, reconnects and checks cleanup on application process death.
It rotates only the fixture's generated host key and verifies changed-key refusal.
The harness stops its fixture, closes forwards and restores the owned emulator's
orientation/IME setting in cleanup. No real VM or Proxmox operation is involved.

Passing this opt-in check is distinct from browser simulated-IPC coverage. ADB
key events do not establish complete software-keyboard/composition, physical
device, suspend/resume or iOS support. Current adapter acceptance requires a fresh
APK runtime report; historical reports for an earlier APK do not verify it.


If an independent TCP probe establishes that the emulator host alias has no
route, explicitly set `VMANYWARE_ANDROID_SSH_ADB_TUNNEL=1` for this fixture run.
The harness creates a no-rebind ADB reverse mapping for only its ephemeral
loopback SSH port, uses 127.0.0.1 in the app, labels the report as tunneled and
removes that exact mapping in cleanup. This mode verifies native SSH/input while
leaving direct network reachability unproven. It is not a deployment transport.
The 2026-10-19 current-APK run passed in this mode after a separate emulator TCP
probe returned `Network is unreachable` for 10.0.2.2. Special-key controls now wrap
on narrow screens with 44px minimum touch targets; the native check asserts this
and the terminal screenshot was reviewed. PTY dimensions are sampled after typed
input triggers keyboard/rotation resizing and settles, before executing `stty`.

## Authenticated HTTPS and read-only recovery

Build/install the current APK and build the production service binary separately.
On the guarded emulator run:

```sh
VMANYWARE_ANDROID_ADB=/absolute/owned/sdk/platform-tools/adb \
VMANYWARE_ANDROID_CONTROL_PLANE="$PWD/target/debug/vmanyware-server" \
  node experiments/android-native/check.mjs .lab/android-remote --reset-owned-app --remote
```

This mode generates a disposable CA and two server certificates, runs an HTTPS
proxy and the actual production control-plane binary on ephemeral loopback ports,
and connects through explicitly reported ADB reverse mappings. OpenSSL and Python
3 are host prerequisites. The child environment contains only fixture service
settings; no provider configuration is inherited. Its database lives in an owned
temporary directory and contains one seeded failed operation/target for recovery.
It is not a claim that provisioning or guest initialization actually happened.
The proxy rejects non-GET requests. No real VM or external service is contacted.

The real APK rejects malformed/private/bundled/oversized CA input, untrusted TLS
and a hostname mismatch before sending HTTP credentials. The actual server rejects
an invalid bearer token. The UI connects with an explicit public CA, reads retained
target/history details, reports the absent template catalog, then reconnects and
recovers the same failed operation after force-stop/relaunch. Restart must forget
the native profile, token and explicit CA without sending a request. Disconnect
clears connection trust. Reports retain only method/path/public CA fingerprint;
generated private keys and tokens are never printed. The service, sockets,
reverse mappings and generated TLS/database directory are removed in cleanup.
A native recovery screenshot is captured without a populated token field.

The public-CA field uses only that CA for the current connection, leaving normal
TLS signature/hostname/validity checks active. It does not install device roots.
See ADR 0050 for Android verifier initialization and JNI/Kotlin integration.
Passing this mode proves native verified HTTPS/read-only recovery via an ADB
tunnel, not deployed/direct-network access, successful provisioning or release.

## Confirmed creation to SSH (generated provider)

After rebuilding/installing the current APK, opt into the integrated local journey:

```sh
VMANYWARE_ANDROID_ADB=/absolute/owned/sdk/platform-tools/adb \
VMANYWARE_ANDROID_CONTROL_PLANE="$PWD/target/debug/vmanyware-server" \
VMANYWARE_ANDROID_SSH_FIXTURE="$PWD/target/debug/examples/android_ssh_fixture" \
VMANYWARE_ANDROID_SSH_ADB_TUNNEL=1 \
  node experiments/android-native/check.mjs .lab/android-create \
    --reset-owned-app --remote --create
```

`--create` requires `--remote`, includes its own SSH journey and rejects an
additional `--ssh`. It uses the production service/worker with an empty isolated
DB and an in-memory loopback HTTPS Proxmox protocol fixture. No existing provider
configuration is inherited. Only the three fixed creation/plan/definition POST
paths are added to this mode's HTTPS proxy; the default remote check stays
read-only. The provider asserts fixture token, stopped-target configuration,
requested resources, the exact Android public key in Cloud-Init, single clone/
seed/start/readiness execution and no arbitrary guest command. Reports record
method/path only, with no provider secret, bearer token or private key.

The native UI must unlock its existing key, add only the public key, review the
plan, explicitly check confirmation and submit once. Ready status then offers an
explicit exact-address/account SSH handoff. For this test, the synthetic guest IP
is explicitly replaced in the SSH form by the owned tunnel endpoint. The same
app key authenticates against the real localhost OS-shell fixture; this proves
neither a real guest nor reachability of the synthetic address. Strict SSH trust,
interrupt/resize/reconnect/process-death checks still apply. The provider counts
are checked again after SSH/restart to reject creation replay.

The successful run produces create-report.json, ssh-report.json, report.json and
reviewed terminal/launch screenshots. Treat this as actual native/API/worker
integration with generated infrastructure, not a real VM or deployment result.
See ADR 0052. The known host-alias network boundary remains a separate next gate.

## Direct host-alias transport

The current owned emulator now passes native HTTPS/SSH without data-plane ADB
reverse mappings. Use `VMANYWARE_ANDROID_HTTPS_DIRECT=1` on the remote or combined
creation invocation and leave `VMANYWARE_ANDROID_SSH_ADB_TUNNEL` unset. For example:

```sh
VMANYWARE_ANDROID_HTTPS_DIRECT=1 \
VMANYWARE_ANDROID_ADB=/absolute/owned/sdk/platform-tools/adb \
VMANYWARE_ANDROID_CONTROL_PLANE="$PWD/target/debug/vmanyware-server" \
VMANYWARE_ANDROID_SSH_FIXTURE="$PWD/target/debug/examples/android_ssh_fixture" \
  node experiments/android-native/check.mjs .lab/android-direct \
    --reset-owned-app --remote --create
```

Direct mode uses 10.0.2.2, includes it in the generated HTTPS certificate SAN and
still rejects the distinct wrong-host certificate. A bounded owned TCP marker
probe verifies host-alias readiness before app reset; reports include attempts and
time. Direct mode refuses preexisting reverse mappings and checks none remain at
the end. It fails rather than silently enabling a tunnel. ADB/CDP still controls
and observes the app, while HTTPS/SSH data uses emulator networking.

Earlier network-unreachable failures were not reproduced after the network
settled; no routing/firewall/trust mutation was needed. Their exact cause remains
unproven, so boot completion is deliberately separated from TCP readiness. This
supersedes the older assumption of a persistent unavailable host-alias route.
Provider/guest simulation, explicit replacement of the synthetic SSH address,
physical-device/deployment/full-IME/release limitations still apply (ADR 0053).

## Brief background/resume

Set `VMANYWARE_ANDROID_BACKGROUND=1` on the owned `--ssh` invocation to press
Android HOME, observe the actual WebView becoming hidden for two seconds, then
return to the same activity. The fixture checks that authentication attempts,
channels and input bytes do not increase while hidden or on return, and verifies
fresh shell output from an explicitly submitted command afterward. Results are
stored under `background` in `ssh-report.json`.

This does not establish screen-lock, Doze, long suspension, process eviction or
background-service behavior. Commands still use ADB key injection. A focused input
and `mInputShown=true`/`mIsInputViewShown=true` do not prove that Gboard's letter
keys are rendered: the owned emulator sometimes shows only its navigation strip
on initial focus. Initial-focus letter-grid visibility and composition remain
unverified; the later-stage ASCII tap check is described below.

## Actual software-key taps

Add `VMANYWARE_ANDROID_SOFTWARE_KEYS=1` to the owned `--ssh` invocation. This
opt-in fixture uses the reviewed Pixel 6 1080×2400 English QWERTY Gboard layout,
after the rotation/control-key checks. It requires Gboard's full touch region,
taps `echo imeox`, taps Gboard backspace, `k`, and the Gboard send action. Each
letter/space and the deletion must match the real input value; the fixture then
requires exactly 13 additional SSH input bytes, a cleared input, unchanged
channel/authentication counts, and the actual shell output `imeok`.

Results appear under `keyboard` in `ssh-report.json`; `ssh-terminal.png` records
the keyboard immediately before those taps. These are touch events on software
keys, rather than ADB text/key-event injection. The rest of the SSH scenario still
uses injected commands. This is a deliberately fixed-layout experiment, not a
portable keyboard automation API. Initial-focus key-grid visibility,
Unicode/composition, other keyboards and physical devices remain open.

## Current deliverable APKs (2026-10-21)

The current sources build for ARM64 and x86_64 using the restored owned toolchain.
`make package-android` builds both variants and verifies/copies them to
`.lab/packages/android/` with public metadata/checksums and installation instructions.
It requires explicitly configured owned JAVA_HOME/ANDROID_HOME/NDK_HOME,
ANDROID_USER_HOME and GRADLE_USER_HOME. The package step does not install anything.
See [manual Android testing](../../docs/install-android.md) and ADR 0070.

The artifact verifier checks actual signatures, package/version/ABI/SDK declarations
and exact embedded-native-library equality to the current target build. Historical
launch/create/SSH reports elsewhere in this document do not validate these new APK
hashes. Fresh exact-artifact runtime and physical ARM64 acceptance remain open.
