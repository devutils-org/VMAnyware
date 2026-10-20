# 0050 — Android HTTPS verifier and connection-scoped private CA

Status: implemented; current native verification recorded in STATE.md.

## Context

The first authenticated Android connection exposed missing platform-verifier
integration. Reqwest 0.13 uses rustls-platform-verifier: Android needs its Kotlin
component and explicit JVM/application-context initialization before networking.
An APK that compiles and opens SSH does not prove this HTTPS path works.

The self-managed control plane may use a private certificate authority. Relying
only on publicly trusted certificates prevents that working homelab use case.
Changing the device's trust store is unnecessary for a single server connection.

## Decision

MainActivity synchronously initializes rustls-platform-verifier 0.7.1 before
starting Tauri, retaining the application context rather than an Activity.
Android-only direct JNI/verifier dependencies reuse versions already locked
transitively. Gradle reads the Kotlin component version from Cargo.lock and
includes its upstream Maven artifact. ProGuard retains JNI-reached classes and
the initialization entry point. Normal connections retain platform verification.
Initialization failure stops activity startup rather than falling back to an
unverified transport.

The connection form accepts one optional public PEM CA certificate, at most
16 KiB. Native validation rejects private-key markers, malformed input, bundles
and oversized input. The CA is retained only in the current native profile and
in the mounted form; disconnect/restart clears it. Tokens remain memory-only and
are cleared from the form on submission. No file picker, private key handling or
persistent connection configuration is added.

With an explicit CA, trust is limited to that CA for this profile on every
platform. Use reqwest's tls_certs_only with normal WebPKI verification: chain,
hostname and certificate validity checks remain enabled. Android's platform
verifier does not support merging extra roots, so a merge would fail when
building the client. Device/global trust is unchanged. The UI explains this and
requires the operator to obtain the public CA through a trusted channel.
There is no accept-any-certificate or hostname-verification bypass.

## Verification boundary

The owned Android harness runs the packaged app's real IPC against the production
control-plane binary with a disposable database and a generated HTTPS proxy.
An explicit ADB reverse tunnel handles the currently unavailable host-alias route.
Seeded failed operation data tests read-only recovery; it is not provisioning or
real guest evidence. No provider is configured and the proxy rejects non-GET
requests. Verify rejected issuers/hostnames before HTTP, invalid bearer tokens,
connection/restart/disconnect and GET-only recovery independently of full create.

The Linux native test build still requires the documented system WebKitGTK
prerequisites. Android evidence must not be reported as a passing Linux suite,
physical-device test or deployed/direct-network validation.

## References

- [Verifier Android integration](https://docs.rs/rustls-platform-verifier/0.7.1/rustls_platform_verifier/android/index.html)
- [Upstream setup and Kotlin/JNI requirements](https://github.com/rustls/rustls-platform-verifier)
- [Reqwest certificate roots](https://docs.rs/reqwest/0.13.5/reqwest/struct.ClientBuilder.html#method.tls_certs_only)
