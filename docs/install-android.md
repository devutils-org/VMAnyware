# Test the Android development APK

These ARM64/x86_64 artifacts are debug-signed development APKs, not a release,
Play Store package or physical-device acceptance claim. `artifacts.json` records
their exact native-library/APK hashes, signer certificate fingerprint, architecture
and declared Android versions. Compilation does not prove runtime behavior of a
new artifact, even when an earlier APK passed emulator tests.

## Choose and install an artifact

Use `android-arm64-v8a-debug.apk` for an ARM64 device and `android-x86_64-debug.apk`
for an x86_64 emulator. The declared minimum is Android 7/API 24, target API 36;
this is a manifest declaration, not proof of support on every declared OS version.
The observed previous runtime baseline is Android 15/API 35 x86_64. These debug
builds retain native symbols and are large. They are debuggable and allow Android
cleartext traffic for development; the native remote-control connection still
requires verified HTTPS. Do not use this build as a hardened production client.

Verify `sha256sum -c SHA256SUMS` before installing. Review the exact device/serial;
installation is an explicit operator action, never performed by the packager.
With Android platform tools and a selected development device:

```sh
adb devices
adb -s <selected-serial> install /absolute/path/to/the-selected-debug.apk
```

You can also copy the appropriate APK to your chosen device and explicitly permit
installation from that source. The debug signing certificate belongs to the owned
build prefix, not a personal release identity. A historical build may have another
debug signer; Android will refuse an in-place update in that case. Do not silently
uninstall the existing app: uninstalling removes local identities/trust/settings.
Review that data and the device before choosing a clean installation. Keep the
owned debug signing cache for future development updates; never commit or share
its private keystore. A release signing/distribution process remains separate.

## Connect and test allocation -> SSH

Start a reviewed control plane behind verified HTTPS; the companion Linux package
includes `INSTALL.md` for service/TLS/provider preparation. Enter the server HTTPS
address and access token in the app's remote mode. For a private CA, enter only
its public certificate obtained through a trusted channel. Tokens are not restored
automatically after disconnect/closing. Never disable certificate/hostname checks.

Load the workspace. The server needs a reviewed Debian template catalog, correct
least-privilege Proxmox configuration and explicit provisioning enablement before
creation can run. Choose a concretely approved disposable node/storage/network/
VMID; do not use a real VM merely because credentials exist. Generate/select the
application identity, review its public key and the definition/target, then explicitly
confirm creation. Keep the operation ID if the app disconnects; read saved progress
rather than submitting a new creation with another key.

The create form offers optional Debian packages, one per line, and an initially
unchecked final disk-only snapshot. Review exact package names/versions and snapshot
intent before confirming; editing clears consent and requires a fresh preview.
A final snapshot requires the independent server snapshot enablement as well as
provisioning. Recreation uses the packages/snapshot declaration saved with the
retained definition; it does not restore previous disk contents or old snapshots.

Use **Read recorded steps** in creation/recreation or saved-operation recovery to
inspect ordered stage statuses, including packages and the final snapshot. This
GET-only view does not check current guest state or authorize another execution.
A dispatch marker does not prove completion. Older servers can leave this optional
view unavailable; keep the operation ID and refresh the saved operation.

Wait for the complete operation to succeed: Cloud-Init availability alone does not
prove package installation or final-snapshot completion. Failed or uncertain work
retains the target for inspection. Refresh the known operation; do not start a new
creation to work around a lost reply. These are snapshots, not independent backups,
and unpinned packages/dependencies can change with the configured guest repositories.

After successful provisioning, prepare SSH to the new address. Independently verify the
host-key fingerprint before trusting it, unlock the chosen identity and test the
terminal. The mobile device must have a route to the guest's actual SSH address
(e.g. the documented LAN/VPN); a reachable control plane alone does not relay SSH.
A generated-provider test address such as 192.0.2.42 is not a real routable guest.

Record the artifact hash, device/OS/WebView, server version, exact approved target
and results of verified HTTPS, creation/readiness, SSH trust/authentication, input,
resize, disconnect/reconnect and rotation/backgrounding. Never include tokens or
private keys in a report. Current artifact launch and real infrastructure testing
remain independent gates until that evidence exists.

## Rebuild and package

Use an owned JDK 21/SDK 36/build-tools 36.0.0/NDK 28.2.13676358 prefix and isolated
ANDROID_USER_HOME/GRADLE_USER_HOME. Keep SDK/signing caches outside the repository;
do not load personal signing keys or override HOME. The checked-in Android project
must not be regenerated over its manifest/activity/Gradle files.

```sh
npm --prefix apps/client run tauri -- android build --debug --apk \
  --target x86_64 aarch64 --split-per-abi --ci
python3 scripts/package-android.py --sdk /absolute/path/to/owned/sdk
```

The packager checks signatures and manifest/ABI, compares the embedded native
library with the current target build. It also checks the compiled manifest for
backup exclusions and absence of the unused file-sharing provider/path resource
(ADR 0073). It copies only the two APKs, public
metadata/checksums and these instructions to `.lab/packages/android/`. It installs
nothing. Historical APKs that differ from the current native build are refused.
See `experiments/android-native/README.md` for isolated emulator/runtime checks.
