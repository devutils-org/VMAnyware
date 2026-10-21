# Roadmap evidence and remaining product work

Reviewed 2026-10-21 against VMAnyware.md, current source, STATE.md and the recorded
artifact checks. This is a readiness review, not a full completion audit or a new
application test run. No project-completion percentage is inferred from checkboxes,
source volume, test count or packaged files. The complete roadmap remains in scope.

## Product gaps to implement

| Priority | Missing behavior | Current source evidence | Next acceptance |
| --- | --- | --- | --- |
| 1 | Start/gracefully stop an existing VM through the authenticated remote profile | `RemoteConnection.tsx` exposes create/snapshot/restore/delete/recreate, and now a lifecycle component. Native remote preflight/clear/confirmed submission/status and React review/consent/status now exist (ADR 0074); actual Linux native GUI/API/worker/generated-provider integration and clean app/server GET-only recovery now pass; Android/real-target acceptance remains open. Existing `LifecycleActions.tsx` takes a local port and invokes `control_plane` commands. Those commands target loopback, not the retained remote HTTPS connection. | Retained native remote target/action/preflight review, explicit confirmation, same-key retry, fixed operation GET and matching-result validation; then remote UI and actual native generated-provider journey. |
| 2 | Inspect live runtime status/detail and initiate SSH for an existing remote VM | Explicit remote power/QMP read/refresh now passes native protocol, React and actual Linux HTTPS/provider integration with timestamp/staleness/target binding. Existing registered associations now offer manual SSH entry; actual Linux host-trust/application-key/PTY integration passes (ADR 0078). Android/real guest acceptance remains open; power does not establish readiness. Server observation/status and local client screens exist. | Explicit scoped read using the remote profile, unknown/stale state shown honestly, and independently reviewed host trust before SSH. Do not infer readiness from power state. |
| 3 | Execute declared package provisioning and optional post-provision snapshot | Fixed package transport/durable worker and optional final snapshot are implemented (ADRs 0079–0080). API admission now passes the full declared create/recreate combinations against generated HTTPS, with snapshot gate and exact new-clone tag (ADR 0081). Native declared-plan acceptance now passes protocol/recovery checks (ADR 0082); explicit inputs/reviews also build/pass React/browser checks (ADR 0083); joined Linux native declared creation/recreation now also pass with same-key retry and exact provider/SQLite stage evidence; refreshed server/Android package checks also pass, while exact-APK runtime remains open; actual guest packages/snapshot acceptance is unproven. | Native validation/form/review, safe failure messages and explicit recorded stage views pass protocol/browser and joined Linux journeys (ADRs 0082–0086). Refreshed server/Android artifacts pass packaging checks, and the extracted server passes the declared native journey. Next: actual approved guest/current-APK acceptance. |
| 4 | Platform-appropriate credential storage and platform parity | Application SSH identity remains passphrase-encrypted local storage. Optional Linux Secret Service unlock-secret storage now passes isolated real-daemon/native SSH and separate browser checks (ADR 0087); joined Linux WebView/production IPC/vault/terminal acceptance also passes; actual other-platform stores are next. Android Keystore wrapping of the unlock secret now compiles/packages for ARM64/x86_64 with fixed JNI methods and scope binding (ADR 0088); real Keystore/restart/SSH acceptance is pending. macOS Keychain integration is absent. Windows identity operations return unavailable; iOS build/runtime evidence is absent. | Platform implementation/decision with actual native acceptance on each claimed target; encrypted-file support must not be relabelled OS-backed storage. |
| 5 | Additional local hypervisor requested after the original MVP | ADR 0062 records the widened scope and a proposed seam, but no implementation. Original roadmap §9 exclusion does not cancel the recorded user request. | Establish hypervisor/first concrete target, then one approved allocation/SSH slice. No VM operation is authorized merely by this document. |

Priorities 1–3 now have source and Linux native integration evidence; mobile/current-APK
and real-target acceptance remain open. Rebuilding unchanged artifacts and adding more
variants of passing fixtures do not replace those acceptance gates.

## MVP journey (§8) evidence map

| Required result | Existing evidence / implementation | Still open |
| --- | --- | --- |
| Client connects to self-hosted control plane | Native verified HTTPS/auth/private-CA commands; historical Android generated-provider journey | Exact-current APK runtime and actual deployment acceptance |
| Secure control-plane → Proxmox connection | Verified TLS/fixed adapter routes; generated HTTPS tests | Supported real endpoint/version and least-privilege ACL acceptance |
| Create from supported YAML | Parser, canonical YAML/hash, saved definitions, native remote create UI | Real mapped template; package/automatic-snapshot declarations noted above |
| Clone/configure CPU, memory, disk | Production worker and generated-provider native journeys | Actual approved template/storage/target behavior |
| Cloud-Init SSH public key | Durable configuration/seed/readiness and generated-provider checks | Actual guest key/user/template attestation |
| Follow provisioning until ready | Retained status/failure reads, safe per-stage progress API and worker guest proof | Native per-stage display passes separate protocol/browser and joined Linux checks; actual readiness/guest failure and current-APK acceptance remain open |
| Start/stop | Server journal/executor and local native UI evidence | Remote native commands and React UI now pass separate protocol/simulated-IPC checks; actual Linux generated-provider native integration and clean restart/history recovery now pass; real-target/platform acceptance remains open |
| SSH from desktop/mobile | Historical macOS/Android actual native owned-shell evidence; strict host trust | Current APK, direct real guest/network, Linux terminal, Windows/iOS, keyboard/backgrounding/parity |
| Snapshot create/restore | Native generated-provider journeys and GET-only restart recovery | Actual supported storage and restored guest-state proof |
| Delete while retaining definition | Native generated-provider deletion/restart recovery, schema-14 retirement | Actual approved volume/target deletion and absence proof |
| Recreate retained definition | Native generated-provider creation/new identity and restart recovery | Actual new VM/readiness; previous disk contents are not restored |
| Errors/interruption recovery | Durable no-replay journals, scoped history/ID reads, lost-response retry and restart fixtures; native 429 retention | Real-provider uncertain-outcome reconciliation and current platform behavior |
| Main journey without Proxmox UI | Most source paths exist; generated-provider workflows exercise them separately | One actual approved complete real-guest journey and exact-current Android execution |
| Install/operate service | Linux release archive; staged install, auth/admission, restart, stopped backup/restore, schema14→15 and future-schema refusal | Actual target systemd/user/sandbox/TLS/configuration, distribution and upgrade matrix |

## Phase acceptance and other requirements

- **Phase 0:** remains open. Existing macOS and historical Android feasibility do
  not establish both mobile families, real snapshot guest-state restoration or
  all target environments. `docs/phase-0.md` retains the individual gates.
- **Phase 1:** definitions/operations/instances/snapshot models, validation,
  migration and deterministic serialization source exist. Several unchecked
  foundational rows lag implementation; do not equate their checkbox state with
  missing code or close them without their requirement-specific evidence review.
- **Phases 2–4:** provider/API/workers/persistence and most safety paths exist.
  Remote lifecycle/status, declared provisioning features and real-infrastructure
  acceptance above remain material. Partial failure visibility is not permission
  to blindly retry a provider mutation or automatically delete an uncertain VM.
- **Phase 5:** main source gaps are remote existing-VM controls/status and platform
  acceptance/storage. Saved history/recovery UI is implemented. Native Linux
  fixture success is not evidence of Windows/iOS or exact-current Android behavior.
- **Phase 6:** threat review, main-window command ACL, bounded admission, packaged
  backup/restore/migration checks and operator docs are present. Real proxy limits,
  systemd sandbox, ACLs, secret storage, release signing/installers and supported
  upgrade/platform matrix remain open. No release candidate may be tagged yet.
- **Definition/security/API contracts (§§4–6,10):** retain strict schema/resource
  validation, public-key-only YAML, fixed provider routes, TLS/SSH verification,
  distinct desired/observed/job state and destructive confirmation. The proposed
  API table is not proof all methods exist; safe definition update/deletion and
  combined deletion routes are not current supported flows. Resolve their intended
  scope explicitly before claiming the whole API surface complete.
- **Testing (§11):** mock/generated-provider, localhost OS-shell, browser IPC,
  actual native IPC and packaged-process checks prove different boundaries. None
  replaces the full create→ready→SSH→snapshot→modify→restore→delete→recreate real
  target journey. Evidence must identify artifact/platform/target and approval.

## Current artifacts and external gates

Current Android APKs/metadata live in `.lab/packages/android/`; public metadata
states runtimeValidatedForTheseApks=false and releaseCandidate=false. Current Linux
archive/companion checksum live in `.lab/packages/`. STATE.md records exact hashes
and logs. Do not transfer earlier binary runtime results to these artifacts.

Owned Android emulator creation/start/app reset approval is still pending for the
previously proposed target. Real VM writes need explicit concrete operation/target
approval. iOS/Windows/physical-device and deployed service acceptance need suitable
environments. These gates do not block implementing the remote lifecycle slice.

**Next slice:** expose safe per-stage provisioning progress across native
creation/recreation/recovery while retaining exact operation identity and no replay.
Fixed failure reasons now pass native/browser acceptance (ADR 0084).
Refresh delivery artifacts now passes for the integrated declared workflow; actual
Android execution still needs its concrete approved target. Keep complete real
homelab, platform, OS storage, release/deployment and local-provider scope open.
