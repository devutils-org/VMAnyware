# 0070 — Current Android development artifacts

Status: accepted for current ARM64/x86_64 build and packaging; fresh runtime open.

Deliver the current application as two debug-signed, single-ABI APKs with public
metadata/checksums and manual testing instructions. These are development builds,
not a release candidate, Play Store distribution or universal minimum-OS claim.
Native debug symbols remain, so the files are large. The declared API 24 minimum
and API 36 target are verified from each actual manifest, not presented as runtime
acceptance across that range.

Tooling is restored under an owned persistent user prefix outside the repository.
JDK 21.0.2 and the SDK 36/build-tools 36.0.0/NDK 28.2.13676358 archives are verified
against official HTTPS repository metadata/published checksums. Extraction retains
NDK symlinks. ANDROID_USER_HOME/GRADLE_USER_HOME isolate generated development
signing/cache data; no personal signing key is read and HOME is not overridden.
Existing checked-in Android activity/manifest/Gradle files are not regenerated.

The normal Tauri build embeds a fresh TypeScript/Vite build and current Rust library
for both targets. The packager verifies each APK signature, actual package/version,
debug flag, declared SDK versions and single ABI. It compares the embedded library
SHA-256 with the current target build before copying the APK. This prevents an old
APK from being relabelled as current merely because an output file exists. Both
split variants must share a signer. Public artifact metadata records APK/native
hashes and signer certificate fingerprint; no private keystore enters the package.

The packager never selects a device or installs/uninstalls an app. A new isolated
debug signer can differ from an old APK; manual instructions preserve the operator's
choice about local app data rather than silently uninstalling. HTTPS/access token,
private public-CA input, explicitly approved VM target and strict SSH host trust
remain prerequisites for the real allocation/SSH journey. Control-plane access
does not provide a network route or automatic SSH relay to a guest.

Current build/signature/hash evidence does not inherit runtime evidence from an
older APK. `runtimeValidatedForTheseApks` stays false until an exact-artifact run
exists. Next: owned Android 15 x86_64 emulator launch and generated-provider native
create -> readiness -> verified SSH, followed by approved real infrastructure and
physical ARM64 acceptance. Release signing/packaging remains a separate gate.
