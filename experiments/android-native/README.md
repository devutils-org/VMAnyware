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
