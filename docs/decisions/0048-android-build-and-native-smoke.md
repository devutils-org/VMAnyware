# 0048 — Android debug packages and actual native bridge evidence

Date: 2026-10-18
Status: ARM64/x86_64 debug APKs built; Android 15 x86_64 native smoke passes.

Use Android first provisionally on the available Linux/KVM host. The unanswered
platform preference does not remove iOS from the roadmap. Keep the generated
Android project's editable source/Gradle wrapper in Git, preserving manifest,
activity and tooling inputs; keep generated Kotlin, local paths, signing material,
JNI libraries, schemas and build outputs ignored. No new runtime crate is needed.
Compile/target SDK 36, minimum API 24 is configuration only, not proof on API 24.

Build real separate ARM64/x86_64 debug APKs containing the existing create/recovery,
identity and SSH/terminal code. Debug signing is owned ephemeral development
material. It is not a release signature or a physical-phone test. Reuse current
verified frontend assets during native-only iteration; run no unrelated suites.

Set explicit no-backup and extraction exclusions for app data. Android documents
that allowBackup alone may not exclude device transfers, so configure both modes.
See [Android backup guidance](https://developer.android.com/identity/data/autobackup).
These are packaged rules; no OEM backup/transfer execution is claimed.

Apply system-bar/cutout layout natively, with a matching dark window backdrop and
readable bar icons. Forward handled inset types as zero while preserving keyboard
notifications, following [WebView inset guidance](https://developer.android.com/develop/ui/views/layout/webapps/understand-window-insets).
A first root-padding attempt produced an unpainted native surface despite a mounted
DOM and working IPC. The revised configuration passes a cold-launch screenshot.
The smoke now also inspects the actual framebuffer, avoiding DOM-only visual claims.
Keyboard behavior and modern WebView parity still require separate tests.

Use only a named disposable emulator with its own SDK/user/AVD/Gradle prefix,
ADB server port and gRPC authentication. Require its name/QEMU property before
resetting app data. Actual WebView CDP invokes the production Tauri bridge; no fake
IPC or production fixture flag is added. Generate a disposable encrypted identity,
reject a wrong phrase/second create, force-stop/relaunch and compare the reopened
public identity. Verify invalid endpoint/session guards without network access.

Evidence: both native architectures compile and package; APK signature, native ABI,
bundled frontend/terminal assets and backup manifest/resources are inspected. Android
15/API 35 x86_64 (WebView 124.0.6367.219) passes actual bridge/identity/restart/guard,
launch layout and framebuffer smoke. Screenshot and report are in ignored .lab.
No real provider, guest/VM, personal credential, physical device, Android SSH/PTY,
keyboard, suspension, release or iOS support follows. Next: actual Android SSH/PTY
with the app key and strict host trust against a disposable local SSH fixture, then
secure remote create-to-SSH integration and an approved real disposable VM gate.
