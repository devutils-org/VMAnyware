# 0089 — Separate Android Keystore acceptance artifacts

Status: test APK and read-only plan compile/verify; runtime execution is not authorized
or demonstrated by this slice.

## Decision

Prepare a separate test-only instrumented APK to exercise actual Android Keystore
negative cases, alongside the existing packaged production IPC driver. Do not add
corruption, alias enumeration, key deletion or credential-file hooks to production
commands. The test requires an explicit owned-emulator marker, QEMU hardware and
the expected application package before creating scopes. Every scope is random,
fixture-owned and cleaned up individually. No ordinary application alias is read
or enumerated. Four instrumented tests cover authenticated roundtrip/forget,
ciphertext/tag corruption and wrong scope, wrapping-key loss without silent
regeneration, and malformed scope/version/size with save-input clearing.

The existing native driver gains a Keystore mode for the same generated app identity
before/after force-stop/relaunch. It verifies save/forget confirmation, wrong-secret
refusal, same public identity, successful vault unlock after process restart,
forget/read refusal and unchanged manual unlock. The installed standalone APK must
match the reviewed local SHA-256 before app reset. Three host-only fake-ADB tests
prove early refusal; they do not emulate Android or Keystore.

A read-only planner uses SDK apksigner/aapt2 and streams APK digests. It verifies
package/ABI/test runner/test-only status and matching main/test signers, then records
exact artifacts, host-controller hash, fixed owned target and operations. Runtime
execution requires explicit opt-in, recomputes the plan before any ADB call and
refuses a different AVD, physical device, unbooted target or ABI. It installs only
reviewed APKs, resets only the owned application, requires exact native/report and
four-case success, stops the app and removes the test APK it installed. It never
starts a VM. VM creation/start requires its own concrete approval (AGENTS.md).

Use AndroidX JUnit 1.3.0 and runner 1.7.0 only in androidTest. They replace the unused
generated test dependencies; no application runtime dependency or permission is
added. The test APK explicitly requires test installation and disables backup.
Targeted test packaging reuses already verified unchanged production native
libraries, excluding the Rust task whose invocation requires a live Tauri CLI
options server. Kotlin/test APK compilation is real; cryptographic runtime proof
is still absent. STATE.md records actual build/plan/host-check evidence and hashes.

## Limits

No ADB/device/emulator, credential mutation, real VM or host service is used while
preparing the plan. Android runtime, system-source SSH/activity recreation, physical
security level, biometric and OEM backup gates remain open. Test-scoped acceptance
must not change full platform/release status. The named disposable AVD and both APK
operations must be approved before running; do not infer approval from available
SDK files or from successful compilation.

[AndroidX Test releases](https://developer.android.com/jetpack/androidx/releases/test)
provide the pinned test-library versions. Existing app TLS/SSH verification remains
unchanged.
