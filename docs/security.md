# Security review and trust boundaries

Reviewed against the current source on 2026-10-21. This is a source/evidence review,
not a penetration test, deployment certification or release acceptance. Historical
runtime evidence in STATE.md applies only to its recorded platform and artifact.

## Assets, operators and attackers

Protect the Proxmox token, control-plane token, application SSH private key and
passphrase, trusted host keys, saved definitions and operation history. Terminal
contents and guest addresses may also contain sensitive operator data. Definitions
and operation approvals are integrity-sensitive: changing them can change the VM
or disks affected by a later operation. SQLite is not an encrypted secret vault.

The MVP trusts one operator, the installed native application, its OS account,
the server administrator, configured certificate authorities and the configured
Proxmox host/template. It does not implement tenant isolation or multiple roles.
A stolen control-plane bearer token grants the operator's API authority; write
flags and confirmation bindings are safety controls, not separate user identities.
An attacker controlling the operator OS, server OS or trusted frontend code is
outside these protections and can act with that process's authority.

Consider an untrusted website attempting localhost requests, a network attacker,
a compromised guest/provider sending malicious data, stolen backups/artifacts,
invalid or excessive API requests, and interruption after a mutation may have
been accepted. Never infer guest trust from a successful provider task alone.

## Boundary review

| Boundary | Current controls and source | Residual risk / evidence needed |
| --- | --- | --- |
| Website → loopback service | [Config](../server/src/config.rs) binds IPv4 loopback; no CORS layer. Mutation handlers reject browser Origin through their validation paths, e.g. [definitions](../server/src/definitions_api.rs), [provisioning](../server/src/provisioning_api.rs) and [recreation](../server/src/recreation_api.rs). [Authentication](../server/src/auth.rs) wraps all API routes when configured. | Absent token intentionally permits local development. CORS is not authentication and loopback is reachable by other local processes. The packaged unit requires a token; do not expose an unauthenticated development process through a proxy. |
| Native app → HTTPS proxy/server | [Remote profile](../apps/client/src-tauri/src/remote_connection.rs) validates HTTPS root endpoints, refuses URL credentials/query/fragment, disables redirects, automatic retries and environment proxies, marks authorization sensitive, sets deadlines, and validates authenticated-response markers. Explicit private CA selection retains certificate/hostname verification. | The user must establish endpoint/CA provenance independently. A trusted malicious server can provide false workspace data. Target HTTPS proxy/TLS configuration and exact-current Android runtime still need acceptance. |
| WebView → native commands | [Tauri configuration](../apps/client/src-tauri/tauri.conf.json) uses a restrictive script/connect CSP; [capability](../apps/client/src-tauri/capabilities/default.json) targets the local main window with explicit permissions for all 76 reviewed application commands plus core permissions. The build declares an application command manifest; only this capability is selected. Native command implementations validate input, retained scope and session state. | CSP and core permissions do not make a compromised authorized WebView safe: custom application commands are privileged. Tokens/passphrases pass through the input UI and IPC. Do not load remote frontend code or claim that secrets never enter the WebView. Actual Linux two-window IPC now verifies main admission and other-window refusal (ADR 0072); other-platform/release-WebView hardening remains a gate. |
| Server → Proxmox | [Adapter](../server/src/proxmox.rs) validates HTTPS endpoint/token shape, verifies certificates, disables redirects/proxies/automatic retries, uses bounded responses and deadlines, and exposes fixed operations rather than an arbitrary URL proxy. | The configured provider is trusted infrastructure. Endpoint validation is not a sandbox against a malicious administrator configuring an internal address. Real least-privilege ACLs, template provenance, storage behavior and provider-version compatibility need target evidence. |
| Confirmation → queued provider write | Approval-bound plans and idempotency journals persist exact target/definition/scope. [Lifecycle](../server/src/lifecycle_executor.rs), [provisioning](../server/src/provisioning_executor.rs), [snapshot](../server/src/snapshot_executor.rs), [restore](../server/src/restore_executor.rs) and [deletion](../server/src/deletion_executor.rs) executors revalidate intent; independent write flags default off. Recovery reads do not reinstall consent. | A preflight is not a reservation. Unknown outcomes need saved evidence and explicit reconciliation, never a new-key blind retry. Generated-provider tests cannot prove real disk/guest state or safety on an unapproved target. |
| App → SSH guest | [Host trust](../apps/client/src-tauri/src/ssh/trust.rs), [host inspection](../apps/client/src-tauri/src/ssh/host.rs) and [sessions](../apps/client/src-tauri/src/ssh/session.rs) bind endpoint/public-key observations, require explicit first-use trust and reject changed keys. Session input/output and transport queues are bounded; see [session policy](ssh-sessions.md). | First-use confirmation needs an independently verified fingerprint. Provider readiness/address is not host authentication. A trusted guest can still display deceptive terminal content or receive anything typed into its shell. |
| SSH identity → app storage | [Identity store](../apps/client/src-tauri/src/ssh/identity.rs) uses app-owned encrypted Ed25519 keys, AES-256-GCM and bounded bcrypt policy, private Unix metadata checks, no-clobber publication, bounded file reads and zeroizing passphrase wrappers. Private keys are not sent to the control plane. | This is passphrase-encrypted application storage, not demonstrated OS Keychain/Android Keystore integration. It does not protect against a compromised unlocked process; full memory erasure is not claimed. The Android manifest already excludes cloud/device-transfer data; the unused broad FileProvider and its path resource are now removed and checked in packaged APKs (ADR 0073). Other-platform permissions, actual backup behavior and recovery need acceptance. |
| HTTP requests → service resources | [Admission](../server/src/request_limits.rs) bounds burst/refill and concurrent handlers before authentication; route body limits restrict YAML/mutation requests. No client-controlled quota map is allocated. | Global unauthenticated exhaustion affects the operator. Admission ends at response headers and does not bound TCP connections or slow header/body delivery. Proxy time/connection/body limits and deployment load acceptance remain open. |
| Process → filesystem/logs | [Service unit](../packaging/server/vmanyware.service) proposes a dedicated account, restrictive umask/state directory and systemd hardening. [Diagnostics](../server/src/diagnostics.rs) emits allowlisted route/status events, not arbitrary request URLs/headers/bodies. Authentication/adapter Debug and errors redact tokens. | Real service-manager sandbox acceptance is open. Auth/admission rejections happen outside route diagnostics, so existing request logs are not a complete security audit trail. OS/proxy logs and backups need their own access/retention policy; do not log terminal content or credentials. |
| Build artifacts → operator | [Server](../scripts/package-server.py) allowlists files; [Android](../scripts/package-android.py) verifies signatures/ABI/manifest/current native hashes. Companion checksums detect corruption. | Checksums are not publisher authentication. Android artifacts are debug-signed, debuggable and large; no release-signing/update provenance or hardened-release claim. A checksum from an untrusted source does not establish trust. |

## Evidence inspected and verification boundaries

The review inspected implementation and existing test sources rather than rerunning
unmodified suites. Relevant contracts are in [auth tests](../server/tests/auth.rs),
[provider tests](../server/src/proxmox/tests.rs), [diagnostics tests](../server/tests/diagnostics.rs),
[native HTTPS tests](../apps/client/src-tauri/src/remote_connection.rs) and the
SSH modules above. These test sources explain intended coverage; their presence
alone is not a new passing execution claim. STATE.md records actual earlier runs.

The latest admission slice passed three focused middleware tests, four affected
auth tests and an actual debug-process burst/refill check. The refreshed optimized
server archive additionally passed exact-package staging, restart/persistence,
authentication and admission/refill checks. See ADRs [0071](decisions/0071-process-wide-request-admission.md)
and [0069](decisions/0069-linux-server-package.md). No provider credentials or
private key files were inspected for this review. Documentation changes require
link/content/diff review, not another application test run.

## Remaining work before release

1. Validate the exact Android artifact and the real approved allocation → SSH
   target journey; retain artifact, platform and target evidence separately.
2. Establish and test deployment proxy/TLS/timeout/connection limits, the systemd
   sandbox, token revocation/rotation procedure and actual least-privilege ACLs.
   The current static bearer has no expiry or per-user revocation; replacing it
   requires changing server configuration/restarting and reconnecting clients.
3. Complete platform secret-storage decisions and acceptance, including OS-backed
   facilities where supported. Verify mobile backgrounding, software keyboard,
   backup exclusions and lost-device recovery without importing private user keys.
4. Keep the reviewed native command manifest/grants aligned and validate the window
   boundary on each supported platform. Complete release CSP/permissions and secure
   distribution/signing/update provenance on each supported platform.
5. Test upgrade/restore procedures against the current schema and retained journal.
   Protect database/configuration backups as sensitive integrity-bearing data;
   follow [installation and recovery](install-server.md), not raw live-file copies.

This review completes the roadmap's threat-model review task only. It does not
close the separate security, deployment, platform or release-candidate gates.
