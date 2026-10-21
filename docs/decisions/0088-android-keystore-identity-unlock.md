# 0088 — Android Keystore protection for SSH unlock

Status: Rust ARM64/x86_64, Kotlin/JNI and debug APK packaging pass; Android runtime
and actual Keystore behavior are unverified. No platform acceptance is closed.

## Decision

Reuse the five explicit system-identity commands and existing host-trust-first SSH
selection on Android. Keep the SSH private key in its passphrase-encrypted app file.
Protect only the optional remembered unlock secret with an AES-256-GCM key generated
by AndroidKeyStore. This is not a non-exportable SSH key, StrongBox/TEE attestation,
biometric authorization or protection against a compromised unlocked app process.
No user-authentication requirement is configured on the wrapping key. Keep the
manual passphrase available as an explicitly selected recovery path.

The native scope is SHA-256 of a versioned purpose, canonical app-identity directory
and public fingerprint. Kotlin accepts only its 64 lowercase hexadecimal characters
and uses one fixed alias prefix. It never enumerates or imports keys. The scope is
also GCM associated data. Ciphertext has a fixed version byte, provider-generated
12-byte IV and 128-bit authentication tag, with strict size/version limits. Only
ciphertext is stored in private SharedPreferences with synchronous commit. Existing
cloud/device-transfer exclusions cover SharedPreferences as well as identity files.

Correct passphrase verification precedes save; native readback must match before
reporting success. Missing/invalidated keys behind retained ciphertext are not
silently regenerated. Explicit forget removes only this scope's wrapping key and
ciphertext, checks key absence and preserves the encrypted SSH key. A failed commit
is an error; an orphan scoped wrapping key can be reused on an explicit save or
removed by explicit forget. Forgetting does not revoke an already selected key or
terminate an existing SSH session. There is no automatic fallback, retry, key
rotation, credential migration or user-store discovery.

MainActivity initializes a narrow application-context helper alongside, without
changing, platform TLS verification. Rust retains the helper/JVM across activity
recreation, not the Activity. Startup does not open preferences or access keys.
Blocking native commands attach their worker thread for one scoped JNI call.
Retrieved plaintext stays native; Kotlin clears save input in finally and Rust
clears the returned Java array after copying into a zeroizing buffer. Full process,
JVM/provider or error-path memory erasure is not claimed. Errors are fixed, with
no Java exception text or raw ciphertext forwarded to the renderer.

System-store labels now describe Linux Secret Service and Android Keystore without
claiming desktop behavior on a phone. Save/forget consent and connection identity
consent remain separate, initially unchecked and cleared after actions. No new IPC
command, dependency, permission or network capability is added (82 commands).

## Actual evidence and remaining gates

A focused Linux-host test proves deterministic scope and independent directory/key
binding; it does not emulate Keystore. Both Android native targets, Kotlin and the
two signed debug APKs build. JNI descriptors match the compiled Kotlin methods;
the new initializer and unchanged TLS initializer are exported. Packaging checks
require the helper in DEX, exact native-library hashes and the existing signature,
manifest and backup constraints. React/Firefox simulated-IPC acceptance still passes.
STATE.md records the exact logs/artifact hashes and initial corrected compile errors.

No emulator/device, personal credential or real VM was accessed. Runtime acceptance
must exercise real save/read, process/activity restart, wrong scope/tampered blob,
missing/invalidated wrapping key, no silent replacement, explicit forget/read refusal
and actual SSH/PTY behind host verification. Device/OEM backup behavior and supported
security levels require independent evidence. Do not mark this implemented source
as Android runtime, hardware or release acceptance.

## References

- [Android Keystore](https://developer.android.com/privacy-and-security/keystore)
- [KeyGenParameterSpec](https://developer.android.com/reference/android/security/keystore/KeyGenParameterSpec)
- [JNI thread attachment](https://docs.rs/jni/0.22.4/jni/vm/struct.JavaVM.html)
