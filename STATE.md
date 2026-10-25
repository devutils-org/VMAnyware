# VMAnyware — current state

Updated: 2026-10-25. Historical evidence and decisions are in Git and docs.

Naming note: historical lab resource identifiers below are normalized to the
current VMAnyware name. External resources were not renamed; use VMIDs and
ignored local lab records to verify actual identifiers before operations.

## 2026-10-25 — User-requested session pause

User explicitly requested stopping the session and resuming later. Preserve the
full goal: finish Proxmox, then complete the roadmap. Current work branch is
codex/home-proxmox-ios; published implementation/evidence through 495a1f7.

New user-run home-host observations supersede the missing-network-input portion
of the older blocker: no UDP port-67 listener; dnsmasq, kea-dhcp4-server and
isc-dhcp-server inactive; net.ipv4.ip_forward=1. This does not prove working NAT,
DNS or guest reachability. qm guest cmd 100 network-get-interfaces reports the
QEMU guest agent is not running. Subsequent ip/address/route output was still
from the Proxmox host (wlo1=192.168.1.36, vmbr0=10.10.10.1), not the guest.
The VM 100 address remains unknown. Do not install an agent or change VM 100.

Next resume: ask the user to open VM 100's Console in Proxmox and report
ip -brief -4 address / ip route from inside Debian, and confirm preservation of
100 plus reservation of VMIDs 9000–9010. Reservation is still unconfirmed and
no home-server VM/network/storage/credential mutations are approved. Then prepare
an exact DHCP/template/network/ACL proposal for approval before executing.
Do not re-request already supplied host DHCP/forwarding observations.

Completed earlier: verified home read-only Rust API probe; scoped privilege probe
with passing local/hosted checks; actual unsigned iOS simulator application build
and downloaded/hash-verified artifact (run 40144618222). iOS runtime, real home
provisioning/SSH/snapshot/restore/delete/recreate, remaining platform/deployment
and release acceptance stay open. No tests or infrastructure operations run in
this documentation-only pause checkpoint; diff review is the check.

## 2026-10-25 — Proxmox acceptance blocked awaiting target inputs

The same home-server prerequisite/approval blocker has persisted across more than
three consecutive goal turns without a user response. Revalidation: workspace is
clean before this checkpoint; all known CI runs are terminal, including successful
40145053948 and later docs-only 40145312892. No live operation remains to observe.
The previous goal turn produced no source/runtime progress. Do not rerun unchanged
checks or create more preparation variants to simulate progress.

To resume the requested Proxmox-first sequence, obtain (1) confirmation that VM 100
is preserved and disposable VMIDs 9000–9010 may be reserved, and (2) user-run
read-only results for DHCP listeners/services and net.ipv4.ip_forward. No agent
has a configured approved authenticated execution channel to inspect those host
settings. The token supplied by the user is not available in the agent environment;
do not inspect local credential files or assume user-shell exports are inherited.
Then prepare exact template/network/ACL operations and obtain explicit approval
under AGENTS.md before any host/VM/credential mutation. Broad roadmap goal text is
not concrete target/operation approval. Full roadmap remains incomplete.

## 2026-10-25 — Scoped privilege probe CI verified

Exact Local checks run 40145053948 completed/successful for b1d4ac7. Selector and
server jobs passed (format, strict Clippy, server/local TLS/process tests); frontend
was intentionally skipped. Full log inspected. No run remains live for this slice.
Public API schema comparison of clone response, Cloud-Init regeneration response
and guest-agent exec/status shapes found no demonstrated transport mismatch; this
is source review only, not installed-version or real-guest validation. No code
change was justified by this comparison. Previous goal turn made source progress;
this turn observed the same queued/live CI through its terminal successful result.

Next real Proxmox action remains the pending DHCP/forwarding observations and
confirmation of disposable VMIDs, followed by concrete mutation approval. No VM
or credential operation is authorized by goal continuation. The rest of the
roadmap still includes Windows SSH/identity implementation, platform runtime,
real deployment and release gates; none is claimed complete by this CI success.

## 2026-10-25 — Scoped read-only token privilege probe

Scope/acceptance: add optional effective-VM-privilege observation to the existing
live probe, needed to verify dedicated-token grants before real provisioning.
VMANYWARE_PROXMOX_PROBE_VMID selects one canonical VMID; invalid selection fails
before network/credential use. Without it the probe retains its previous reads.
New vm_privileges performs fixed GET access/permissions at /vms/<selected ID>,
validates scope/privilege-name/propagation shapes, and returns sorted names only.
Zero/false propagation values are grants. No ACL modification, arbitrary API
passthrough or production behavior change. Empty grants do not establish free IDs.

Verification: two generated-TLS tests pass (exact selected GET/nonpropagating
privileges; malformed scope/values/name refusal, invalid ID with zero requests,
empty grants). Example cargo check, scoped library/test/example strict Clippy,
server formatting and diff review pass. No unrelated application suites rerun.
No live Proxmox request, credential file read or VM mutation occurred. Next: use
this opt-in check on the explicitly selected disposable target after reservation
confirmation; DHCP/forwarding observations and write approvals remain pending.
The full roadmap is still incomplete; iOS artifact evidence remains as below.

## 2026-10-25 — Downloadable exact iOS simulator artifact verified

Run 40144618222 completed/successful for debed94, including archive digest check
and upload. Artifact 12579559677 (vmanyware-ios-simulator-arm64) downloaded to
ignored .lab/packages/ios/run-40144618222. Independent SHA-256 check passes:
a94ac374b698a6938907caee9ec0e2456a160a3a1532973201a20d10d639a497.
Source metadata matches debed94bef076bcaae7fd3b9940dc0b8827baacc. Archive paths
are contained within VMAnyware.app; executable mode retained; file identifies
ARM64 Mach-O. Plist identifier dev.vmanyware.client, platform iPhoneSimulator,
minimum OS declaration 15.0. No runtime/minimum-OS compatibility claim. Initial
local archive-path assertion rejected the root directory name without slash;
corrected the inspection predicate and reran successfully, no artifact changed.
Local checks 40144618128 also completed/successful. No simulator launched,
credential or Proxmox change. Diff review passes for documentation updates.

Next: obtain concrete runtime-target approval for an exact-artifact iOS simulator
launch when full Xcode is available, while home Proxmox DHCP/routing and target
reservation responses remain pending. Full roadmap remains incomplete.

## 2026-10-25 — Retain exact unsigned iOS bundle for acceptance

Scope: extend the proven iOS workflow to tar the generated application (preserving
bundle modes/symlinks), verify SHA-256 and include source commit metadata, then
upload with the official upload-artifact v4 action pinned to its resolved commit.
Retention is 14 days and missing output fails the job. Workflow YAML/diff reviewed;
actual upload/download remains pending the affected CI run. No simulator launch,
credential operation or real Proxmox mutation is included. Next: observe upload,
download and independently check the exact archive/hash/bundle structure before
preparing any separately approved runtime plan.

## 2026-10-25 — Actual iOS simulator application compilation succeeded

Exact run 40144274220 completed/successful for e4a39c9. Full log inspected:
`Finished 1 iOS Bundle` at gen/apple/build/arm64-sim/VMAnyware.app. Environment:
macOS 15.7.9 ARM64, Xcode 16.4 (16F6), simulator SDK 18.5, CocoaPods 1.17.0,
Rust 1.99.1. Generated project and actual unsigned application build succeeded.
No simulator boot, installation, app launch, signing or secure-store operation.
App was not uploaded as a downloadable artifact; no runtime proof is inferred.
Local checks 40144274136 also completed/successful: selector, frontend and server
jobs all ran and passed. No local application suite was repeated.

Updated platform evidence table and detailed iOS section; documentation diff
review passes. Previous turn was verified progress (workflow push and live CI).
Next: retain the unsigned app as a CI artifact for later exact-artifact simulator
acceptance, without launching a simulator; real Proxmox prerequisite responses
and target/operation approval remain pending. Full roadmap is incomplete.

## 2026-10-25 — iOS workflow published and running

Prepared changes committed on codex/home-proxmox-ios: 29fca10 (home acceptance
and corrected Proxmox guidance), e4a39c9 (iOS workflow/platform evidence). Push
succeeded. Exact iOS run 40144274220 is confirmed in progress for e4a39c9,
with checkout/setup-node succeeded and Apple environment recording in progress.
Local checks run 40144274136 also exists/in progress. No result is inferred.
Continue observing these exact handles, do not restart on a polling timeout.
No app/simulator launch, signing or Proxmox mutation occurred. Next: inspect iOS
run results and fix demonstrated failures; full roadmap remains incomplete.

## 2026-10-25 — Revalidate hosted CI before iOS execution

Read-only GitHub inspection confirms Local checks 40031022048 is terminal/cancelled,
not live. Later 40031373475 succeeded for 63a7041, but only its changes selector
ran; server/frontend were skipped. Do not treat this docs-only run as fresh
application evidence. Windows run 40030829324 remains confirmed successful for
00439c4. Repository now resolves to devutils-org/VMAnyware via the old remote.
Next: push the prepared unsigned iOS workflow on a work branch and observe its
exact run. No simulator launch or credential operation is included.

## 2026-10-25 — Prepare actual unsigned iOS compilation gate

Scope: add the missing iOS source CI using the existing pinned actions and
macos-15 runner, locked dependencies, generated Xcode project and unsigned
Apple Silicon simulator build. Official CLI docs and exact CLI 2.12.3 upstream
build source confirm no-sign and forwarded Cargo arguments. Local Mac has only
Command Line Tools selected, no working xcodebuild/CocoaPods; no local application
build is claimed. Workflow syntax/diff review is the preparation check. No CI
execution is claimed until pushed/observed. No simulator/device was started,
installed or contacted, no signing or credential-store action occurred.

Proxmox runtime remains dependent on pending DHCP/forwarding observations and
explicit disposable-ID/operation approval. Previous turn made documentation
progress, not infrastructure progress. Next independent step: execute/observe
the iOS compilation gate and fix actual compiler failures; home Proxmox acceptance
remains the first real-infrastructure task once its prerequisites are available.

## 2026-10-25 — Correct current Proxmox preparation guidance

Scope: remove obsolete current-status claims in the Debian template and provisioning
plan guides, while retaining explicitly historical implementation milestones.
Verified current route /api/v1/provisioning/create, production worker stage calls
and RemoteCreate.tsx source. Guides now distinguish implemented opt-in execution
from pending real-template acceptance, and spell out required guest-agent and
image-default-user attestation preparation. The basic import/catalog example is
not advertised as sufficient for confirmed creation. Preview remains read-only.

Verification: documentation diff/whitespace review only; no application tests,
credentials, VM, network or ACL operations. Existing uncommitted home acceptance
plan and earlier STATE changes remain separate prior work. Next: collect pending
DHCP/forwarding observations and disposable-ID confirmation, then prepare concrete
home template/network changes. Full roadmap goal remains active/incomplete.

## 2026-10-25 — Full roadmap goal resumed, home acceptance preparation

User requests completion of Proxmox work followed by the full roadmap. Previous
turn made progress by recording actual user-run read-only probe evidence; it did
not authorize VM writes. Prepared docs/home-proxmox-acceptance.md with exact known
target, preservation boundary, proposed IDs and required real application proofs.
Source inspection confirms production Cloud-Init currently requires DHCP and
readiness requires QEMU guest-agent execution/network support. Home bridge DHCP,
forwarding, guest reachability and template prerequisites remain unverified.
Do not claim a manual static clone proves the application provisioning workflow.

Verification: documentation diff review only, no live access or credential reads.
Public official API schema review also identifies required guest-agent unrestricted
execution for readiness, root-path Sys.Audit for the current HA deletion preflight,
and adapter-required Datastore.Allocate for complete post-delete volume inventory.
Recorded these boundaries in the home acceptance plan; they are not installed-
version minimum-role proof. No ACLs were changed. Do not promise complete workflow
isolation using VM-only grants on shared storage.
Next: obtain ID reservation response, inspect existing DHCP/routing without secret
configuration dumps, then prepare concrete template/network/ACL changes for approval.
The full objective remains active; all real guest and remaining platform gates stay
open. Do not mark completion based on existing generated-provider tests.

## 2026-10-25 — Home Proxmox read-only connection proven

Scope/acceptance: record the user-executed Rust probe against the selected home
endpoint; documentation only. User reports successful verified-TLS curl reaching
HTTP 401, then successful proxmox_probe output: Proxmox 9.2.21 (release 9.2),
node debian-btw online. Endpoint is https://192.168.1.36:8006; the public cluster
CA is configured locally. User confirmed creating the dedicated API token after
the privilege-separated setup instructions. Exact ACLs/expiry were not inspected.
No templates were printed; this proves only token-visible inventory, not absence
of resources. No agent read credential files or executed infrastructure requests.

User-provided host inventory shows running Debian VM 100, no templates, local
and local-lvm storage, and private bridge vmbr0 (10.10.10.1/24) behind the host's
Wi-Fi LAN connection. Guest routing/readiness and effective NAT remain unverified.
VM 100 remains outside the mutation scope. Disposable VMIDs and all VM mutations
remain unapproved. Do not copy pasted network configuration into repository files.

Next task: confirm preservation of VM 100 and a reserved disposable VMID range,
then prepare a concrete separate Debian Cloud-Init template/access plan for review.
Existing source/runtime/platform gates below remain open. Verification for this
slice is documentation diff review only; the live probe evidence is user-reported.

## 2026-10-25 — Resume review and home Proxmox onboarding

Scope/acceptance: review the latest checkpoint and document the next real-target
access prerequisites; no application or infrastructure changes. Workspace was clean
at review. User reports a new home Proxmox server; endpoint, version, node,
template, storage, network and disposable VMIDs are not supplied yet. No access,
credential creation or VM mutation is authorized by this report. Existing lab
credentials must not be assumed valid or reused for the home server.

Next task: obtain non-secret target metadata and a concrete read-only scope,
prepare dedicated privilege-separated token ACLs and verified TLS configuration,
then run the opt-in inventory probe only after the target is explicitly selected.
Token secrets must remain outside chat and repository output; agents must not
inspect local credential files. Later clone/start/snapshot/restore/delete tests
require concrete target/operation approval. Latest CI result and platform runtime
claims remain as recorded below; no CI observation or application tests ran in
this documentation-only review.

## Latest resume point

Goal resumed 2026-10-23. Android Keystore production source and both debug APKs
compile/package (ADR 0088). ADR 0089 now prepares a separate test-only instrumented
APK (four real-Keystore cases), an exact-artifact read-only plan and the packaged
IPC save/read/restart/forget scenario. Three host-only fake-ADB refusal checks pass.
The test APK compiles; signatures, target/runner/test-only metadata and matching
main/test signer verify. No runtime case has been executed. No emulator was created,
started, installed/reset or contacted; no personal credential was read/changed.

Reviewable plan: `.lab/android-keystore/plan.json`, SHA-256
`47d7cc257434b038f7be1b62b8f8f5f4635565ef20acb63e9bd976c41b4798bf`.
It requires the named already booted x86_64 QEMU AVD, serial emulator-5580, ADB 5047,
installs only the two hashed APKs and resets only dev.vmanyware.client. Proposed
creation/start uses a fresh owned API-35/Pixel-6 AVD in the task-local Android user
directory, 2 vCPUs/2048 MiB RAM/2 GiB data. These VM/app operations still require
explicit concrete approval under AGENTS.md. No live task handle remains pending.
Next: obtain that approval, then execute the exact plan and record actual runtime
results; otherwise continue other source work without starting a VM.

Goal resumed: do everything feasible in this environment. Latest push ed05c64
passed frontend, formatting and server Clippy in CI 40029310966, but one server
unit test failed (177 passed). The snapshot foreign-key migration test relabeled
an already-current database as older, causing schema 14 to recreate an existing
index. It now builds separate real schema fixtures for fresh/10/11/12/13/14 using
the committed migrations, then checks version 15, foreign-key enforcement,
foreign_key_check and orphan rejection. Production migration code is unchanged.
The focused test passes (one test, six fixture versions). Full native macOS build
and scope-only test remain proven by CI 40028754564 for 26eb8a2; real Keychain/GUI
runtime remains unproven.

Local checks run 40030230911 completed successfully for ee14eae: frontend,
server formatting, strict all-target Clippy and server/local TLS/process tests.
The historical-schema fixture correction is now validated in hosted CI too.
Session paused at the user's request on 2026-10-24. All implementation changes
are committed/pushed through fea5f91. Windows native CI 40030829324 completed
successfully for 00439c4 after adding icon.ico: actual bundled Tauri client build
passes. This proves compilation/linking, not Windows launch, SSH or secure storage.
Local checks 40031022048 for fea5f91 is still in progress; its changes selector and
frontend job have succeeded, server result remains to be observed. The selector
change itself conservatively runs both application jobs.

Next session: observe that exact Local checks run, record Windows compilation in
docs/platforms.md, then prepare the actual unsigned iOS simulator application build
on hosted macOS. No iOS workflow or generated project has been created yet. CLI
2.12.3 source confirms ios build supports --no-sign, aarch64-sim and forwarded
Cargo args; proposed commands: ios init --ci --skip-targets-install, then ios build
--debug --target aarch64-sim --no-sign --ci -- --locked, with the simulator Rust
target installed. Verify tool prerequisites and actual build results before claims.
No simulator boot, app install, signing credential or Keychain operation is approved
by this preparation. Android target/operation approval remains unanswered; do not
create/start an AVD or install/reset apps. No sibling repository was modified.
No completion percentage is audited.



The full roadmap remains active/incomplete: real Proxmox, Android runtime/SSH/IME,
other platform stores/parity, deployment and release gates remain open. No audited
completion percentage is claimed.


Latest source slice: production HTTP admission now has a global burst-120,
20/second budget and 32 in-flight handler cap before authentication. Focused
middleware and affected authentication checks pass (ADR 0071). The Linux release
archive has now been rebuilt and its exact staged process passes admission/refill,
authentication and restart/persistence checks. No host service was installed.
Android APKs are unaffected by this server-only change. Next delivery gate remains
approved exact-APK Android allocation/SSH execution. The phase-6 source threat-model
review is recorded in docs/security.md. The follow-up native-command audit found
custom commands outside the old core-only capability. All 76 current production commands
use an explicit local-main application ACL; actual Linux two-window refusal and
recreation/restart integration pass (ADR 0072). Android APKs are now rebuilt with
this ACL: both architectures compile/package and exact embedded native hashes pass.
Exact-current Android runtime and real target acceptance remain open. The secret-
storage review confirmed existing backup exclusions and removed the unused broad
Android FileProvider; rebuilt APKs pass compiled-manifest policy checks (ADR 0073).
Native remote HTTP 429 handling now displays a fixed wait/retry instruction across
all four decoders without losing retained mutation/idempotency state. Focused HTTPS
and affected error/status checks pass. Both Android APK snapshots now include this
native message change and pass packaging/signature/hash/content checks; exact-current
runtime still awaits approved emulator execution. Same-version stopped-directory
backup/restore now passes against the exact packaged server, including retained
operation/history/idempotency and rollback of a later definition. The packaged
process also now migrates an isolated SQL schema-14 fixture to schema 15, retaining
legacy definition/operation/key, integrity and backup across restart/restore.
Future-schema refusal now passes too: the exact package rejects two schema-16
starts while preserving database bytes/version/rows/schema and releasing its lock.
Roadmap readiness review is recorded in docs/roadmap-status.md. It identifies a
material source gap: existing start/shutdown controls used local loopback. Remote
native lifecycle commands and React controls are now implemented; explicit standalone
live status/detail and saved lifecycle recovery remain open.
Native retained remote lifecycle preflight/clear now compile and pass five focused
HTTPS tests (ADR 0074). Native confirmed start/shutdown, identical-key/body unknown-
reply retry and known-operation GET status now also pass: nine lifecycle tests plus
three affected local and two capacity checks. Remote workspace lifecycle UI now builds
and passes the focused Firefox simulated-IPC flow plus two affected parent scenarios.
Actual Linux native remote lifecycle integration now passes through HTTPS/API/worker:
one provider start/shutdown each despite lost accepted replies and identical retries.
Server scoped saved lifecycle history/detail now pass six router/SQLite checks
without provider/write flags, plus nine affected lifecycle storage tests (ADR 0075).
Stateless native lifecycle history/detail GET readers now also compile/pass six
focused HTTPS tests, plus three local and nine remote affected lifecycle checks.
No approval/consent is constructed or installed by recovery. The explicit saved
lifecycle history/manual-ID/refresh UI now builds and passes focused Firefox
simulated-IPC checks. Actual Linux clean app/server restart now passes: no provider/
catalog, all write flags false, GET-only recovery and unchanged journal/identity/
provider counters. Linux server archive and both Android APKs are now refreshed
with this integrated path; exact-package checks and signature/native-content checks
pass. Explicit scoped remote power/QMP native reading now also passes five focused
HTTPS tests (ADR 0076), with no readiness/ownership inference. Registered remote
rows now offer read/refresh with device-clock observation time/staleness wording;
frontend build and focused simulated-IPC checks pass. Actual Linux native HTTPS/
server/provider reads now verify stopped→running→stopped across confirmed actions,
with observation invalidation and no additional mutation. Both Android APKs now
include this completed client feature and pass signature/manifest/native-content
checks. The server archive hash remains unchanged. Retained remote instance detail
now also compiles/passes five native HTTPS tests, binding selected target/name and
exact YAML SHA-256 (ADR 0077). Explicit detail/YAML inspection UI now builds and
passes focused Firefox simulated-IPC checks for active/archived rows, literal text,
metadata, desired-state wording and stale-result rejection. Actual native/server
inspection now also passes for registered and archived associations after restart
without provider/writes, preserving exact YAML/hash/revision and all journal/provider
invariants. Android artifacts now include this completed client feature and pass
signature/manifest/native-content checks; server hash remains unchanged. Existing
registered rows now expose explicit manual SSH preparation with empty endpoint/account
and unchanged strict host trust; frontend build and focused simulated-IPC checks pass
(ADR 0078). Actual Linux native host-trust preparation now also passes against an
owned loopback SSH peer: explicit trust/fresh inspection, changed-key refusal,
zero auth/channels and unchanged trust on refusal. The owned integration now also
passes application-key authentication, real PTY/shell command output, remote resize,
explicit disconnect and subsequent changed-key refusal, without personal keys/agent.
Both Android APKs now include the existing-instance manual SSH entry and pass
packaging/signature/manifest/current native-library checks. Exact-APK execution
remains open.
Exact-current Android runtime and real deployment/VM acceptance remain open.

Delivery priority after user feedback: produce installable/testable artifacts for
mobile -> allocation -> SSH, while retaining the full roadmap. The Linux x86_64
server release archive now exists in `.lab/packages/`, with service/config files,
checksums and a copy-only staging installer. Actual packaged process auth/restart/
persistence checks and unit syntax verification pass. No host service was installed
or started. Current-source ARM64/x86_64 Android test APKs now compile and are
packaged in `.lab/packages/android/`, with verified signatures/manifests/ABIs and
current native-library hashes. Emulator 37.2.12 and verified Android 15 x86_64
image are now prepared in the owned SDK; current host SSH fixture compiles. No
AVD exists on the dedicated port. Explicit approval was requested for creating/
starting vmanyware-native-smoke and resetting/installing only its VMAnyware app,
under AGENTS.md's VM-mutation rule; it has not been granted yet. Do not interpret
automatic goal continuation as target approval. Next: run the exact-APK owned
Android allocation/SSH journey after approval. Target systemd/TLS/real Proxmox
acceptance still needs concrete environment and approvals.
The saved-recreation client history/manual-ID panel is implemented and passes
focused Firefox simulated-IPC and actual Linux native restart checks. Both Android
APK snapshots have now been rebuilt from this source; signatures, manifest/ABI and
embedded native-library hashes pass. Their exact-artifact Android runtime test
stays pending approval. Actual isolated Linux app/server saved-recreation restart now
passes without provider/catalog/writes and with unchanged provider/journal/identity
records. Next delivery gate: approved exact-APK Android allocation/SSH execution;
the rebuilt snapshots now include the latest source UI.


2026-10-21: the user widened the scope to a local provider beside Proxmox
(VMAnyware.md §9). ADR 0062 proposes the seam, direct QEMU (or libvirt) and a
first slice with its acceptance; it awaits the user's choice of hypervisor and
first local target. No code, no VM operation.

Current implementation (2026-10-21): deletion now has confirmed API/native/client
controls and scoped server GET history/detail, including deleted associations.
Actual server restart recovery passes without provider/write flags. Native saved-
deletion GET readers now compile/pass eight focused tests and 14 affected checks.
Client history/manual-ID UI now builds and passes focused Firefox simulated-IPC
checks. Actual isolated Linux native deletion/app+server restart recovery now passes
with one provider DELETE, identical API retry after a lost response, retained
definition/deleted association and unchanged GET-only recovery without provider
configuration or write flags. Read-only retained-definition recreation planning
is now implemented with exact saved-YAML/hash binding and no provider request,
reservation or enqueue. Schema-15 recreation storage now atomically retains
source lineage and fresh confirmed provisioning with a distinct instance ID;
focused restart/idempotency checks pass. Confirmed HTTP recreation enqueue
and source-scoped GET detail now pass queue/retry/restart/refusal tests. The
production recreation API/worker journey now also passes generated HTTPS
clone-to-readiness, database-reopen and no-replay cases with two disk sizes.
Source-bound live recreation preflight now passes generated-HTTPS and HTTP
checks without writes/reservations. Native retained source/preflight review and
clear commands now compile/pass focused HTTPS tests. Native confirmation,
identical-key retry and source-scoped GET status now compile/pass too. Client
deleted-row review/confirmation/status and SSH trust handoff now build and pass
focused Firefox simulated-IPC checks. Actual isolated Linux recreation now
passes native WebView/IPC/authenticated-server/worker/generated-HTTPS integration,
including identical retry, six mutations, source retention and new guest readiness.
Server source-scoped recreation history now passes bounded discovery/restart/
isolation/corruption checks without provider/writes. Stateless native fixed GET
history/detail readers now compile/pass focused HTTPS and affected retained
workflow tests. Client saved-recreation history/manual-ID recovery now builds
and passes focused Firefox simulated-IPC checks. Actual native Linux app/server
saved recreation restart recovery now also passes with provider/catalog removed,
all writes false and only GET recovery traffic.
Real VM/platform/release gates remain open; the roadmap is not complete.


Latest server slice adds read-only snapshot listing for registered instances,
with fixed retained scope/target, exclusion of Proxmox's live `current` entry,
explicit incomplete metadata and safe errors. See docs/snapshots.md and the
verification entry below. The remote client now displays snapshot metadata with
explicit reads, target binding and empty/error/incomplete states; frontend build
and focused Firefox simulated-IPC checks pass. The actual native crate now
compiles and all three snapshot HTTPS tests pass using the recreated, signed
package prefix `.lab/native-deps`; four affected shared-read tests also pass.
Actual native WebView -> production Rust HTTPS -> authenticated Axum -> generated
HTTPS provider snapshot listing also passes (details below). Read-only snapshot
creation preflight now validates bounded inputs, capability, uniqueness, incomplete
work and retained target stability, with five targeted tests passing. Snapshot
HTTP submission now requires independent runtime opt-in and full confirmed
target/request with one idempotency key. Schema 11 includes the durable confirmed
snapshot intent, mutation exclusion, dispatch/task guards and restart semantics. Storage tests
pass, including affected lifecycle/provisioning migrations. Fixed one-shot
transport, revalidating executor, task/metadata completion and GET-only recovery
now pass generated-HTTPS tests. The server services this queue with the independent
VMANYWARE_SNAPSHOTS_ENABLED opt-in, default false. Confirmed HTTP enqueue,
capability reporting and generated-provider queue-to-result checks now pass.
Native retained prepare/confirm/read/ack commands now compile and pass ten
focused HTTPS tests, including unknown-response same-key retry and disconnect.
The remote client now has a single retained snapshot review/consent/status form;
frontend build and focused Firefox simulated-IPC checks pass. Actual isolated
Linux WebView -> production native commands -> authenticated server -> generated
HTTPS provider snapshot creation now passes, with one POST and journal/result
verification. Read-only per-instance server operation history/detail recovery now
passes seven targeted HTTP/SQLite tests without provider access or write flags.
Native fixed GET history/detail commands now compile and pass seven targeted
HTTPS tests without preparing a new intent. The client history/saved-ID UI now
builds and passes focused Firefox checks, including a fresh connection and stale
reply suppression. The real native fixture now creates once, restarts both app
and server, and recovers the unchanged succeeded operation through GET only with
provider configuration removed and writes disabled. Read-only disk-only restore
preflight now passes six targeted HTTPS/API tests; it binds selected metadata and
canonical saved configuration, requires stopped power and keeps execution off.
Schema 12 now adds explicit data-loss approval and durable no-replay restore
storage, with matching rollback-task and stopped-target evidence guards.
Fixed one-shot rollback transport and the revalidating executor now pass seven
generated-HTTPS tests. A separate VMANYWARE_RESTORES_ENABLED flag, default false,
is wired into the actual server worker; process tests pass. Confirmed restore HTTP
enqueue now binds both path identities and data-loss approval, reports independent
capability and passes API/generated-provider/actual-process checks. Native retained
destructive restore commands now compile and pass 14 HTTPS tests,
including both consent flags, unknown same-key retry and GET-only known-operation
tracking. Explicit client restore selection, destructive review, dual consent and
status now build and pass focused Firefox checks; affected creation/list checks
also pass. Actual isolated native restore now passes with a deliberately lost
HTTP response, identical API retry, one provider rollback POST and independent
journal/fingerprint/stopped-target checks. Scoped read-only restore history/detail
now pass 11 HTTP/SQLite tests without provider/write capabilities. Native fixed
GET saved-restore recovery now compiles and passes nine HTTPS tests without
preparing consent. Explicit client saved-restore history/manual-ID/refresh now
builds and passes focused Firefox checks. Actual native app/server restart
recovery now passes with provider removed, all writes off, GET-only traffic and
unchanged operation. Read-only deletion target/protection/volume review now passes
five HTTPS/API tests. Permission-backed HA/replication reads now pass four new
and five affected checks. Fixed provider effects, recorded fleecing and candidate/
preserved volume scope now pass ten targeted checks. Read prerequisites can pass,
execution stays off. Full-scope deletion consent and schema-13 durable no-replay
journal now pass eight new and 67 affected checks. Independent deletion
absence observation passes three targeted HTTPS tests. Schema-14 atomic retirement
now requires an opaque matching task/absence proof; retained deleted identities
and client list compatibility pass local checks. Standalone fixed DELETE and
no-replay executor now pass eight generated-HTTPS checks, including GET-only
partial-cleanup recovery. The independent default-off deletion flag and fair
production worker now pass actual isolated process, crash/restart and router
checks. Full-scope confirmed deletion HTTP enqueue now passes five focused
API tests and an actual authenticated server/generated-provider worker journey.
Retained native deletion review/dual consent/same-key submit/status commands
now compile and pass 14 focused HTTPS/contract tests; shared server hash and five
affected native checks also pass. Explicit client deletion scope/dual consent/
same-key retry/read-only status now builds and passes focused Firefox simulated-IPC
checks, plus affected snapshot-create/restore checks. Scoped read-only server
deletion history/detail now passes eight HTTP/SQLite tests and actual server
restart recovery without provider/writes. Native saved-ID/history GET readers now
compile/pass eight new plus 14 affected checks. Client saved-ID/history UI now
builds/passes focused Firefox checks. Next: actual native deletion/restart journey.
Snapshot creation/restore and real infrastructure testing remain open.

Android SDK/emulator were stored under `/tmp/vmanyware-android-resume`, which is
absent on 2026-10-20. Native Android work requires recreating that owned toolchain;
do not rerun tests or claim fresh mobile evidence from the historical reports.
The initial Gboard key-grid visibility and Unicode/composition gates remain open.

Current Android x86_64 APK passes the actual native confirmed-create/API/worker/
generated-HTTPS-provider journey, explicit ready-target handoff and same-key
SSH/OS-shell checks through direct emulator host-alias networking, with zero ADB
reverse mappings (ADR 0053). Earlier tunnel-only limitations are superseded for
this owned emulator. The provider/guest remain synthetic, not a real VM.
Brief Android HOME/background/resume now also passes on the same SSH session
without input or authentication replay (details below). Actual Gboard ASCII
letter/space, backspace and send taps also pass after rotation/control-key checks.
The initial Gboard key-grid visibility issue and Unicode/composition remain
unverified. Keep real VM writes gated by concrete
target/action approval. Full roadmap, other platforms, lifecycle/snapshots/
recreate/delete, deployment and release acceptance remain open.

## Licensing

Original code, documentation and assets are offered under MIT OR Apache-2.0.
Added both license texts and aligned Cargo workspace/client/server/SSH experiment
and npm metadata. Third-party licenses remain unchanged.
Verification: parsed manifests and checked license inheritance/texts; no native,
application or infrastructure tests were needed for metadata changes.
Next: include both texts in future application installers when packaging is added.

## README presentation

The README follows the devutils structure: overview, install, use, documentation,
development and license. Detailed text is preserved in `REFERENCE.md`.
Documentation-only verification: checked relative link targets and section order;
no application or infrastructure tests were run. Product verification is unchanged.
Next documentation task: keep the concise entry points aligned with implementation.

## Priority — first usable mobile journey

The user wants a usable mobile app that creates a VM and opens SSH before completing
the rest of the roadmap. Prioritize this vertical journey: secure remote control-plane
access; complete create/start/readiness workflow; one selected mobile platform with
OS-appropriate identity storage and terminal; then an explicitly approved disposable
VM end-to-end check. Defer snapshots, recreate/delete and broad packaging until this
journey works. The full roadmap remains the overall goal.

A platform preference question is pending (Android/iPhone/both). Platform-independent
backend work can continue. The macOS agent remains platform-specific; application-key
SSH now passes Linux native protocol/real-shell fixtures. Android ARM64/x86_64 debug
APKs build; Android 15 x86_64 native bridge/key persistence/launch now pass. Mobile
SSH/ADB-injected keyboard/rotation/reconnect now pass via an owned ADB tunnel.
Native authenticated HTTPS and read-only recovery now pass against the actual
local service through a verified-TLS ADB tunnel. Direct networking, complete
create-to-SSH, full software IME/lifecycle and deployed access remain unproven.
Do not infer that the desktop agent backend works on mobile or expose unauthenticated
remote control-plane writes. ADR 0034 records this priority correction.

## Resume here — latest Linux slice

- Verification rule: select the smallest tests/checks covering the current slice's
  diff and affected callers. Do not repeat unrelated/already-passing suites without
  a concrete reason. See AGENTS.md; documentation-only edits need review only.
- Registered instance associations, scoped synchronous/queued observations and
  explicit client monitoring are implemented; schema version is 14.
- Latest slice: confirmed minimum create API and production worker pass complete
  generated-HTTPS journeys with/without resize, restart and idempotent replay.
  Remote native create commands, visible confirmation and status now pass local
  targeted tests; read-only reconnect/restart recovery now passes native HTTPS and
  browser checks. Encrypted app identity/public-key insertion now passes Linux
  native and browser checks. Application-key SSH authentication/PTY now passes
  real local shell and browser checks. Android debug APKs and actual emulator
  bridge/key/restart/painted-launch checks pass; current APK SSH/PTY/input/rotation
  and reconnect pass through an explicit owned ADB reverse tunnel. Direct remote
  network/create integration and full software IME remain next. Native HTTPS
  connection/read-only recovery now pass with a connection-scoped private CA.
  Prior Rust/HTTPS/journal evidence is
  recorded below. These are targeted runs, not a new full suite. Earlier baseline: 107 server tests; frontend build + 23 Node tests;
  Linux Firefox simulated-IPC walkthrough and real server/TLS/worker/restart flow.
- Native Tauri HTTP commands now compile and pass on Linux via a temporary
  signed-package WebKitGTK prefix (122 workspace tests: 104 server, 18 native).
  Isolated Linux native lifecycle GUI/IPC passes; normal packaged startup
  and real VM snapshot/restore remain unverified; no lifecycle
  write was executed on real infrastructure. Runtime capability defaults off.
- Read-only lifecycle preconditions now check marker, lock, template, pending
  changes and configuration revision. API capability reflects explicit runtime opt-in;
  passing checks never replace a full action confirmation.
- Lifecycle checks now have an explicit client display with scoped/stale-result
  guards. Durable confirmed action storage and fixed provider transport are
  implemented with a locally tested durable executor, validated opt-in capability
  and fair sequential worker rounds. Confirmed HTTP enqueue is locally tested.
  Real binary HTTPS start/restart/no-replay evidence now passes.
  Typed native action/read commands now compile and pass HTTP tests.
  Standalone confirmed intent/monitoring controller now passes Node tests.
  React explicit confirmation, persistence and resume controls now pass browser tests.
  Real Linux WebKit/Tauri/Axum/generated-HTTPS lifecycle UI now passes.
  Fixed full-clone transport/target observation now passes HTTPS tests.
  Durable provisioning intent/journal and clone dispatch/recovery guards now pass
  targeted SQLite tests. Standalone clone execution/completion and GET-only restart
  reconciliation now pass local HTTPS tests. Fixed CPU/memory configuration and
  observation pass locally. Schema-7 durable CPU/memory execution and GET-only
  restart reconciliation pass. Fixed mapped-disk expansion/configured-size
  observation passes. Schema-8 disk dispatch/task/outcome/recovery guards pass;
  standalone disk execution and GET-only recovery now pass HTTPS fixtures;
  fixed network/Cloud-Init settings and independent observation now pass locally;
  schema-9 Cloud-Init journal and standalone execution/recovery now pass;
  generated seed semantic inspection now passes locally and rejects the known
  scalar form. Remote client reads and explicit template/default-user opt-in
  now pass targeted local checks; next: seed regeneration/start/readiness.
  The detailed final sections below distinguish actual and deferred evidence.

## Implemented

- Tauri 2/React/TypeScript native shell; user confirmed the macOS native bridge.
- Loopback-only Axum service, validated runtime configuration, verified-TLS
  Proxmox reads and safe errors. Fixed lifecycle POST transport is locally tested;
  the service defaults new writes off, with full-target confirmed HTTP enqueue
  available only under explicit runtime capability.
- Strict versioned YAML definitions, deterministic exports/hashes and SQLite
  create/list/read storage. Native editor validates without saving and provides
  safe field hints. Definitions named `validate` can also be read normally.
- Durable inventory jobs with idempotent enqueue, background worker, polling,
  terminal persistence and interruption recovery. SQLite migration 2 and an
  exclusive process lock protect database ownership.
- Explicit server template catalog, deterministic offline provisioning preview,
  and read-only preflight for template/disk/Cloud-Init/storage/bridge/VMID.
  Results are passed/failed/unknown and bound to current inputs/plan hash.
- Read-only provider task observation with validated UPID/node identity,
  redacted failure status and bounded polling. No durable provider-task linkage
  or guest readiness is implied by this helper.
- Safe JSON request/lifecycle events and generated request IDs. Unix SIGINT and
  SIGTERM request graceful shutdown with a five-second HTTP drain limit; startup
  failures and drain timeout emit fixed stage codes.
- Prepared GitHub Actions server/frontend checks with pinned actions and locks;
  workflow not pushed or executed; local Linux server container checks pass.
- Debian Cloud-Init template guide, platform/toolchain evidence matrix and
  interim feasibility report. No real template preparation was executed.
- Isolated opt-in desktop SSH/PTY experiment, outside the root workspace and
  application dependency graph. Generated in-memory keys and localhost fixture
  prove strict host-key checks, signed authentication, real local shell output,
  resize, Ctrl-C foreground interruption and fresh reconnect. See ADR 0008.
- Isolated macOS OpenSSH-agent authentication proves exact public-fingerprint
  selection, delegated signatures, no identity fallback and rejection of
  untrusted hosts before agent access. Agents/keys/sockets are disposable;
  no user agent or credential file is accessed.
- Shared native/experimental persistent endpoint-scoped SSH host trust. On
  macOS, the native UI inspects keys and offers explicit first-use confirmation;
  known keys require a completed verified handshake, while changed keys cannot
  be approved. Host inspection never authenticates or loads private keys.
- Explicit macOS agent-identity listing and an authentication-only native check.
  Rust keeps expiring, one-use window/endpoint/user/socket/key selections; fresh
  persisted host trust is verified before each agent access. Only one selected
  identity is tried, and success closes the connection without a terminal.
  Startup and host inspection never access the configured agent.
- Native Rust PTY/session transport with window-bound IDs, positive PTY/shell
  acknowledgments, bounded byte input/output, resize and explicit close. Startup
  reuses the same selected-agent authentication path. Window destruction aborts
  its actor and closes its owned socket.
- Graphical macOS terminal prototype using pinned xterm.js 6.0.0/fit 0.11.0.
  Explicit opening consumes the chosen identity; output pulls wait for parser
  consumption, input is serialized/bounded and resizes coalesce. Ctrl-C and
  disconnect controls, stale-result cleanup and StrictMode lifecycle are wired.
  Terminal output cannot set the app title, navigate links or access clipboard.
- Rust-owned window/endpoint/key-bound UUID challenges expire after 120 seconds,
  are consumed once and invalidate on new inspection, cancellation or window
  destruction. Owner/probe limits are eight each; probes have a ten-second
  overall deadline. Trust initializes lazily in the client app-local directory.

## Current verification

Environment: macOS arm64, Rust/Cargo 1.99.1, Node 26.11.1, npm 11.20.0,
Apple CommandLineTools. Dependency locks and compiler pin are committed.

- Latest `make check` passed frontend TypeScript/Vite build, seven Node terminal
  controller tests, Rust formatting/all-target Clippy with warnings denied and
  all 83 workspace tests: 50 server, 33 native. Latest `make check-ssh` passed
  sixteen isolated tests. `make check-terminal-webkit` passed both isolated
  development scenarios. The main bundle is ~246 KiB and the lazy terminal
  ~335 KiB before gzip; no chunk-size warning.
- Provider tests use generated localhost TLS fixtures, never runtime credentials.
  They cover certificate/hostname/authentication failures, time/size bounds,
  inventory, preflight, task status validation and bounded polling.
- Actual server subprocess tests passed safe log-field allowlists/redaction,
  unknown routes/methods, generated IDs, SIGTERM/SIGINT exit zero and restart
  with the same database after lock release. Tests clear the child environment.
- HTTP/storage tests cover YAML validation, save/list/read/conflicts, canonical
  hashes, migrations, reopen, idempotent jobs, terminal persistence and recovery.
- Native HTTP tests cover typed requests, validation/provisioning input bounds,
  safe field hints, redaction, previews and preflight result decoding.
- CI YAML passed actionlint 1.7.12 using a temporary checksum-verified binary.
  Its commands passed locally; no GitHub/native/mobile CI result is claimed.
- Latest `make check-ssh` passed isolated formatting, locked Clippy and all sixteen SSH
  tests on macOS arm64. The real PTY test uses two fresh connections, observes
  stty size changes and requires foreground sleep interruption/status 130 within
  two seconds. Fixture cleanup leaves no owned shell/sleep processes. An early
  development timeout remains undiagnosed and is documented honestly; repeated
  final checks passed. The experiment has no existing credentials or config.
- Trust tests cover exact confirmation/reopen/authentication, rejection before
  authentication, changed keys, port/DNS/IP scope, corrupted/missing/insecure
  data, bounded 250 ms locking, stale/concurrent claims and endpoint validation
  after deserialization. Generated public keys persist only in temporary stores.
  The native client and isolated fixture now import the same trust source.
- New native manager tests cover handshake/confirmation/reopen/changed keys with
  zero authentication attempts, replay/wrong-owner/fingerprint/expiry/cancel,
  invalidation and out-of-order results, capacity and corrupt storage. Window
  destruction cleanup also prevents reuse of an old label reviving a challenge.
  Non-macOS wrappers are source-prepared, not cross-compiled or tested.
- Native agent slice: `make check` passed frontend build, fmt, Clippy and all
  76 workspace tests. Six native agent tests use cleared-environment disposable
  system agents with `-D -U -a`, in-memory fixture keys and temporary trust.
  They cover exact selection/no fallback/no channels, unknown/changed hosts
  before agent access, fresh verification before authentication, removed keys,
  wrong-window/expired/cancelled/replayed selections, stale cancellation, bounds
  and unresponsive-agent timeout/socket closure. No personal agent was accessed.
  The isolated experiment remains at sixteen passing tests; no new dependency
  was added. Native screens compile but their manual walkthrough is pending.
- Native PTY slice: `make check-rust` passed all 80 tests. Two localhost
  protocol tests prove binary NUL/255/Ctrl-C round trips, 32 KiB input, resize,
  positive PTY/shell acknowledgments and explicit refusals, exit/EOF/close,
  wrong-window refusal, disconnect/reconnect and zero remaining owned fixture
  connections after explicit close/window destruction. Two policy tests cover
  queue saturation, single-reader enforcement, capacity and invalid dimensions.
  This is protocol echo, not a native graphical or actual remote OS shell test.
- Native real-shell slice: `make check-rust` passed all 81 tests; isolated
  `make check-ssh` passed all sixteen tests. The native HostManager → explicit
  disposable agent → SessionManager path now exercises actual /bin/sh output,
  stty size 24×80 → 43×132, foreground sleep interruption/status 130 within two
  seconds, reconnect and process/socket cleanup after explicit close and window
  destruction. Closing while sleep 30 is running also leaves none of the owned
  shell/foreground PIDs alive. The experimental and native tests share one
  bounded-output PTY fixture. portable-pty is a macOS dev-dependency only; no
  runtime dependency or user agent/credential/real infrastructure was accessed.
- Output pressure slice: a real yes/paused-reader test exposed Ctrl-C starvation
  in the pinned SSH library's full receive queue. The native actor now fails and
  closes after three seconds of blocked output; failure survives a full queue
  and is observed after queued data, followed by Closed. `make check-rust` passed
  all 82 tests. The pressure test verifies bounded failure/owned-process cleanup,
  then a fresh connection with actively consumed output interrupts yes within two
  seconds and remains usable. This is a documented prototype limitation: a long
  renderer pause can close a session (ADR 0014), not transparent indefinite pause.
- Graphical terminal slice: seven Node tests cover binary/split UTF-8 output,
  renderer backpressure, stale results/errors, bounded serialized input, resize
  coalescing, overflow, native saturation retries and exit/EOF/close.
- Isolated macOS WebKit development smoke passed with simulated IPC and a
  nonpersistent store: explicit identity choice/no automatic agent listing,
  split UTF-8 rendered by xterm, resize, synthetic Unicode textarea input,
  Ctrl-C byte, StrictMode no premature close/duplicate login, disconnect and
  cleanup of a late start after edits. It never invokes Rust SSH commands or
  accesses a personal agent. See `make check-terminal-webkit`. This is not an
  integrated native-to-shell test or packaged CSP/real-keyboard evidence.
- Deferred renderer slice: xterm loads only on explicit terminal opening, before
  any native start. The parent owns the native ID even while a lazy mount is
  pending. WebKit tests now deterministically pause/fail the renderer module: an
  edit during loading starts no late connection; a load error is visible, cancels
  the selection and starts no native connection. Existing render/input/resize/
  StrictMode/stale-native-result checks still pass. No new dependency was added.
- Shutdown-bound slice: `make check-server` passed all 50 server tests. A real
  child with an unfinished HTTP body exits zero after the five-second drain
  deadline, emits only the fixed timeout/stopped events, closes its socket and
  restarts with the same database/port. An idle restart still exits on SIGINT in
  under two seconds. The bound covers HTTP drain, not arbitrary blocking OS I/O.
- Real native Tauri slice: the optimized macOS Cargo example passed the full
  compiled frontend/lazy-xterm → real Tauri IPC → generated-key SSH peer → real
  disposable agent → /bin/sh chain. First-use trust is confirmed only against
  fixture-generated public metadata, then rechecked before exact identity use.
  Two sessions render actual stdout, match stty rows to the resized renderer,
  interrupt sleep/status 130 within two seconds and disconnect/reconnect. Rust
  confirms resize observations, zero owned SSH connections and closed/reaped
  shell fixtures. The child environment, trust store, agent and incognito WebView
  are isolated. No new dependency, production test flag, fake IPC or CSP bypass.
  `make check` still passed 83 Rust/seven frontend tests; all-target Clippy passed.
  Two development assertions were corrected (DOM row concatenation and assuming
  uncoalesced resize counts); final native scenario passed all checks.
- Native changed-key slice: the real Tauri example also rotates only its generated
  localhost server key after two terminal sessions. The actual UI refuses the
  changed key and offers neither overwrite nor identity listing. Native report
  confirms exactly two authentications and byte-identical temporary trust data
  before/after rotation. Optimized native smoke, formatting and all-target Clippy
  passed. This adds no production command or real infrastructure operation.
- The owned local service was restarted with current shutdown code on loopback
  8080 and the existing local database, with provider/catalog variables unset. A
  real GET /api/v1/health returned {"status":"ok"}. Earlier validation POST
  evidence returned HTTP 200/canonical hash/request ID. Current native screens
  have an isolated real-Tauri walkthrough; normal startup/user-agent behavior
  and real guest interoperability still require their own checks.
- Template guide: seven shell blocks passed syntax checks without execution;
  links/catalog sample reviewed; official dated image/manifest/build metadata
  exist. No cloud image was downloaded and no VM operation was executed.

Earlier runtime evidence: `make doctor`, actual local HTTP/SQLite restart smokes,
`make dev` native launch and user-confirmed `Native bridge connected (macos)`.
Latest `npm --prefix apps/client run tauri build -- --no-bundle` passed and
produced the current optimized `target/release/vmanyware-client` (1m02s Rust
release build), confirmed Mach-O arm64, including stall handling/lazy rendering. It was not launched for an integrated SSH walkthrough. The
older renderer smoke uses an isolated WebKit host. The new optimized Cargo
example now proves real Tauri IPC/custom-protocol/CSP integration with disposable
resources; normal app startup/distribution and real guest tests remain open.
Windows shutdown code is source-prepared, not compiled or tested on Windows.
At that historical macOS verification point, no Proxmox infrastructure had
been contacted. The later local x86 lab evidence is recorded below. Phase 0
remains incomplete.

- Linux server slice: `make check-linux-server` passed on committed revision
  9f126f8: Debian 12 aarch64, Rust/Cargo 1.99.1, formatting, locked all-target
  Clippy and all 50 server tests. The first run exposed an external kill-command
  assumption in diagnostics tests; POSIX shell builtin signaling fixes it without
  adding dependencies. `make check-server` also passed all 50 tests on macOS.
  Docker used the explicit local socket, isolated anonymous configuration, pinned
  public image, disposable tracked-source archive and bounded container resources.
  Container/source were removed. No personal credentials or infrastructure were
  accessed. This is not GitHub Actions, Linux frontend or desktop evidence.

- Linux frontend slice: `make check-linux-client` passed against committed
  revision a1ceead in Debian 12 aarch64 with Node 26.11.1/npm 11.20.0: locked
  npm installation, TypeScript/Vite build and all seven controller tests. No
  application code or dependency changes were needed. The server/client targets
  share the isolated Docker runner, with separate digest-pinned public images.
  This does not validate Linux native Tauri or GitHub Actions. Temporary sources
  and container were removed; public images remain cached locally.

- Development launch recovery: a second `make dev` failed because an existing
  project Vite/Tauri session already owned port 1420. The identified old session
  and remaining native child were stopped, leaving the API untouched. A fresh
  `make dev` passed Vite startup, compiled the native client and launched
  target/debug/vmanyware-client. README documents single-session recovery.
  No application code changed; native UI interaction was not asserted here.

- Local-service demo slice: an explicit Check local service / Retry service
  check action calls the native health command without requesting inventory or
  discovering an SSH agent. The UI distinguishes unchecked/checking/available/
  unavailable, clears errors on retry and resets health state on port changes.
  A healthy service does not imply Proxmox is configured. README gives the demo
  steps. `make check-client`, `make check-rust` and `make check-terminal-webkit`
  passed: seven JavaScript tests, 84 Rust tests (50 server/34 native), formatting,
  all-target Clippy and both WebKit scenarios. Native HTTP tests cover healthy,
  unhealthy/malformed responses, invalid port and refused connection; simulated
  WebKit IPC covers unavailable/retry/reset with zero inventory calls. These
  are separate transport/UI checks, not a normal-app end-to-end walkthrough.

- Local server handoff: the previously owned PID 25304 held the default SQLite
  lock and port 8080, causing the user's second server launch to fail. It was
  stopped with SIGTERM. A cleared-environment server reopened the same database,
  emitted server_listening on 8080, then exited zero on SIGTERM. No lock file or
  database was deleted. No agent-owned server remains; the user can run make server.

## Remote repository

Private GitHub repository: https://github.com/moha-bekh/VMAnyware
Local remote: origin (currently SSH). Created at the user's explicit request to publish
existing commits. Main is the branch to push; verify remote HEAD after transfer.
Pushing the prepared workflow may trigger GitHub Actions; local checks are not
proof of a successful hosted run.

## Session handoff — 2026-10-17

- User explicitly stopped the session; do not continue background development.
- Latest implementation commit: 557ff7f (independent local-service check/retry).
  Checks passed: 84 Rust tests, seven JavaScript tests, Clippy/formatting/build,
  two simulated-IPC WebKit scenarios. Linux server/frontend checks passed in
  earlier slices; no Linux native desktop or iOS application proof.
- The duplicate database-lock failure was resolved by stopping the known local
  server and verifying reopen/clean exit against the same database. The user
  may subsequently start their own server; inspect current ownership before
  starting or stopping anything. Never delete the lock to bypass an active owner.
- Demo commands: make server and make dev in separate terminals. Run only one
  Vite/Tauri development session on port 1420. Check local service works without
  Proxmox; Connect requests Proxmox inventory and requires provider configuration.
- Agreed priority: improve/verify the macOS demo while the user prepares Proxmox
  on another PC. Keep macOS as the primary development host; use isolated Linux
  checks. An iOS UI demo is a later feasibility step requiring full Xcode;
  current SSH-agent authentication is macOS-specific.

## Next single task

Obtain explicit approval of the concrete clone-202 snapshot/rollback operations
in `docs/x86-snapshot.md`, then execute that bounded experiment. Read-only API
preflight passed: stopped target, no named snapshots, snapshot feature supported,
active local-lvm with 17,977,492,007 bytes available. Thin-pool metadata gates
still need read-only verification before execution. No snapshot/restart/rollback
was performed. Preserve stopped 201/202 and template 9000.

## Resumed session — Linux environment

- Current execution host is Linux x86_64, not the historical macOS arm64 host.
  `make doctor` failed: Node.js, npm, Rust and Cargo are unavailable to the
  process. No build, test suite or native application launch was performed here.
- Prepared `docs/macos-startup-smoke.md` for the normal application entry point:
  distinct test identifier, removed agent environment, unused loopback port,
  unavailable/retry/reset observations and owned-process cleanup. README links
  it. Reviewed against the native startup source, UI labels and Tauri config;
  the procedure and data/WebView isolation remain unexecuted on macOS.
- No roadmap item is newly complete. No service was started/stopped, no
  credentials were inspected and no Proxmox endpoint was contacted.

## x86 host readiness slice — 2026-10-17

- User redirected work toward tests previously blocked by the M1. Actual host:
  Arch Linux x86_64/kernel 7.2.9, Ryzen 9 3900X, 24 threads, approximately
  62 GiB usable RAM and 278 GiB repository-filesystem free space.
- AMD-V (`svm`) is exposed; `kvm_amd` nested parameter is `1`. Outside the
  sandbox, `/dev/kvm` opens and KVM_GET_API_VERSION returns 12. The sandbox's
  missing device is not a host failure. No VM was created to obtain this result;
  successful nested guest execution remains unproven.
- Earlier doctor output was misleading: sandboxed Node spawnSync reports EPERM
  even when a subprocess prints a version. Rustup also encountered read-only
  paths and an incomplete pinned toolchain. Approved external diagnostics let
  rustup repair/install 1.99.1. External `make doctor` now passes: Rust/Cargo
  1.99.1, Node 26.11.1, npm 12.2.0. No doctor code was changed.
- Locked frontend `npm ci --no-audit --no-fund` installed 26 packages. npm
  blocked esbuild's postinstall under its existing script policy; no policy was
  changed, and the actual Vite build still passed. `make check-client` passed
  TypeScript/Vite and all seven tests. `make check-server` passed formatting,
  all-target Clippy with warnings denied and all 50 server tests, including
  subprocess shutdown/restart. Checks ran outside sandbox subprocess/network
  restrictions. Tracked dependency locks remain unchanged.
- QEMU/libvirt/container runtime executables were not found on PATH; WebKitGTK
  4.1 pkg-config metadata is absent. No Linux native application was built or
  launched. Native SSH managers remain macOS-only; x86 does not enable them.
- Prepared `docs/x86-lab.md` with host evidence, outer-VM resources, disk path,
  loopback management/console ports and acceptance boundaries. Requested explicit
  approval for QEMU installation and this VM's creation/start; no VM mutation
  or package installation has occurred. Credentials, inner guests and restores
  need separate concrete approval. No Proxmox endpoint was contacted.

## External inputs and limits

- Historical Apple M1 host reports nested virtualization unsupported. Current
  Ryzen x86 host now runs the approved disposable Proxmox lab described in
  `docs/x86-lab.md`; inner guest execution remains unproven.
- Live version/node/empty-inventory reads now pass against the approved lab
  with an explicit CA and dedicated token. Known-template visibility and
  preflight still need a prepared catalog and scoped permissions. Follow
  `docs/debian-template.md`.
- Real clone/configure/start/snapshot/restore/cleanup operations need approval of
  the concrete disposable target and operations. Guest readiness remains untested.
- SSH/PTY on a real guest and iOS/Android require their own hosts/devices/toolchains.
- `../omniscient` and `../atfq` were inspected; `../distroproof` was not found.

## Remaining scope

Application provisioning, VM instances/lifecycle, snapshots, production desktop/
mobile SSH, authenticated remote control planes and signed distribution remain
incomplete. The macOS terminal is a fixture-tested prototype, not real-host proof.
Default tests work without configured Proxmox, user credentials or mutable VMs.
The macOS native target now includes the pinned russh candidate for handshake/
trust, explicit agent authentication and PTY transport (ADRs 0010–0013); no
production/mobile library guarantee is implied.

## Approved outer lab preparation — 2026-10-17

- User explicitly approved QEMU installation and creation/start of only
  `vmanyware-pve-lab` with the resources/network/disk in `docs/x86-lab.md`.
  This supersedes the earlier pending-approval status. Inner guests and
  credential creation remain outside this approval.
- Added `/.lab/` to `.gitignore`; `git check-ignore` confirms exclusion.
- Downloaded official amd64 Proxmox VE 9.2-1 over verified HTTPS from
  https://enterprise.proxmox.com/iso/proxmox-ve_9.2-1.iso into `.lab/pve/`.
  SHA256 is `4e88fe416df9b527624a175f24c9aa07c714d3332afb1ee3dbf3879573ef2c6c`,
  matching both the official release page and downloaded checksum. The ISO
  was renamed from `.part` only after hashing succeeded. No boot occurred.
- `download.proxmox.com` failed hostname verification; verification was never
  bypassed. The official enterprise download host verified successfully.
- Ports 18006/5901 were free; `/dev/kvm` is visible to this process. QEMU
  executables remain absent. `sudo -n pacman -S --needed --noconfirm
  qemu-system-x86 qemu-img` failed with `a password is required`, without
  installing packages. User was asked to run the package command locally.
- No virtual disk, VM process, credentials or service changes were made.
  No application tests were run for this preparation-only slice.

## Outer VM boot evidence — 2026-10-17

- User completed local package installation. Both QEMU system and image tools
  report 11.1.2. Read installed invocation help before selecting arguments.
- Rehashed the ISO successfully, checked free ports and refused any preexisting
  disk before creating `.lab/pve/pve.qcow2` (64 GiB virtual capacity).
- Started `vmanyware-pve-lab` as UID 1000, PID 171988, with q35/KVM, host CPU,
  eight vCPUs, 16 GiB RAM, virtio disk/network and user-mode NAT. No host bridge,
  service, firewall, shared folder or physical disk was changed. Actual launch
  arguments and PID are saved under ignored `.lab/pve/`; verify PID identity
  before future process operations.
- QMP query-status reports running; query-kvm reports present/enabled;
  query-cpus-fast reports eight CPUs. Screenshot inspection shows the Proxmox
  VE 9.2 ISO release 1 boot menu. No installer selection was made.
- `ss -ltnp` confirms this process alone listens on `127.0.0.1:5901` (VNC) and
  `127.0.0.1:18006` (forward to guest 8006). This is forwarding evidence, not
  an available Proxmox API. QMP is an ignored local Unix socket.
- VM remains running for the next installation step. No credentials or inner
  guests were created. Nested guest execution and installed Proxmox remain
  unproven; no roadmap checkbox was completed. No application code changed or
  application tests ran. `git diff --check` passed.

## Installer boot preparation — 2026-10-17

- User authorized continuing the outer installation. Sent Enter at the verified
  ISO boot menu through the owned QMP socket. The installer kernel booted;
  console shows DHCP assignment of 10.0.2.15 on enp0s2 from QEMU 10.0.2.2.
- No disk installation or credential creation was initiated. Asked the user
  to approve concrete hostname/network defaults and choose local root-password
  handling (self-entry over VNC or explicitly authorized generated local file).
  That answer remains required before dependent installation steps.
- Proposed hostname: pve-lab.vmanyware.test; address 10.0.2.15/24; gateway
  10.0.2.2; DNS 10.0.2.3; ext4/LVM on the sole 64 GiB virtual disk.
  Existing credential files were never read. VM remains running.

## Installed outer Proxmox lab — 2026-10-17

- User explicitly approved the proposed installation settings and generation
  of the local administrator password. Installed only on `/dev/vda` (the new
  64 GiB qcow2), ext4/LVM. Country France, Europe/Paris, US English keyboard;
  hostname pve-lab.vmanyware.test; nic0 bridged by installer vmbr0 with
  10.0.2.15/24, gateway 10.0.2.2 and DNS 10.0.2.3. Notification address
  root@vmanyware.test is a lab placeholder, not working email delivery.
- The generated 32-character random root credential is in ignored
  `.lab/pve/root-password`, mode 0600, used only for owned local-console login.
  No secret was printed. Initial rapid simulated keystrokes failed password
  confirmation; slower input and regenerated pre-installation credential
  succeeded. The final credential was validated by actual console login.
- Installer completed and automatically rebooted from disk. Console reports
  pve-manager/9.2.2/b9984c6d90a4bd80, kernel 7.0.2-6-pve. `pvesm status`
  shows local (dir) and local-lvm (lvmthin), both active. `ip -br address`
  confirms nic0 UP and vmbr0 UP with 10.0.2.15/24. `/dev/kvm` exists and
  `grep -c svm /proc/cpuinfo` returns eight. No inner guest was booted; nested
  execution remains unproven. Root console was logged out afterward.
- Exported only public `/etc/pve/pve-root-ca.pem` via the owned console to a
  temporary bounded loopback HTTP receiver. Receiver closed after receipt.
  CA SHA256 fingerprint:
  D6:11:0C:E7:1B:12:F5:B6:79:22:99:22:E9:19:34:E8:98:FE:84:0D:88:69:56:70:0E:5F:A0:A6:38:5D:1D:2D.
  curl with this explicit CA and hostname pve-lab.vmanyware.test resolved to
  127.0.0.1:18006 returned HTTP 200 from the login page with TLS verification.
  No system trust/DNS configuration was modified or verification bypassed.
- VM PID 171988 remains running; ss confirms both 5901 and 18006 still bind
  only to 127.0.0.1. This is an installed lab and verified login-page transport,
  not Rust/API-token inventory, template, provisioning or snapshot evidence.
  No API user/token or inner VM was created. No roadmap item was checked off.
- Updated lab instructions and state; `git diff --check` passed. No application
  source changed and no application check suite ran for this infrastructure slice.

## Inventory credential preparation — 2026-10-17

- Inspected existing probe and runtime configuration. Downloaded the installed
  Proxmox API schema with verified TLS: version/nodes/qemu-list GET endpoints
  allow authenticated users; VM visibility requires per-VM VM.Audit.
- Prepared `.lab/pve/create-reader.py` for only pve-lab: root HTTPS login,
  passwordless vmanyware-reader@pve, privsep inventory token, no ACLs, both
  expiring in 24 hours. Secret output is exclusive/0600 under ignored .lab.
  Script syntax parsed successfully; it has NOT been executed. Concrete
  credential approval was requested and remains pending.
- `cargo build --locked -p vmanyware-server --example proxmox_probe` passed.
  Bubblewrap read-only hosts-file override resolved pve-lab.vmanyware.test to
  127.0.0.1 without changing host DNS/hosts. No new dependency or production
  code was introduced. Live authenticated inventory remains untested.
- Next: after approval, run the prepared script in the isolated hosts namespace,
  then pass the generated token only in the probe child environment and record
  actual version/node/empty-template evidence. No template or VM operation is
  part of this slice. `git diff --check` passed.

## Live Rust inventory slice — 2026-10-17

- User explicitly approved the prepared account/token operation. Created only
  vmanyware-reader@pve (no password/group/ACL) and privsep inventory token on
  the disposable pve-lab. Both expire 2026-10-18 21:22:17 UTC. Secret output
  is `.lab/pve/inventory-token.json`, verified mode 0600, never printed.
- Root credentials/tickets were used over verified HTTPS only for approved
  creation and readback, not by the Rust probe. Administrative GETs confirmed
  privsep=1, matching expiry and no direct user/token ACLs. Token GET
  /access/permissions returned HTTP 200 with an empty object.
- Existing compiled Rust proxmox_probe exited zero: Proxmox 9.2.2/release 9.2,
  pve-lab online, no visible templates. Separate token GETs to version/nodes/
  nodes/pve-lab/qemu returned 200; qemu data was an empty array. This does not
  prove VM/template visibility or template permissions.
- All connections verified the lab CA and hostname. Process-local Bubblewrap
  hosts mapping avoided any host DNS/trust changes. Only probe child
  environment carried the token; no credential command arguments were used.
- Live invalid-token GET returned 401; TLS handshakes rejected an untrusted CA
  and wrong hostname. Rust probe also exited one with its expected fixed error
  for a bad token and absent lab CA. No mutation request was attempted with
  the reader token. Initial diagnostic helper wrongly assumed 401 bodies were
  JSON; it was corrected to inspect status without parsing error bodies.
- `cargo build --locked -p vmanyware-server --example proxmox_probe` passed.
  No application source/dependency changes; no broad test suite was repeated.
  `git diff --check` passed. Updated roadmap's live connection and TLS/token
  checks only; known-template/provisioning gates and Phase 0 remain open.
- Updated docs/proxmox-setup.md, docs/x86-lab.md, docs/feasibility.md and state.
  VM remains running; no inner guest/template, ACL grant, snapshot or restore
  was created. Next single task is the concrete template preparation plan.

## Frontend visual direction — 2026-10-17

- User selected sibling ../devutils as the temporary visual reference while
  its organization name may change. Read its src/style.css and index.html;
  did not modify that repository or copy its organization identity/assets.
- Bounded visual slice: adapted near-black background, cyan/violet accents,
  fine panel borders, translucent-looking gradients, monospace labels, light
  primary actions and visible cyan keyboard focus. Added a product header and
  phase badge; retained all existing actions, labels and component state logic.
  No dependency, external font request, WebGL or animation system was added.
- Changed apps/client/src/styles.css and main.tsx. Responsive controls wrap and
  stack on narrow screens; reduced-motion preference removes button transitions.
  Existing native terminal transport/rendering logic was untouched.
- `make check-client` passed TypeScript/Vite and all seven Node controller tests;
  `git diff --check` passed. Firefox headless screenshots of the built browser
  preview at 1280x1000 and 390x844 were visually inspected for header/initial
  controls. Captures are ignored under .lab/frontend-preview/. These are not
  full-page, native Tauri, terminal, physical keyboard or mobile-device checks.
- Used fresh disposable browser profiles and a temporary loopback static server;
  server stopped after capture. No native IPC, credentials or VM operation was
  invoked. The underlying application remains a Phase 0 prototype.
- Next single task remains the concrete Debian template preparation plan on
  pve-lab; further frontend layout work should follow a separate bounded slice.

## Workspace layout slice — 2026-10-17

- User asked to take the devutils-inspired frontend further. Kept one bounded
  workspace-layout slice: persistent desktop sidebar, wrapping narrow-screen
  navigation, compact workspace header, observed service/node/template summary,
  and adjacent definition editor/provisioning preview on wide screens.
- Navigation uses normal fragment links to focusable section headings. All
  components remain mounted: navigation does not reset YAML or intentionally
  close an SSH session. Active destination styling uses CSS :target/:has.
  Added a keyboard skip link, visible heading focus and native-size checkbox/
  radio controls. No routing library, new runtime dependency or provider action.
- Summary cards derive exclusively from current UI state: service starts as
  Not checked, node/template values are dashes until inventory is loaded.
  Browser preview is identified explicitly; historical lab results are never
  injected as current application data.
- Modified main.tsx, styles.css and section headings in ProvisioningPreview.tsx
  and SshHostTrust.tsx. Existing action handlers, native IPC and terminal
  transport remain unchanged. No sibling repository was modified.
- `make check-client` passed TypeScript/Vite and seven Node tests on the final
  changes; `git diff --check` passed. An isolated Firefox/Marionette browser
  smoke verified all five fragment links/focus destinations, unknown initial
  state, YAML edit persistence and actual Tab/Enter skip-link activation.
  No horizontal document overflow at actual widths 1440, 800 and 500. Firefox
  window resizing clamps requests below 500; no narrower automated assertion
  is claimed. Separate CLI screenshots at 390x844 and 1440x1100 were visually
  reviewed. Captures are in ignored .lab/frontend-preview/workspace-*.png.
- Early browser helper read fragment/focus before the default anchor action
  completed; separated click/read commands fixed the assertion. No production
  behavior change was needed. No native IPC or credentials were used.
- This does not validate macOS Tauri layout, live inventory states, error/result
  layouts or SSH continuity during navigation. Next frontend task: validate
  this workspace with loaded/error states and a live terminal in the isolated
  macOS native walkthrough. Infrastructure next remains the Debian template
  plan, not an authorized template/guest creation.

## Workspace state/lifecycle validation — 2026-10-17

- Host is Linux; macOS Tauri/WebKit execution is unavailable here. Continued
  the authorized validation by extending existing fixture scenarios rather
  than claiming native evidence or changing the product's support boundaries.
- Extended experiments/terminal-webkit/fixture.js: loaded one-node/one-template
  inventory; failed refresh clears counts; successful empty inventory shows
  zeros; validation error/retry; mixed passed/failed/unknown preflight; port
  changes invalidate results. All data/IPC is explicitly simulated.
- While xterm is mounted, four workspace anchors must focus their headings,
  preserve the exact terminal DOM element and cause no additional session
  start/close. Loaded preflight and terminal states assert no horizontal
  document overflow. Existing lazy-renderer cancellation/failure, split UTF-8,
  input/Ctrl-C, resize, StrictMode and stale-result checks remain in the runs.
- Executed this same extended fixture under isolated Firefox 158 on Linux
  using temporary Vite injection/module gating and a disposable Marionette
  profile. Both normal and renderer-failure scenarios passed at 1440 and 500
  CSS-pixel viewport widths (four scenario runs). Temporary adaptation lives
  under ignored .lab/frontend-preview/; it is not a new supported check target.
  Browser profiles and Vite instances were closed/removed by the harness.
- Extended experiments/terminal-native/scenario.js to navigate the same four
  anchors during each actual shell session, require the same terminal element,
  then require fresh AFTER_NAVIGATION_OK shell output. Syntax checked only;
  the modified native scenario and macOS WKWebView fixture have NOT run.
- Updated both experiment READMEs with the assertions and platform evidence
  boundary. `make check-client` passed TypeScript/Vite and seven Node tests;
  both scenario files passed node --check; git diff --check passed. No product
  source/dependency changes were needed and no Rust suite was repeated.
- No credentials, agents, SSH peers, Proxmox endpoints or VMs were accessed by
  these browser tests. Next frontend task: run make check-terminal-webkit and
  make check-terminal-native on a macOS GUI host for native workspace evidence.
  The Debian template plan remains the independent infrastructure next task.

## macOS transfer handoff — 2026-10-17

- User requested commit/push so the Mac can pull and execute the native checks.
  Reviewed the accumulated workspace/tests and x86 lab evidence documents.
  Remote main was still ca2db67, equal to the local base, before publication.
- Publishing only application source, fixture source and repository documents.
  All .lab runtime files (disk/ISO/credentials/CA/profiles/screenshots/helpers)
  remain ignored. No dependency lock or Rust source changed in these slices.
- Current checks: make check-client passes build/seven tests; extended normal
  and renderer-failure fixture scenarios pass in Firefox at 1440/500 widths;
  both fixture scripts parse; git diff --check passes. Existing x86 server and
  real read-only provider evidence is recorded separately above.
- README now gives the Mac handoff: git pull --ff-only, locked npm ci,
  make check, make check-terminal-webkit, make check-terminal-native. Then
  review the normal UI in one make dev session. Neither smoke needs the x86
  lab or any transferred credential. No new macOS execution is claimed.
- Next single frontend task: execute those checks in a macOS GUI session and
  record native workspace/terminal outcomes. Linux work can proceed later on
  the independent Debian template plan without treating native gates as passed.

## Native navigation smoke correction — 2026-10-18

- Reproduced the reported native failure at step 8 on macOS arm64 with
  `node experiments/terminal-native/check.mjs`. Added example-only assertion
  reporting to stderr; the exact failure was waiting for AFTER_NAVIGATION_OK
  after workspace navigation. All four destination-focus/terminal-identity
  assertions had already passed.
- The navigation scrolls the terminal offscreen, where xterm defers rendering.
  Corrected the scenario to scroll the retained terminal back into view before
  typing/asserting fresh shell stdout. The same real IPC/agent/shell scenario
  then passed step 21: two authentications, resize, Ctrl-C, reconnect, changed
  host rejection, unchanged trust and owned socket/shell cleanup. No product
  behavior, dependency, credential or infrastructure change was needed.
- Full `make check-terminal-native` passed after the fix, including the
  frontend TypeScript/Vite build, seven Node tests, release example build and
  native step-21 scenario. Example-specific locked Clippy with warnings denied, Rust formatting, scenario syntax and
  diff whitespace checks passed. The sandbox blocks the disposable agent;
  native GUI execution requires the approved unsandboxed command.
- This is local fixture evidence, not a real guest, personal agent, physical
  keyboard, packaged distribution or mobile check. Next single frontend task:
  run `make check-terminal-webkit` for the extended macOS simulated-IPC states;
  the independent infrastructure task remains the Debian template plan.

## Debian template preparation — 2026-10-18

- User reports all Mac tests green after edits and authorized continuing the
  remaining work. Clean workspace already contains 759aac5: example-only native
  failure reporting and scrollIntoView before checking post-navigation xterm
  output. Its recorded real macOS native run passed step 21. User's broader
  all-green report is accepted as user-reported evidence, not a local Linux
  rerun or real-guest/mobile proof.
- Current host remains Linux. Owned outer QEMU PID 171988 still has loopback
  ports 18006/5901. Verified-TLS read-only lab observations: no VM/container
  resources; nextid 9000 available; local-lvm active/enabled with image content
  and 21,302,870,016 available bytes; vmbr0 active over nic0. Requested custom
  role VMAnywareTemplateReader does not exist. Observations reserve nothing.
- Downloaded official dated Debian 13 genericcloud amd64 20261015-2632 image,
  SHA512SUMS and build JSON to ignored .lab/pve/images/. Exact SHA-512 matches
  the HTTPS manifest (recorded in docs/x86-template.md). qemu-img confirms qcow2,
  3 GiB virtual size and no backing file; metadata JSON parses. No image boot,
  mount, import or guest filesystem change occurred.
- Prepared docs/x86-template.md: create only VM9000 on pve-lab, import/expand
  scsi0 to 8 GiB on local-lvm, Cloud-Init/debian/NoCloud, 2 cores/2048 MiB,
  serial console and vmbr0, convert while stopped and never booted. Separate
  approval also covers a VM.Audit-only role and nonpropagating /vms/9000 grants
  for existing reader user/token. No token regeneration or expiry change.
- Prepared ignored .lab/pve/templates.yaml mapping this candidate; not claimed
  live-validated. Template DHCP is dormant, not proof of nested guest routing.
  A clone/start slice must establish an explicitly approved usable guest network.
- Reviewed installed qm/pveum manuals fetched with explicit CA/hostname trust.
  All three proposed shell blocks pass bash -n without execution. No product
  source changed, so no application suite was repeated; git diff --check passed.
  Read-only observations used only the lab's generated root credential over
  verified HTTPS, without logging it or inspecting personal credentials.
- Next: explicit approval of this concrete template/ACL operation, then create
  and verify stopped template metadata through the scoped reader/Rust probe.

## Real unbooted Debian template — 2026-10-18

- User explicitly approved the concrete template/ACL operation. This approval
  persists for that target/operation; no repeated confirmation is needed. It
  does not authorize arbitrary VM creation, clone/start, network changes,
  token renewal or deletion.
- Rechecked 9000 availability, active image-capable local-lvm, vmbr0 and absent
  role. Downloaded the same pinned Debian image inside pve-lab over verified
  HTTPS; required the exact SHA-512, qcow2/no backing file/3 GiB before import.
  Created only 9000, imported scsi0, expanded it to 8 GiB, added Cloud-Init and
  configured 2 cores/2048 MiB/debian/NoCloud/ciupgrade=0/serial boot/vmbr0.
- Verified stopped/configured state by HTTPS API before conversion. Converted
  to template; final disk is local-lvm:base-9000-disk-0,size=8G, Cloud-Init disk
  is vm-9000-cloudinit. No guest password/key, extra disk, hooks, passthrough or
  custom QEMU args. No start was issued, so the template remains unbooted.
- Created VMAnywareTemplateReader with VM.Audit only; exactly two direct
  nonpropagating /vms/9000 ACLs for vmanyware-reader@pve and its inventory token.
  Administrative readback confirms the role/grants, privsep and unchanged expiry.
  Reader's own effective-permissions endpoint reports only VM.Audit at that path.
- Reader GETs list exactly one stopped template and return its current config
  HTTP 200. Existing Rust proxmox_probe exits zero with 9.2.2, pve-lab online
  and Template 9000: vmanyware-debian-13-20261015. Explicit CA/hostname checks
  remain enabled; token is passed only in child environment, never arguments.
- A diagnostic helper initially used an unsupported administrative token-
  permissions URL; replaced it with role/ACL metadata readback and the reader's
  own effective-permissions endpoint. No mutation was repeated after this error.
- Script hashes were compared through the owned QMP console before each stage.
  Temporary public-only loopback script server stopped; root console logged
  out. Outer VM remains running. All image/runtime/credential/evidence files
  stay ignored under .lab/pve/. No private user credentials were inspected.
- Updated template/lab/setup/feasibility docs and roadmap known-template item.
  No product source changed; actual infrastructure checks above are the
  verification for this slice. Shell syntax and git diff --check pass; no
  application suite was repeated. Catalog is prepared locally, not live
  provisioning/preflight proof. Phase 0 remains incomplete.
- Next single task: propose an isolated guest-network/clone plan. No DHCP/SSH
  reachability, nested guest boot, Cloud-Init completion, snapshot or restore
  is implied by a readable stopped template.

## Isolated clone plan — 2026-10-18

- Continued with the requested network/clone preparation. Live read-only lab
  observations confirm VMID 201 available, vmbr1 absent, template 9000 stopped,
  active local-lvm with 20,301,635,126 bytes available. Host port 18222 is free;
  inspected host routes show no 192.0.2.0/30 route. Recheck all inside/outside
  routes, listeners and resources immediately before execution.
- Prepared docs/x86-clone.md: full clone 9000 -> 201, 2 cores/2048 MiB/12 GiB,
  runtime-only private vmbr1 at 192.0.2.1/30, guest 192.0.2.2/30 without default
  gateway. Literal-address SSH through a fixed Proxmox Python relay and owned
  outer QEMU hostfwd on 127.0.0.1:18222 avoids NAT/DHCP, package installation,
  Linux host networking changes and management-bridge reloads. DNS may inherit
  host defaults; no guest Internet or working DNS is assumed by this smoke.
- Plan includes exactly one newly generated disposable SSH identity (no personal
  agent/config/key), serial capture attached before first CPU execution, trusted
  console host-fingerprint comparison, strict SSH, Cloud-Init/resource/PTY checks
  and graceful shutdown/owned transport cleanup without deleting the clone.
- Read installed qm documentation for freeze-at-startup; owned outer QMP help
  confirms hostfwd_add syntax. info usernet shows only existing management
  forwarding. No new bridge, listener, forward, identity or VM was created.
- Static-address manual clone does not match current DHCP-only product preview.
  Documented this limitation rather than claiming application provisioning.
  No product source changes; two shell blocks pass bash -n, git diff --check
  passes. No application suite was repeated for this preparation-only slice.
- Next single task: approval of the concrete clone/network/SSH-identity and
  graceful-shutdown operations, then execute the prepared smoke in stages.

## First real nested clone/SSH smoke — 2026-10-18

- User approved the concrete docs/x86-clone.md plan. Rechecked all target/
  storage/network/port collisions; created only runtime vmbr1 and full clone
  201 from template 9000. Used root lab HTTPS API for clone/config/resize/
  frozen start/continue/shutdown with bounded task polling and recorded UPIDs.
  No product mutation method, reader ACL expansion or new API token.
- Generated one new disposable Ed25519 key in ignored .lab/pve/clone201, mode
  0600. Only its public key was supplied through Cloud-Init. No personal key,
  agent or SSH configuration was accessed. Outer forward bound only to host
  127.0.0.1:18222; owned Python relay bound only to 10.0.2.15:18222, reaching
  guest 192.0.2.2:22. No NAT/DHCP, management/host network reload or package install.
- QEMU confirmed prelaunch before the serial recorder attached; guest final
  Cloud-Init stage completed at about 6.8s after CPU continuation. Captured
  Ed25519 host key independently matched SSH endpoint before isolated trust
  was written. Strict selected-key SSH/reconnect pass; mismatched known-host
  data rejected with 255, without changing any guest key.
- Real guest resource assertions pass: Debian 13, debian, 2 CPUs, 1974 MiB RAM,
  12 GiB disk, 12,468,432,896-byte expanded root filesystem, private /30 route
  and no default route. Linux OpenSSH PTY passes 24x80 -> 43x132, foreground
  sleep Ctrl-C/status130 in 0.022s, usable shell and reaped SSH child. No
  remaining debian sleep process. This is separate from native macOS Tauri.
- Strict initial readiness failed correctly: cloud-init 25.1.4 returns 2,
  done/degraded done, errors=[], recoverable DEPRECATED scalar user warnings.
  Root HTTPS readback confirms Proxmox emits user: debian. Warning is recorded,
  not suppressed; separate SSH/resource checks do not constitute clean readiness.
  Leave phase-0 clone/readiness checkbox open pending a deliberate correction.
- Initial PTY harness expected the remainder of an interrupted interactive Bash
  list to execute. A separate post-prompt status command corrected that test;
  final real interruption passes. No product source change was needed.
- Graceful shutdown task OK; 201 and template9000 both stopped. Owned transport
  PID exited, no attached bridge links remained, vmbr1 removed, only added
  outer forward removed. Temporary script/serial receiver stopped and root
  console logged out. Clone/disk/identity retained; freeze=1 and absent vmbr1
  mean restart requires deliberate restoration. No snapshot/restore/deletion.
- Updated docs/x86-clone.md, template guide, feasibility and roadmap with exact
  evidence/limits. No application source/dependency changed or broad suite
  repeated; infrastructure assertions above and git diff --check are checks
  for this slice. Raw logs/keys/tasks stay ignored, not committed.
- Next single task: investigate scalar-user Cloud-Init generation and define
  a compatible correction/explicit readiness policy before another guest test.

## Offline Cloud-Init compatibility correction — 2026-10-18

- Confirmed Proxmox generator's scalar user output against upstream source and
  the lab's generated data. Cloud-Init 25.1.4 internally maps that scalar to a
  name dictionary but emits a recoverable deprecation. Proposed minimal change
  is user: {name: debian}, retaining users:[default] and all other allowed data.
- Added experiments/cloud-init-user/convert.py, check.py and README; ignored
  generated Python __pycache__ directories. Converter
  accepts only observed passwordless/default-user data and explicit matching
  expected username; rejects unknown features/passwords/upgrades; writes a new
  0600 output exclusively. It has no provider access and never executes data.
- Downloaded exact upstream 25.1.4 source archive into ignored .lab; SHA256
  fa70a77fc3cd3167a051e9ab04af4d4f56d3ffa0deb320735c889a6a367d3a3d.
  Installed upstream Python requirements only into an experimental venv. No
  application/system dependency was added. Local tested packages are recorded
  under .lab/cloud-init-compat/tested-python-packages.txt.
- Actual upstream normalizer/schema check passes: original scalar causes one
  deprecation, mapping causes none, normalized user/groups/default/sudo/shell/
  locked-password values are identical. Unexpected input and file-overwrite
  guards pass. Applied the same conversion offline to real stopped-201 generated
  data: schema valid, identity equivalent and exact public key retained.
- Lab read-only observations: local storage content currently import/backup/iso/
  vztmpl (no snippets); 202 available; local-lvm free 19,140,628,710 bytes; both
  201 and 9000 stopped. No Proxmox package, storage, VM, network or credential
  mutation was made. No old guest cache was reset and no warning was suppressed.
- Prepared docs/cloud-init-compatibility.md with exact fresh-202 retest: scoped
  local snippets capability/file, converted 202-specific user-data, same bounded
  private SSH topology, strict first-boot exit-zero/error-free acceptance and
  graceful shutdown/transport cleanup. This requires concrete approval; the
  existing 201 candidate must not be reused for a differently named/keyed clone.
- Added link to clone evidence. Offline semantic/schema/CLI guards and README
  shell syntax passed; git diff --check passed. No product source changed, so
  no application suite was repeated. First-boot compatibility remains unproven.
- Next single task is approval/execution of that fresh-202 retest. Keep the
  phase-0 readiness gate open until actual guest exit-zero evidence exists.

## Fresh Cloud-Init compatibility retest — 2026-10-18

- User approved the concrete 202 retest. Rechecked unused VMID/storage/ports;
  full-cloned 9000 to 202, configured 2 CPUs/2048 MiB/12 GiB, new disposable
  key and private static /30. Clone/resize/frozen-start/shutdown tasks all OK.
- Converted fresh 202-specific user-data, verified exact key/hostname and
  upstream schema/normalization. Added only local storage snippets capability,
  exclusively created root-owned 0600 snippet and bound only 202 cicustom user.
  Generated network/metadata retained; no template/201/package changes.
- Serial capture preceded CPU continuation. Initial key check correctly refused
  before the boot block was available; after capture the endpoint key matched.
  Strict selected-key SSH and reconnect passed; wrong pinned key rejected 255.
- Actual first-boot Cloud-Init 25.1.4 exit 0, done/extended done, no fatal or
  recoverable errors globally or per stage; final completion at 6.95s. Consumed
  user-data SHA256 equals prepared snippet (recorded in compatibility document).
- Debian 13/debian, 2 CPUs, 1974 MiB, 12 GiB disk and 12,468,432,896-byte root
  filesystem verified; no default route. PTY 24x80 -> 43x132, Ctrl-C/status130
  in 0.036s, usable shell/reaped child/no leftover sleep.
- Graceful shutdown OK; final API readback confirms 202/201/9000 stopped.
  Owned relay/recorder exited, runtime bridge removed, added loopback forward
  removed, HTTP receiver stopped and root console logged out. Retain stopped
  202/disks/snippet/storage capability/key; freeze=1 and absent vmbr1 require
  deliberate restoration before another start. No snapshots/restore/deletion.
- Updated compatibility evidence, experiment README, feasibility, roadmap and
  ADR 0017. Phase-0 manual provisioning prototype gate is now evidenced;
  application execution, least-privilege mutation ACLs and mobile remain open.
- Checks: actual infrastructure assertions above, offline upstream semantic/
  schema/guard checker and git diff --check. No product source changed; no
  broad application suite rerun. Next single task: concrete snapshot test plan.

## Snapshot experiment preparation — 2026-10-18

- Read STATE.md, roadmap and phase-0 brief; scoped this slice to preparation.
- Verified by trusted lab HTTPS API: 9000/201/202 stopped, 202 exact disk/network/
  snippet/freeze configuration, snapshot list only current, snapshot feature
  hasFeature=1, active enabled nonshared LVM-thin pve/data, available bytes
  17,977,492,007. No VM/storage/network/credential mutation was performed.
- Added docs/x86-snapshot.md: exact named stopped-disk snapshot on 202, existing
  strict SSH identity/trust, A/B fsynced guest marker plus post-snapshot file,
  explicit stopped rollback then guest A/absence proof, bounded task observation,
  capacity/metadata/config gates and graceful shutdown/owned transport cleanup.
- Retain snapshot and stopped target; no deletion/live RAM snapshots/ACL changes.
  External user-data snippet is preserved and is not claimed as snapshot backup.
- Official upstream LVM-thin docs and QEMU API consulted; installed API behavior
  and thin-pool metadata remain execution gates, not asserted runtime evidence.
- Validation: git diff --check. No application source changes or suite rerun.
- Next single task: explicit approval and execution of this concrete plan.

## Product naming — 2026-10-18

- Standardized product/UI/docs, Rust/npm package names, Tauri identifiers,
  environment variables, YAML API namespaces, fixtures and scripts on VMAnyware.
- Existing databases, app trust directories and environment configuration need
  explicit migration/reconfiguration; no automatic migration is implemented.
- Historical lab identifiers in documents use current branding. Ignored local
  records and external resources retain actual names; verify these before any
  operation, particularly TLS hostname/snippet binding and snapshot preflight.
- Next single task remains the concrete 202 snapshot plan, resolving actual
  external identifiers from VMID and local lab records before execution.
- Naming verification: make check-client passed (build + 7 tests); make
  check-server passed (fmt/clippy + 50 tests). Locked Cargo metadata, workspace
  formatting, changed JS syntax and git diff --check pass. Case-insensitive
  source/document search finds no former product name outside ignored artifacts.
- Full native workspace check cannot complete on this Linux host: GTK/WebKit
  development libraries are missing. No native application launch was performed.
- make check-ssh: fmt/clippy pass, 9 tests pass; the existing real-shell PTY
  fixture fails waiting for its expected READY line framing (actual READY was
  received). No renamed strings occur in that assertion; left this independent
  Linux fixture issue unchanged. Native macOS validation remains to be rerun.

## Phase 0 boundary and operation history — 2026-10-18

- User requested continued sequential work across phases, grouping checks where
  useful. No Proxmox/VM/network/credential operation was executed in this slice;
  the prepared 202 snapshot/rollback still has no execution evidence.
- ADR 0019 deliberately limits the initial MVP terminal to macOS. Mobile and
  Linux/Windows native terminals remain unsupported pending separate evidence.
  Updated platform/feasibility documents and the mobile-limitations roadmap
  item. Phase 0 stays open; no mobile build/runtime success is claimed.
- Corrected the existing isolated SSH real-shell READY framing: printf now
  emits an explicit leading newline rather than relying on OS command echo.
  Linux real-shell stdout/resize/Ctrl-C/reconnect now passes. No SSH runtime
  code, dependency, real host, personal key or agent was involved.
- Added GET /api/v1/operations: latest 100 summaries, deterministic descending
  creation-time/ID order, no inventory result bodies/request keys. Existing
  detail reads retain full results. Database work remains off the async executor;
  history never contacts Proxmox, claims work or retries mutations.
- Added explicit native history loading with empty/error/interrupted states;
  port-keyed unmount invalidates pending results. Added typed Tauri command and
  an HTTP decoding test. Native Rust additions are source-prepared only here.
- Verification: make check-server passed fmt/Clippy and all 51 server tests.
  New history test covers 107 persisted records, reopening, tied timestamps,
  response bounds/payload exclusion, unchanged queued/interrupted records and
  POST rejection. make check-client passed TypeScript/Vite and the existing
  terminal-controller Node test script. No graphical history walkthrough ran.
- make check-ssh passed fmt/locked Clippy and all 10 tests enabled on Linux
  after approved rerun outside sandbox (initial localhost bind was denied).
  macOS-only agent tests are not part of that Linux count.
- make check-rust formatting passed, but workspace Clippy stopped in native
  dependencies: WebKitGTK/JavaScriptCoreGTK 4.1 development packages missing.
  New native command/test did not compile or run; no native launch is claimed.
- Phase 1 command-setup checkbox now reflects existing commands and recorded
  evidence. No lifecycle/provisioning/Phase 2 completion claim was added.
- Remaining history limits: most recent 100 only, no pagination/retention or
  cancellation; records remain addressable by ID. No schema/dependency change.
- Next single task: compile/test and walk through native operation history on
  an equipped native host (empty history, failed/interrupted refresh, port edit
  during loading), then resume the concrete 202 snapshot execution gate.

## Phase 2 — Explicit read-only VM status — 2026-10-18

- Resumed from state/roadmap/phase-0 brief. Current Linux host still lacks
  WebKitGTK/JavaScriptCoreGTK 4.1; sudo requires a password. No package was
  installed and native history compilation/walkthrough remains open. Continued
  with a server-only vertical slice rather than inferring native success.
- Scope/acceptance: read a precise validated node/VMID; use verified TLS and
  bounded existing transport; reject wrong provider identity; distinguish power,
  QEMU execution and unproven guest readiness; never perform VM mutations.
- Added validated VmReference and typed power/QMP observations. New GET
  /api/v1/provider/nodes/{node}/vms/{vmid}/status makes exactly one provider GET.
  Checks returned VMID, omits unrelated fields, normalizes unknown QMP states,
  preserves missing QMP as null and rejects invalid power/identity responses.
- API validates canonical decimal VMIDs/node before provider access, catches path
  decoding rejection, and returns fixed safe validation/provider errors. It
  neither registers ownership nor infers readiness, persistence or authorization.
- Added static log route for VM status and corrected missing history route in
  diagnostic allowlist. Real child-process tests prove dynamic node/query text
  stays out of request logs and both route patterns are recorded correctly.
- Documented contract/limits in docs/vm-status.md and linked from docs/api.md.
  Consulted upstream QEMU API implementation for status/current and VM.Audit;
  installed-lab permissions/runtime behavior are not newly verified here.
- Checks: make check-server passed fmt/Clippy and all 55 server tests. Four new
  tests cover target bounds, power/QMP states, unknown/missing/malformed fields,
  wrong VMID, exact GET path/no extra request, untrusted CA, safe HTTP errors,
  timeout and method refusal. After expanding diagnostic assertions, its real
  subprocess log/redaction/restart test passed again. git diff --check passed.
- No dependency/schema change, real infrastructure request, credential access,
  VM mutation, native build or application launch. Phase 0 snapshot/desktop
  integration and Phase 2 managed ownership/start-stop remain open. Marked only
  the existing Axum/versioned-route foundation item evidenced, not Phase 2 done.
- Next single task: persist an explicit managed-instance/provider reference and
  definition hash before implementing durable start/shutdown intent and task
  reconciliation. Keep native history verification as a host-dependent gate.

## Phase 1/2 — Persist local instance references — 2026-10-18

- Read current state, domain/phase roadmap and phase-0 brief. Scope: persist an
  explicit definition/target association, retain the exact definition snapshot,
  reject duplicates/stale inputs and read it after restart. No provider mutation.
- Added instance registration types and SQLite migration 3. An instance keeps
  canonical definition YAML/hash/revision, generated ID, endpoint-scoped provider
  reference, node/VMID, creation time and registered status. Stored definition
  schema/hash/name are checked in the registration transaction before copying.
- Provider scope is SHA-256 of the canonical configured HTTPS origin. Tokens,
  CA bytes and URLs are not copied into the record. Token rotation retains the
  scope; different endpoint/port changes it. This is not cluster identity or
  authorization; aliases, VMID reuse and node movement need reconciliation.
- Unique (scope, VMID) prevents duplicate local associations across nodes and
  concurrent database connections. Expected definition hash rejects stale
  clients; snapshots survive future edits to the saved definition. A definition
  can still be reused for several instances. No existing record is overwritten.
- POST /api/v1/environments performs local registration only, requires provider
  configuration for scope derivation, rejects Origin writes and unknown fields,
  returns fixed safe errors and responds 201/Location after commit. No provider
  network access or VM existence/ownership/readiness claim is made.
- GET /api/v1/environments returns the newest 100 deterministic summaries without
  YAML; GET /api/v1/environments/{id} returns the retained detail. Both work with
  provider configuration absent and preserve older records by ID. No native UI,
  deletion, pagination, retention or lifecycle execution was added.
- Documented contract and migration in docs/instances.md, API/definition docs and
  ADR 0020. Roadmap annotations distinguish this preparation from unfinished
  provisioning/lifecycle items; none of those gates was marked complete.
- Verification: make check-server passed formatting, warnings-denied Clippy and
  all 63 server tests. Eight new tests cover v2->v3 migration preserving old
  definitions/operations, clean creation/reopen, immutable retained snapshots,
  invalid/stale/missing inputs, concurrent connections, scope isolation/token
  rotation, bounded tied-timestamp lists, HTTP rejection/redaction and detail
  reads without configuration. A listening localhost fixture confirms no provider
  connection occurs during HTTP registration/list/detail. Existing v1 migration
  and newer-schema refusal still pass. Subprocess tests verify static environment
  route logging without names/IDs/queries/body text. git diff --check passes.
- No runtime dependency, credential access, real Proxmox request or VM/network
  mutation. Server-only change: no frontend/native suite or launch repeated.
  Native history verification, phase-0 snapshot/desktop integration and real
  least-privilege lifecycle testing remain open.
- Next single task: expose a read-only observation for a registered instance,
  refusing a changed configured provider scope before any network access and
  using its persisted node/VMID. Ownership verification and durable lifecycle
  intent/task reconciliation must still precede any mutation implementation.

## Registered-instance observation preparation — 2026-10-18

- User requested consecutive slices without a confirmation turn between them.
- Added GET /api/v1/environments/{id}/status using the saved node/VMID. Configured
  provider scope must match before network access; invalid persisted targets fail
  closed. Observation returns separate instance/scope identity and VM power/QMP
  data; registration remains unchanged and no ownership/readiness is inferred.
- Two focused generated-TLS tests pass for exact target/no retargeting, unchanged
  registration, changed-scope refusal and malformed stored-target refusal. Added
  malformed/unknown-ID and method checks; full checks are grouped with next slice.
- Next slice: durable asynchronous instance observation through the existing
  read-only worker, binding idempotency to kind/instance and preserving interrupted
  results after restart. No provider mutation or generic job payload is planned.

## Durable instance observation — 2026-10-18

- Continued sequentially into a second slice in the same turn at user request.
  Added POST /api/v1/environments/{id}/refresh returning 202/Location before
  provider work. Existing single worker handles fixed inventory/instance reads.
- Migration 4 retains all inventory operation IDs/keys/results/timestamps and
  instance snapshots, adding an instance FK/kind constraint and per-instance
  active-read uniqueness. Idempotency now binds globally to kind and instance;
  cross-kind/target key reuse conflicts instead of returning unrelated work.
- Worker freshly checks provider scope and stored target, persists scoped
  power/QMP results or fixed failure codes, and leaves registration unchanged.
  Interrupted reads remain interrupted; queued reads may resume after restart.
  No provider mutations, UPID linkage, ownership/readiness claim or new dependency.
- make check-server passes fmt/Clippy and all 73 tests. New focused tests cover
  scope/target checks, async HTTP acceptance, kind/target idempotency, concurrent
  enqueue, distinct-target queues, v3->v4 preservation, interrupted recovery,
  safe provider failures and exact single GET/no replay of completed reads.
- Documented ADR 0021 and API/storage semantics. Next slice: explicit client
  instance list and resumable observation, with a disposable simulated-IPC browser
  walkthrough; native Rust compilation remains gated on missing WebKitGTK.

## Client observation and real-process workflow — 2026-10-18

- Continued into client and integration slices without a confirmation turn.
  Client explicitly lists registered instances and starts/resumes scoped read
  operations. A per-port/instance non-secret request key survives failed/paused
  monitoring and is cleared only after a confirmed terminal state.
- Added typed Tauri list/begin/read commands and prepared HTTP contract tests.
  Result kind/job ID/instance binding is checked in Rust; renderer additionally
  checks scope/node/VMID. Source preparation is not native compilation evidence.
- Added six Node observation-controller tests for polling, deadlines/same-key
  resume, stale completions, leaving during delays, unrelated responses and
  transport failure/interruption. Existing seven terminal tests remain passing.
- Added opt-in make check-instances-browser: owned Linux headless Firefox,
  temporary profile/loopback Vite and development-only injected fake IPC. Actual
  React interactions verify empty/list loading, queued/polled success, paused VM,
  safe scope failure, port-change stale result/error rejection, same-key resume
  and observation identity in history. No native commands/provider are invoked.
- Initial browser runs intermittently timed out with the list stuck Working.
  Stage diagnostics exposed first-click invalidation during StrictMode's initial
  effect cleanup. Instances/history now enable actions only after lifecycle
  setup. The fixture clicks at the first enabled DOM commit via MutationObserver;
  the final full make target and strengthened direct browser run both pass.
  Firefox's headless graphics diagnostic appeared on failed runs; no renderer
  failure is attributed to it. No owned fixture Firefox process remains.
- Added real binary subprocess workflow with cleared environment, generated
  HTTPS CA/peer and fake token only: HTTP definition save/register/202 enqueue ->
  actual background worker -> TLS GET -> persisted paused-VM observation. A
  second held provider read survives graceful SIGTERM/restart as interrupted;
  same-key polling returns the original record and never replays the GET.
  Instance snapshot stays identical. Final SIGINT exits zero; logs stay redacted.
- Extended static log coverage for scoped status/refresh and safe path rejection
  for malformed operation IDs. No real infrastructure/credential access, VM
  mutation, production fixture flag or new runtime dependency was introduced.
- Verification: make check-server passes fmt/warnings-denied Clippy and all 74
  tests; make check-instances-browser passes TypeScript/Vite, all 13 Node tests
  and the simulated-IPC browser walkthrough. Workspace formatting and git diff
  --check pass. Latest bundle is ~257 KiB plus lazy terminal ~335 KiB before gzip.
- make check-rust was attempted: formatting passes, native Clippy/build stops in
  missing WebKitGTK/JavaScriptCoreGTK 4.1 dependencies. Native additions/tests and
  normal native launch are still unverified. No container engine/socket is
  available here as an alternative; no system package or dependency was installed.
- Next single task: define and implement read-only ownership/identity preconditions
  for a registered instance before preparing durable start/shutdown execution.
  The saved association, matching endpoint/VMID and power state alone cannot
  authorize writes. Real 202 snapshot/restore approval, native integration and
  least-privilege mutation tests remain separate external gates.

## Read-only lifecycle preconditions — 2026-10-18

- User explicitly requested uninterrupted sequential continuation; an active goal
  now tracks authorized roadmap work without requiring another continue prompt.
- Implemented GET /api/v1/environments/{id}/lifecycle-preflight. It reuses scope/
  target checks and makes current-config -> pending -> power -> current-config
  GETs under one 20-second deadline. No schema/dependency/provider write added.
- Checks an exact sole instance tag, non-template, absent lock, no pending values
  or deletes (including force-delete 2), and unchanged config digest. Missing or
  malformed metadata fails closed; action preconditions can be passed/failed/
  unknown, while executionEnabled remains false in every response.
- ADR 0022 explicitly treats the marker as an administrative declaration, not
  independent ownership proof, and digests as observations rather than an atomic
  reservation. No marker/tag/ACL was installed or changed on infrastructure.
- Four focused HTTPS fixture tests and then make check-server passed (80 tests,
  fmt/Clippy). Tests cover exact read order, power/QMP distinctions, foreign/
  duplicate markers, locks/templates/pending deletions, changing digest, malformed
  data, scoped API blockers, unchanged registration and no write method.
- Final grouped make check-server also passes all 80 tests after conservative
  malformed-tag-list rejection and new subprocess static-route log assertions.
  git diff --check passes. No client/native code changed in this slice.
- Next slice: explicit client lifecycle preflight display with identity-bound
  results and stale-result suppression. Native compilation remains an external
  gate; do not infer ownership/readiness or enable writes from the new report.

## Client lifecycle preflight — 2026-10-18

- Continued from the verified read-only server preflight. Added an explicit
  per-instance Check lifecycle control, typed Tauri GET command and prepared HTTP
  contract test. It displays fixed local check labels and never exposes mutation
  controls or raw provider/server messages.
- Renderer validates instance/scope/definition hash/marker/config digest and
  node/VMID, exact known check set/statuses and executionEnabled=false. New reads
  clear old results; leaving/changing ports suppresses late preflight results.
- make check-instances-browser passes TypeScript/Vite, all 15 Node tests and the
  disposable Firefox simulated-IPC walkthrough. New checks cover lifecycle
  blockers, absence of mutation controls/raw messages and stale report rejection.
  No native command, real control plane or real provider was used by this smoke.
- Workspace formatting and git diff --check pass. Tauri command/test source is
  prepared; native compilation remains unproven pending the dependency attempt.
- Next bounded slice: try signed Arch packages in a temporary user-owned build
  prefix to compile/test native HTTP commands without root/system installation.
  If unavailable, retain the external gate and continue durable lifecycle work.


## Native Linux command compilation unblocked — 2026-10-18

- Classified prior goal turn as progress (server preflight implementation and
  passing evidence). This turn delivered its client display, then resolved the
  native build dependency gate without system/root installation.
- Queried public package metadata/local missing-dependency plan; downloaded seven
  exact Arch packages/signatures through verified HTTPS into the owned temporary
  /tmp/vmanyware-native-deps-61ktnqj7 prefix. Verified every signature against the
  public packaged Arch keyring (17 files, zero altered). Initial armored-keyring
  format failure was corrected by dearmoring a local copy; no trust bypass or
  personal GPG config/key/credential file was accessed. No package hooks ran.
- Only copied pkg-config paths were rewritten for that prefix. With its pkg-config
  and library paths, make check-rust passed fmt, all-target warnings-denied Clippy
  and all 95 workspace tests: 80 server and 15 native HTTP. The actual history,
  scoped observation and lifecycle preflight command contracts now run on Linux.
  A standalone client test-list build also succeeds (15 tests listed), independently
  of workspace server feature unification. No native graphical launch is claimed.
- No root package, system database, repository dependency/lock or real VM was
  modified by setup. Temporary helper/manifest/checksums/signature evidence stay
  in that owned /tmp directory; docs/native-linux-prefix.md records reproduction
  and limits. Future goal turns should check it still exists before using it.
- Client evidence: make check-instances-browser passes build, 15 Node tests and
  Firefox simulated IPC, including read-only lifecycle blockers and stale results.
  New native HTTP test uses a complete legitimate preflight shape. Workspace
  formatting/git diff --check pass. Main bundle ~259 KiB before gzip.
- Proposed ADR 0023 records the next actual lifecycle execution contract: explicit
  full-target/action binding, initially disabled capability, persistent dispatch/
  UPID linkage, no POST replay, uncertain outcomes and read-only reconciliation.
  This is planning only; no start/shutdown method, route or enabled write exists.
- Next single task: implement durable lifecycle action inputs/state/storage and
  fixed provider dispatch/task validation with local HTTPS fixtures, then wire
  worker/API only after recovery and no-replay behavior are verified. Keep the
  full roadmap goal active; real infrastructure mutations still need concrete
  approval. Native GUI/mobile/real snapshot and least-privilege gates remain open.


## Durable lifecycle intent storage — 2026-10-18

- Continued the active roadmap goal sequentially. Scope was storage and recovery
  invariants before wiring provider writes; no real infrastructure was accessed.
- Migration 5 preserves existing reads/associations while adding fixed start and
  graceful-shutdown intent, full confirmed input hash, server confirmation time,
  immutable dispatch acquisition and private validated task linkage. No new crate.
- Confirmations expire before dispatch after 120 seconds; retrying an identical
  key cannot extend this window. Changed input/key bindings fail. Per-instance
  queued/running/uncertain mutation exclusion permits diagnostic reads.
- Startup interruption after dispatch becomes uncertain, never an automatic POST
  retry. Known tasks permit future GET-only reconciliation; unknown references
  retain the exclusion. Undispatched running work becomes interrupted.
- Success persistence requires successful task and matching independent power
  evidence; no guest readiness inference. Existing read workers exclude mutation
  jobs, and generic read completion cannot complete them. No lifecycle enqueue
  HTTP route, provider write transport or execution worker is implemented yet.
- History/native contracts understand uncertain state and a task-known boolean;
  raw UPIDs remain private and Debug-redacted. Documentation records this boundary.
- make check-server passes fmt, warnings-denied Clippy and 88 tests, including
  eight new SQLite tests for binding, expiry/clock rollback, concurrency, dispatch
  and crash recovery, task validation/redaction, terminal immutability, corruption
  rollback and schema-4 preservation. Via the temporary signed-package prefix,
  make check-rust passes all 103 workspace tests (88 server, 15 native).
- Next single slice: fixed verified-TLS start/graceful-shutdown transport with
  retries disabled, exact returned task identity checks and local HTTPS tests.
  Wire durable worker/API only after no-replay recovery is covered. Real VM
  operations still require separate concrete approval; GUI/mobile gates remain.


## Fixed lifecycle provider transport — 2026-10-18

- Previous goal turn made verified progress: schema 5, eight lifecycle storage
  tests, passing server/client/native checks. This slice added actual fixed
  provider transport, without exposing service execution or touching real VMs.
- Checked current official upstream src/PVE/API2/Qemu.pm for POST paths, form
  parameters, task kinds and permission checks. An initial old source path failed;
  the correct source includes src/. No upstream source was copied into the repo.
- Added typed start and graceful-shutdown requests. Start sends an empty form;
  shutdown sends exactly timeout=60&forceStop=0. No force/skiplock/passthrough.
- Explicitly disabled reqwest retries on the shared verified-TLS client. Redirects
  remain refused. GET and POST share the bounded safe envelope decoder.
- Returned UPIDs must bind node, canonical VMID and qmstart/qmshutdown kind.
  Unexpected responses stay safe errors; callers must treat post-dispatch
  ambiguity as uncertain. No raw response/UPID enters public operation records.
- Extended the disposable HTTPS fixture to consume request Content-Length;
  three tests exercise both exact POST bodies/paths/auth, malformed/unbound tasks,
  permission/server/redirect errors, trust rejection, timeout and response bounds.
  Error fixtures see no repeated request. HTTP/2 retry signals are not simulated;
  explicit retry::never policy addresses internal client retries.
- make check-server passes formatting, all-target warnings-denied Clippy and all
  91 tests. git diff --check passes. Client source was unchanged in this slice;
  the earlier 103-test workspace result predates these three server tests.
- docs/lifecycle-execution.md records contracts, evidence and boundaries. No
  roadmap item is newly marked complete: integrated execution is still missing.
- Next single slice: default-disabled lifecycle executor linking fresh scoped
  preconditions to committed dispatch, persisted task and GET-only reconciliation;
  exercise local crash/no-replay flows before wiring the confirmed HTTP route.


## Durable lifecycle executor — 2026-10-18

- Previous goal turn was verified progress (fixed provider transport and 91
  passing server tests). This sequential slice links the durable state machine
  to actual local-HTTPS start/shutdown requests and subsequent observations.
- New lifecycle executor checks capability, retained provider scope/target, fresh
  preconditions and approved digest. Approval expiry is rechecked while committing
  the dispatch guard. It persists task linkage before polling, then separately
  verifies resulting power. Task failure is terminal; ambiguous post-dispatch
  outcomes are uncertain. No guest readiness/SSH success is inferred.
- Retained tasks reconcile by GET even with writes disabled; unknown references
  are never automatically reclaimed. Storage failures propagate rather than
  allowing the service worker to silently lose durable state. Database operations
  use spawn_blocking. The default worker invokes this only after idle read claims
  and always passes false for write capability. No HTTP action route exists.
- Six integrated HTTPS/SQLite executor tests exercise start/shutdown success,
  zero-POST disabled/scope/revision/expiry/foreign-marker rejection, provider task
  failure, malformed submission reply, independent power mismatch, same-key
  identity and conflicting uncertain actions. Local worker cancellation after
  POST receipt and after task linkage is followed by SQLite reopen/recovery; no
  POST repeats. Known-task diagnosis occurs with writes disabled. A seeded
  post-link crash boundary reaches success using task/power GETs only.
- These are async worker cancellation/connection-reopen fixtures, not actual
  process-kill or real provider mutation evidence. All credentials/certificates
  are disposable fixture data. No real VM, credential or infrastructure changed.
- make check-server passes fmt, warnings-denied Clippy and all 97 tests. Client
  code is unchanged; latest native workspace check remains the earlier 103-test
  result. git diff --check passes. Execution docs/ADR record the new boundary.
- Next single slice: validated default-disabled runtime capability and fair
  sequential worker scheduling, then confirmed enqueue API with local HTTP/TLS
  and real binary restart/no-replay coverage. Continuous read traffic currently
  may delay lifecycle reconciliation. Real-target authorization remains separate.


## Runtime lifecycle opt-in and fair worker rounds — 2026-10-18

- Previous turn made verified progress: integrated local executor and 97 passing
  server tests. This slice wires validated runtime capability into the real binary
  and removes read-queue starvation of lifecycle reconciliation.
- Config defaults lifecycle to false; VMANYWARE_LIFECYCLE_ENABLED accepts only
  exact lowercase true/false. Invalid values emit a fixed startup reason before
  provider/database initialization and never echo the value. Loopback-only binding
  is unchanged. Explicit true still requires valid persisted confirmation, scope,
  revision, marker/preconditions and expiry before any new provider dispatch.
- Main passes this capability to one sequential worker. Each round processes at
  most one read and one lifecycle claim; long bounded requests can delay work but
  continual reads cannot indefinitely exclude lifecycle claims. Idle rounds wait
  250 ms. Default library runner remains disabled; retained task GET reconciliation
  is independent of permission to start new writes.
- Tests: exact config parsing/default/redaction; repeated rounds completing both
  queues under true/false; actual cleared-environment binary startup rejects invalid
  settings without creating database/lock; absent/false/true binary runs process
  preseeded intent with no provider, proving option-to-worker wiring and persisted
  failures without acquiring dispatch markers. SIGTERM exits successfully.
- make check-server passes fmt, all-target warnings-denied Clippy and 101 tests.
  git diff --check passes. No client code, dependency, real VM or credential changed.
  Latest native full workspace evidence remains the earlier 103-test result.
- No enqueue HTTP route exists yet and preflight executionEnabled remains false.
  These subprocess tests do not prove real binary provider POST/restart recovery;
  async worker restart fixtures from the prior slice remain the current evidence.
- Next single slice: explicit full-target/action confirmed start/shutdown enqueue
  routes with default-disabled API policy and idempotency/error tests, then local
  TLS-backed real binary success/interruption/no-replay verification.


## Confirmed lifecycle HTTP enqueue — 2026-10-18

- Previous turn delivered verified runtime opt-in and fair sequential worker
  rounds. This slice exposes fixed POST /api/v1/environments/{id}/start and
  /shutdown with a complete explicit Approval body, global idempotency key and
  4096-byte bound. Origin, unknown fields and mismatched path/action/identity are
  rejected. Default routers disable execution; main shares validated capability
  with HTTP state and worker. No force-stop/generic mutation route exists.
- Handler checks capability, configured scope and saved target and commits queued
  intent before HTTP 202/Location, making no provider request itself. Same key/body
  returns the original operation; changed input/cross-kind/active conflicts fail.
  Expiry and fresh marker/config/power conditions remain worker responsibilities
  checked before the durable dispatch marker. Existing GET history/detail still
  retrieves outcomes if enqueue becomes disabled or provider configuration changes.
- Preflight API now explicitly reports configured capability (default false),
  independently of preconditions. Renderer/native contracts accept both booleans;
  renderer displays capability and still has no mutation control. An enabled
  report does not confirm or trigger any action.
- Two HTTP/TLS tests cover disabled/missing-key/origin/provider rejection, target/
  action/hash/VMID/scope binding, unknown/oversized input, idempotency/conflicts,
  zero-network enqueue, accepted request -> actual executor -> one fixture POST
  -> persisted task/power result, and enabled preflight reporting. Real binary
  diagnostic coverage additionally verifies static/redacted start/shutdown routes.
- Native full check initially failed its obsolete rejection-of-true assertion;
  updated that contract test to accept both boolean values while rejecting string
  capability/unrelated identity. Final prefix make check-rust passes fmt/Clippy
  and all 118 tests (103 server, 15 native); final make check-server also passes
  103 tests after added diagnostic coverage. make check-instances-browser passes
  build, all 16 Node tests and disposable Firefox simulated-IPC walkthrough.
  git diff --check passes. No new dependency, real VM or credential change.
- Real process lifecycle provider POST/restart behavior and approved real-host
  least-privilege execution remain unverified; previous async cancellation tests
  are not substituted for them. Native lifecycle commands/UI remain pending.
- Next single slice: generated-HTTPS real binary success, interruption with and
  without persisted task, restart/reconciliation and exactly-once dispatch evidence.
  Then add native explicit action confirmation/monitoring. Keep real tests opt-in.


## Real binary lifecycle start and restart — 2026-10-18

- Previous turn delivered confirmed HTTP enqueue and final 118 passing workspace
  tests plus client/browser evidence. This slice adds actual server-process
  lifecycle success/recovery evidence, beyond async worker cancellation fixtures.
- Added a disposable HTTPS lifecycle peer with generated certificate/fake token,
  explicit fixture marker, separately observed power/task state and one held POST
  or task-status response. Peer state stays in memory; it never touches a VM.
- Real cleared-environment binary saves definition/registers association, fetches
  enabled passing preflight, sends confirmed enqueue and persists task success/
  running power. Duplicate same-key requests return the same terminal operation.
- SIGTERM while POST reply is held leaves the durable dispatch marker but no UPID;
  actual process restart preserves uncertainty, same-key identity and new-key
  conflict. A 600 ms observation spans more than two idle intervals with no new
  provider request. Unknown submission is never automatically replayed.
- SIGTERM during the first task GET proves UPID linkage already persisted;
  actual restart with new writes disabled reconciles task/power by GET to success.
  Every scenario records exactly one POST; final SIGINT exits successfully,
  association snapshots stay unchanged and logs omit fixture token/raw UPID.
- make check-server passes formatting, all-target warnings-denied Clippy and all
  104 tests. git diff --check passes. Client/native source unchanged this slice;
  latest full workspace evidence remains 118 tests from the preceding slice.
- These are local start/normal exit/restart fixtures, not SIGKILL/power-loss fsync,
  real Proxmox, least-privilege ACL, shutdown subprocess or guest-readiness proof.
  Shutdown remains locally covered in the transport/executor TLS tests. No actual
  infrastructure/credential mutation occurred; real approval boundaries persist.
- Next single slice: typed native confirmed action command and bounded operation
  monitoring, then explicit target/action confirmation controls with same-key
  resume/uncertain handling and simulated-IPC browser evidence.


## Native confirmed lifecycle commands — 2026-10-18

- Previous goal turn delivered verified real binary local-HTTPS start/restart
  no-replay evidence and 104 server tests. This slice adds registered Tauri
  begin_lifecycle_action/read_lifecycle_operation commands, without a new UI flow.
- Typed deny-unknown-fields full approval accepts only start/shutdown, exact
  lowercase hash/ID formats, validated node/VMID and confirmed=true. Input/key
  validation occurs before network. Fixed loopback POST sends JSON and the original
  idempotency key; native reqwest retries are now explicitly disabled as well.
- GET-only operation resumption binds requested operation ID and complete original
  target context. Responses validate kind/instance/state/result/failure consistency,
  retained task presence and separate successful task plus matching action/scope/
  node/VMID/desired power. Only known failure codes pass; fixed app-owned lifecycle
  errors replace raw server messages. Unknown/inconsistent responses fail safely.
- Three native HTTP tests cover both exact action requests, JSON/key headers,
  uncertain and successful GET resumption, wrong operation/target/evidence, invalid
  inputs before transport, unknown fields and safe error mapping.
- Prefix make check-rust passes fmt, warnings-denied Clippy and all 122 workspace
  tests (104 server, 18 native). git diff --check passes. Frontend unchanged; latest
  frontend build/16 Node tests/Firefox simulated-IPC result is from the prior API
  slice. No dependency, real VM, infrastructure or credential changed.
- Native GUI/WebView lifecycle actions remain unverified. Source is compiled and
  command HTTP tests run on Linux; that does not prove a user-facing action flow.
- Next single slice: client confirmation and bounded lifecycle monitoring with
  same-key resume, explicit uncertain handling, stale target/port suppression and
  Node/browser simulated-IPC evidence. Never automatically reconfirm or replace
  a key after transport loss. Real-provider execution stays separately opt-in.


## Client lifecycle monitoring controller — 2026-10-18

- Previous turn delivered typed native commands and 122 passing workspace tests.
  This small sequential slice implements standalone client intent/monitoring logic
  before adding React controls; no automatic invocation or user-facing action
  was introduced.
- confirmedLifecycle builds a full target/digest/action/key only for an explicitly
  confirmed enabled report with passing common and action power/QMP conditions.
  Cached intent validation can additionally bind the current saved instance.
- monitorLifecycle snapshots context and requires persistence before transport.
  Unknown IDs reuse original approval/key; acknowledged IDs use GET only. Every
  response validates kind/ID/scope/action/task/power and known failure codes.
  Acknowledgements retain IDs even after leaving, without stale UI updates.
- Polling pauses after a monotonic 60-second interval plus at most one native
  bounded in-flight request. Leaving stops later reads. Uncertain returns a
  distinct outcome and retains context; terminal records also remain for explicit
  review. No error, pause or uncertain state automatically replaces a request key.
- Seven Node tests cover confirmation blockers and bound inputs, persistence before
  POST, one-ID polling, lost reply/same-key resume, GET-only known/uncertain resume,
  deadline/leave behavior, late ack persistence and malformed evidence rejection.
- Initial build rejected explicit .ts import syntax; enabled the compiler option
  under existing noEmit mode so direct Node source tests and TypeScript build use
  the same module. No runtime dependency or lock changed.
- Final make check-client passes TypeScript/Vite build and all 23 Node tests.
  git diff --check passes. Rust unchanged; latest workspace remains 122 passing
  tests. No native GUI, browser action flow or real infrastructure test is claimed.
- Next single slice: React explicit target/action confirmation and per-port/instance
  pending-intent persistence, resume and uncertain display. Protect stale views
  and block fresh confirmation while a prior intent needs review; then exercise
  the flow with disposable Firefox simulated IPC.


## React lifecycle confirmation and resume — 2026-10-18

- Previous turn delivered standalone lifecycle intent/monitoring logic and 23
  passing Node tests. This slice connects it to per-instance React controls.
- Enabled/passing preflight exposes review for allowed start/graceful shutdown.
  Review displays exact instance, service port, node/VMID, scope, definition and
  config digest; initially unchecked consent plus a separate confirm click is
  required. Cancel performs no command. Changed reports reset review.
- Full pending context persists by port/instance before transport; corrupt or
  unavailable storage blocks fresh actions. Existing pending intent prevents a
  new key. Known IDs resume by GET; unknown IDs explicitly explain same-request
  retry. Local collision checks do not claim cross-window transactional locking;
  server idempotency/exclusion/preconditions remain authoritative.
- Uncertain outcomes cannot be acknowledged/cleared into fresh actions. Terminal
  success/failure/interruption require explicit result acknowledgement, which
  clears only local pending context and invalidates preflight. Late ack retains
  its scoped ID without updating a departed port view. No automatic resume on load.
- Strengthened disposable Firefox simulated-IPC walkthrough proves start review/
  consent/cancel/success, exact scope/revision display, result acknowledgement and
  fresh-check requirement, lost reply preserved across port remount, original-key
  retry, uncertain GET reconciliation, late ack/stale suppression and graceful
  shutdown/stopped power. Initial fixture still expected the old no-actions text;
  updated its disabled-capability assertion and added enabled scenarios. Final
  make check-instances-browser passes build, all 23 Node tests and the walkthrough.
- git diff --check passes. Rust unchanged; latest 122 workspace tests remain valid
  native-command evidence. No actual native GUI/provider action is claimed, no
  real infrastructure/credential/dependency changed.
- Next single slice: inspect the available Linux native GUI/WebView harness and
  exercise lifecycle confirmation -> real Tauri IPC -> disposable loopback server
  if locally available. Keep GUI evidence distinct from Firefox simulation and
  real Proxmox tests; continue remaining provisioning roadmap work afterward.


## Real Linux native lifecycle UI/IPC — 2026-10-18

- Previous goal turn delivered React confirmation/resume and passing Firefox
  simulated IPC. This slice closes an additional local integration gap with real
  WebKit/Tauri IPC and actual server/provider transport, without touching real VMs.
- Verified/extracted three signed Arch display packages into the owned temporary
  prefix (libxmu/xorg-xauth/Xvfb); retained manifest/signatures in display-packages.json.
  No root/system package installation or credential inspection occurred.
- Added opt-in lifecycle_smoke example with production command module, built React
  assets, fixture-only scenario/reporting and separate ephemeral app identity.
  Runner owns Xvfb auth cookie/display, XDG directories, D-Bus, generated HTTPS
  certificate/key/fake token and actual Axum database/process. A read-only user
  namespace overlay supplies WebKit helper paths; existing home directories hide.
- Native UI checks real Linux bridge, list/preflight, review/cancel/consent/confirm
  start, task/running power, acknowledgement/fresh check and graceful shutdown to
  stopped power. Runner requires exactly one POST per action, unchanged association
  and safe server logs. IPC is never mocked. No production fixture flag was added.
- Initial Xvfb initialization crashed in host NVIDIA EGL; explicit software Mesa
  fixes this test backend. Initial OpenSSL fixture certificate CA constraints
  caused verified-TLS rejection; generated server-auth end-entity constraints fix
  it without trust bypass. Minimal D-Bus/a11y/portal warnings remain nonfatal.
  WebKit sandbox, TLS/hostname verification and X11 cookie auth stay enabled.
- Rustfmt found an ambiguous imported module test path; explicit control_plane/
  tests.rs path fixes formatting/all-target compilation. Final prefix make
  check-rust passes fmt/Clippy and 122 tests. Complete make check-lifecycle-native
  target passes frontend build/23 Node tests, native/server builds and real GUI
  start/shutdown fixture. git diff --check and Python syntax parsing pass.
- This proves isolated Linux software-rendered lifecycle UI/IPC. Normal packaged
  startup, native app restart/persistent profile, GPU, macOS/mobile, guest readiness
  and real least-privilege Proxmox mutations remain distinct open evidence gates.
- User again requested nonstop continuation; goal remains active without waiting
  for another prompt. Next single slice: inspect existing provisioning plan/
  preflight contracts and implement durable explicitly confirmed clone intent/
  execution against local HTTPS fixtures, preserving no-replay and real-target
  approval boundaries before Cloud-Init/resource/start orchestration.


## Phase 3 fixed full-clone transport — 2026-10-18

- User explicitly asked to keep going and try finishing the roadmap. Active goal
  remains unchanged; real/platform gates remain evidence-based rather than being
  marked complete to satisfy scope. Previous turn made progress with real Linux
  native lifecycle UI/IPC and passing full native/build/browser checks.
- New slice derives fixed CloneRequest from existing validated deterministic plan
  plus instance ID. Same-node full clone sends only newid/name/storage/full=1 and
  a generated instance/definition/plan description. No linked clone, arbitrary
  fields, tags/pool/snapshot/cross-node override or new public API route.
- Rechecked official upstream Qemu.pm: qmclone UPID binds SOURCE VMID, unlike
  start/shutdown target VMID. Transport validates exact source/node/kind/canonical
  ID. Wrong destination/source aliases, foreign node or action are rejected.
- Post-task target observation independently reads current config then status
  within 20 seconds: exact description/name/digest, non-template, no lock, stopped
  power and absent/stopped QMP. It does not claim ownership authorization, resource
  configuration, Cloud-Init or guest readiness.
- form_urlencoded is a new direct server dependency for dynamic multiline forms;
  it already existed transitively in Cargo.lock. Lockfile changes only the server
  dependency edge, with no new resolved version/crate. ADR 0024 records this choice
  and future durable stage/reservation/no-replay contract.
- Three HTTPS tests cover exact encoded form, source-task binding, invalid replies,
  auth/provider/redirect errors, timeout/size bounds, no observed replay and target
  identity/lock/template/digest/power/QMP failures. make check-server passes fmt,
  warnings-denied Clippy and all 107 tests; git diff --check passes. Native/client
  unchanged; latest full workspace evidence is the preceding 122-test result.
- No transport caller exists in service orchestration yet, and no real VM or
  credential was accessed/changed. Clone success cannot complete Phase 3 without
  durable provisioning intent, resource/disk/Cloud-Init stages and readiness.
- Next single slice: durable full-plan provisioning intent with canonical definition
  snapshot, local scope/VMID exclusion, per-stage dispatch/task retention and
  interruption recovery, then connect the clone stage to local HTTPS fixtures.


## Schema 6 provisioning intent and journal — 2026-10-18

- User authorized autonomous continuation after establishing diff-scoped checks.
  Scope: durable full-plan inputs, local target exclusion, ordered stage journal,
  clone dispatch/task guards and restart recovery. No provider execution was wired.
- Migration 6 adds vm_provision to existing operations and retains approval/hash/
  server timestamp, canonical instance definition snapshot, catalog and exact plan.
  Plan stages are persisted in order; only clone currently has claim/dispatch/link
  methods. Journal/task data is validated against regenerated retained inputs.
- Association allocation shares one immediate transaction with operation/journal.
  Existing scope/VMID uniqueness excludes manual registration/other provisioners;
  status registered continues to mean only local association, not VM existence.
  Incomplete provisioning blocks lifecycle writes through a database trigger,
  including failed/interrupted intent; diagnostic observations remain permitted.
- Global key/body binding returns original retained plan/timestamp on retry even
  with changed current catalog. New inputs must match current definition/catalog/
  plan. Dispatch commits workflow+clone stage markers once within original 120s
  approval; task linkage validates SOURCE qmclone identity and cannot be replaced.
- Startup recovery now transactionally updates running stages and operations.
  Known dispatched tasks permit future read-only reconciliation; unknown clone
  submissions remain uncertain and unclaimable. No confirmation is renewed.
- Verification: new intent suite first passed five of six tests; migration fixture
  used obsolete SQL column names, corrected to definition_yaml/definition_hash/
  revision and reran only that test successfully. New tests cover snapshots/order,
  binding/exclusion, dispatch/recovery/task redaction, expiry/clock rollback, corrupt
  claim rollback, concurrent keys and schema-5 lifecycle preservation/FK integrity.
- Five affected compatibility tests passed: schema-2 instance migration, running
  read restart, and three lifecycle recovery/task cases. Earlier schema-4 migration
  test also passed after updating expected schema to 6. Scoped library/new-test
  Clippy passes; its boolean assertion lint was corrected. Changed-file rustfmt and
  git diff --check pass. No broad server/client/native/GUI suite was rerun: earlier
  107-server/122-workspace evidence remains historical rather than a new full run.
- No dependency, real VM, infrastructure or credential changed. No provisioning
  HTTP route/worker or completed provisioned VM is claimed. Failed intent currently
  retains its target association until a future explicit recovery/release policy.
- Next single slice: clone-stage completion/failure/uncertain persistence and
  executor linking retained intent, fresh provider/plan checks, one POST and GET-only
  task/target reconciliation against local HTTPS fixtures. Keep creation disabled
  until later resource/Cloud-Init/readiness stages are implemented.


## Durable clone execution and completion — 2026-10-18

- Scope: finish only the clone stage against generated HTTPS fixtures, without
  exposing provisioning writes through an API/worker or advancing later stages.
- Executor checks current catalog/plan, seven observational preflight gates,
  source digest, lock/pending state and stopped power before one durable dispatch.
  Public preflight still reports unknown provenance/permissions/readiness; these
  clone gates do not establish full provisioning feasibility or guest readiness.
- Retained tasks reconcile by GET with new creation disabled. Unknown submissions
  cannot replay. Task failure and ambiguous task/target results persist distinct
  fixed failures; a five-second delay bounds known-task reconciliation frequency.
- Completion validates retained task, scope/instance/plan and exact stopped target
  observation. Only clone becomes succeeded with evidence; workflow stays running,
  result remains absent and later stages stay queued. Restart preserves completed
  clone evidence without allowing another clone claim. Reservations remain held.
- Verification: original six intent tests passed; two new outcome tests passed;
  completed-stage restart assertion then passed separately. Four generated-HTTPS
  executor tests cover disabled/changed inputs, source gates, task success/failure,
  bad target marker, ambiguous submission and disabled GET-only recovery. Nine
  affected preflight tests and scoped library/intent-test Clippy passed. No broader
  server/client/native suite was rerun; previous full-suite counts are historical.
- No real infrastructure/credential mutation, new dependency or production caller.
- Next single slice: fixed resource configuration transport and durable stage
  completion/reconciliation on the exact stopped clone, without replaying clone.


## Stopped-clone CPU/memory transport — 2026-10-18

- Scope: fixed digest-bound CPU/memory update and applied-value observation on the
  exact stopped clone. No production resource executor/stage transition was added.
- Plan builds validated inputs with generated identity and retained revision.
  Transport rechecks clone marker/name/digest/template/lock/power/QMP and pending
  changes, then sends one synchronous PUT with fixed topology, vCPU count, no CPU
  limit, requested MiB and disabled ballooning. Proxmox digest comparison detects
  concurrent configuration changes; external power transitions are not locked.
- Observer requires exact applied values and matching digests across settings and
  independent clone observation, with no pending changes. Reconciliation is GET
  only and allows a new applied revision. CPU/memory evidence implies no readiness.
- Official upstream QEMU synchronous API reviewed; ADR 0026 records semantics and
  limitations. Four targeted HTTPS tests pass after adding explicit CPU-limit
  handling: exact form, pre-write target gates, observation values/revisions, and
  no replay on malformed submission replies. Scoped library Clippy passes. Prior
  clone/storage/client/native suites were not rerun. No new dependency or real VM
  mutation; production provisioning remains unavailable.
- Next single slice: durable resource-stage claim/dispatch/completion and GET-only
  uncertain reconciliation using clone completion evidence; never replay clone.


## Durable CPU/memory execution and recovery — 2026-10-18

- Scope: advance only from validated clone evidence into one resource update,
  persist its revision/dispatch boundary, reconcile uncertainty by GET and retain
  stage completion separately from incomplete whole-plan status.
- Migration 7 adds a private stage config_digest with resource-kind/dispatch/hex
  constraints. Claims validate clone evidence and bindings and roll back corrupt
  state. Retained resource revision must match the completed clone revision.
- Resource stages interrupted before dispatch may resume after fresh checks;
  dispatched stages always reconcile by GET, without a resource UPID, even with
  new creation disabled or no current catalog. Five-second backoff limits repeats.
  Earlier clone task stays retained as history and is never replayed.
- Full-plan approval expires for first clone dispatch; subsequent retained-plan
  resource work can outlast 120 seconds without renewing confirmation or changing
  inputs. First resource dispatch requires capability/current plan and exact
  stopped-clone revision/pending checks, then commits the marker before PUT.
- Fixed observations can prove the requested state after success, malformed reply
  or actual connection loss. Unproven outcomes remain uncertain, never repeat PUT.
  Completion binds identity/plan/CPU/memory/stopped target and saves only resource
  evidence. Workflow remains running, result absent; all following stages queued.
- Verification: 8 affected pre-existing intent cases passed; 5 new SQLite cases
  passed in focused runs, including restart, corruption/rollback and schema-6
  migration. After refining pre-dispatch resume, only that changed test was rerun.
  Three HTTPS executor tests pass, including real connection-close injection and
  one-PUT counting. Two affected old migration cases and future-schema refusal
  pass. Scoped library/intent-test Clippy, changed-file rustfmt and diff review
  pass. No broad server/client/native suite or real VM test was run.
- No new dependency, infrastructure/credential mutation or production caller.
  ADR 0027 records approval lifetime and synchronous no-task recovery semantics.
- Next single slice: fixed disk expansion transport and exact post-expansion
  observation, using retained resource completion and the mapped disk device.


## Fixed mapped-disk expansion transport — 2026-10-18

- Scope: fixed absolute-size expansion and configured-size observation for only
  the catalog-selected primary device and exact observed volume/revision.
- Plan builds bounded inputs on the mapped target storage. Same-size/non-expansion
  requests are rejected; no disk stage is planned when source/target are equal.
  Transport verifies current CPU/memory, stopped clone identity, pending changes,
  consistent digests and exact original source volume/size before one PUT.
- Official upstream resize API reviewed: returned resize task binds TARGET VMID,
  unlike qmclone. Wrong node/action/source/canonical-ID aliases fail validation.
  Form contains only mapped disk, absolute requested GiB and digest; no relative
  increment, skiplock or generic fields. Existing TLS/retry policy is unchanged.
- GET-only post-task observation requires exact configured size/volume/device and
  independently consistent current clone/resources. Integer byte/K/M/G/T syntax
  accepted; duplicates, fractional/overflow/missing sizes and CD-ROM media rejected.
  This proves no backend allocation, guest partition/filesystem growth or readiness.
- Four targeted HTTPS tests pass (first three together, added mapped-device case
  separately), covering fixed form/task identity, before-PUT failures, exact size,
  actual connection loss/malformed replies with one PUT, all mapped devices and
  file-backed volume. Scoped library Clippy, changed-file rustfmt and diff review
  pass. Earlier unrelated clone/storage/client/native checks were not repeated.
- No schema/dependency or real infrastructure/credential change. No production
  caller or completed disk journal step; ADR 0028 records this boundary.
- Next single slice: persist exact disk volume/revision and dispatch/task linkage,
  then execute/reconcile only expand_disk after validated resource completion.


## Schema 8 retained disk dispatch and journal outcomes — 2026-10-18

- Scope: durable exact volume/revision, disk-stage claim/dispatch/task/completion
  and recovery guards, after validated clone/resource evidence. No executor wired.
- Migration 8 adds a separate private disk-dispatch record; resource-only digest
  constraints remain intact. Reader validates volume inputs, resource revision and
  target-bound resize tasks. Invalid evidence/dispatch/task causes claim rollback.
- Undispatched queued/interrupted disk work can resume. Dispatched uncertainty can
  be claimed only with a known task after backoff; unknown submissions never replay.
  Begin atomically saves volume/revision and marker, clearing current task while
  retaining the earlier clone task on its stage. Linking is bound and immutable.
- Completion binds scope/instance/plan/task, exact device/volume/bytes, CPU/memory
  and stopped target; only disk becomes succeeded. Whole operation stays running
  without result, later stages queued. Definite auth refusal or known failed task
  may fail; ambiguous outcomes remain uncertain. Associations remain reserved.
- Verification: 13 affected existing intent/resource tests passed after migration
  and reader changes. Six new journal tests passed in targeted groups, covering
  binding, completion, no replay, restart, failure classification, corrupted state
  rollback and schema-7 upgrade/FK preservation. Two affected schema-2/schema-4
  compatibility cases pass. Scoped library/intent-test Clippy and changed-file
  rustfmt/diff checks pass. No unrelated HTTPS/frontend/native suite repeated.
- No dependency, actual VM, infrastructure or credential changes. ADR 0029 records
  storage guarantees separately from pending disk execution and real size proof.
- Next single slice: standalone disk executor with fresh exact-volume/resource
  checks, one PUT, durable resize task linkage/polling and GET-only reconciliation
  against generated HTTPS fixtures; keep production provisioning unavailable.


## Standalone disk execution and HTTPS recovery — 2026-10-18

- Scope: connect the disk journal to fresh provider checks, one PUT, durable resize
  task retention/polling and independent configured-size observation. No public
  enqueue route or production worker was enabled.
- New-dispatch capability, scope and current catalog gate volume selection. Only
  the mapped drive is selected; full clone/resource/source-size/revision/pending
  checks run before the marker and again in transport after its commit.
- Known tasks reconcile using retained inputs with GET only, even if new creation
  is disabled or current catalog absent. Unknown replies, including actual HTTPS
  connection loss, remain unclaimable submissions. A post-marker config change
  prevents PUT and still cannot authorize automatic replay.
- Success requires task success plus exact configured volume size and retained
  target/resource bindings. Disk alone succeeds; operation stays running without
  result and later stages remain queued. Task failure and observation uncertainty
  retain distinct safe failures and the local target association.
- Verification: 3 initial HTTPS executor tests passed; an added post-marker race
  test passed separately. Cases include one PUT, failed/unknown/foreign tasks,
  size/marker mismatch, disabled known-task versus unknown restart, and first-write
  capability/catalog/scope/volume/revision/resource/pending/power gates. Four
  affected disk transport tests pass after extracting the shared read-only guard.
  Scoped library Clippy, changed-file rustfmt and diff review pass. No unrelated
  storage/client/native or full suite was repeated.
- No schema/dependency, real infrastructure or credential change. No backend
  allocation, guest filesystem growth, readiness or full provisioning is implied.
- Next single slice: fixed network/Cloud-Init transport and independent applied
  settings observation, retaining strict SSH-key/identity checks. Keep documented
  scalar-user compatibility and real first-boot evidence separate from API setup.


## Fixed network and Cloud-Init settings transport — 2026-10-18

- Scope: bounded definition/plan-bound network/user/public-key settings and exact
  independent observation, without durable stage/worker/API activation.
- Supports one virtio net0; preserves its observed MAC/firewall and uses catalog
  bridge. Rejects extra NIC/IP configs and unmodeled VLAN/rate/MTU/etc. Requires
  exact retained primary volume/requested disk size and target-specific ide2
  Cloud-Init drive, current resources/stopped identity/revision/pending checks.
- Fixed synchronous PUT sets IPv4 DHCP, NoCloud, catalog user, disabled implicit
  upgrades and canonical public keys; deletes inherited cicustom/cipassword.
  Public-key comments are removed while key material and original saved definition
  remain intact. API URI-unescaping requires inner URI + outer form encoding.
- Observer requires exact applied settings and no inherited override/password,
  with independent consistent current resources/disk/drive. Reports key count and
  guest_ready=false; never returns key contents. Request Debug redacts keys.
- Verification: 3 initial targeted HTTPS tests pass; an added multi-key encoding
  test passes separately, comparing decoded key material through both layers.
  Tests cover fixed form, definition binding, source gates, applied settings and
  actual dropped/malformed reply with one PUT. Scoped library Clippy and changed
  rustfmt/diff review pass. No unrelated disk/storage/client/native suite repeated.
- No schema/dependency or real VM/infrastructure/credential change. ADR 0030
  records supported template shape and the still-open scalar-user compatibility
  requirement from ADR 0017. Built-in API setup does not implement the lab's
  corrected target-specific snippet or prove first-boot readiness.
- Next single slice: durable configure_cloud_init stage after validated resource
  and optional disk evidence, retaining observed network/volume/revision for
  one-shot dispatch and GET-only uncertainty recovery. Keep seed compatibility
  and strict guest first-boot readiness as subsequent uncompleted requirements.


## Schema 9 durable Cloud-Init API-settings journal — 2026-10-18

- Scope: validated prior-step evidence, exact observed network/volume/revision
  retention, one dispatch marker, completion/failure/uncertainty and recovery.
  No provider executor or production worker/API is wired in this slice.
- Migration 9 adds private Cloud-Init dispatch inputs. Prior resource completion
  supplies the source revision when no expansion is planned; validated disk task/
  completion supplies revision and exact volume otherwise. Claims require every
  preceding planned stage succeeded and reject corrupted evidence transactionally.
- Begin validates the original full-plan approval and fixed request, then commits
  inputs and marker together. Clears current task while retaining old stage tasks.
  Dispatched work cannot begin again; uncertainty allows future GET-only recovery
  without UPID. Undispatched interrupted work may resume; backoff is preserved.
- Completion binds identity/plan, primary disk, network, user/key count and current
  stopped resources/revision, requiring guest_ready=false. Only API-settings stage
  succeeds; full operation stays running with no result and later stages queued.
  Post-marker errors cannot become definite failures. Reservations remain held.
- Verification: 19 affected prior journal tests pass with migration/reader changes.
  Five new focused SQLite cases pass for optional expansion, binding/restart,
  corruption rollback, schema-8 upgrade and failure exclusion. Clippy found the
  larger success enum variant; boxed its result and reran all five changed cases
  plus scoped library/intent-test Clippy successfully. Two affected older migration
  cases and changed-file rustfmt/diff checks pass. No broad/client/native rerun.
- No new dependency or real VM/infrastructure/credential mutation. ADR 0031 records
  storage guarantees separately from future execution, generated seed compatibility
  and strict guest readiness; the known scalar-user issue remains unresolved.
- Next single slice: standalone Cloud-Init executor linking current catalog/target/
  network/disk checks, one synchronous PUT and GET-only restart reconciliation to
  generated HTTPS fixtures. Keep production provisioning unavailable.


## Standalone Cloud-Init settings execution — 2026-10-18

- User asked to resume; scope connects validated journal inputs to current
  provider/catalog/target checks, one synchronous PUT and independent observation.
  No production worker/API creation caller was activated.
- Executor selects only mapped volume and net0, verifies optional disk identity,
  then checks current source revision, resources, disk size, Cloud-Init drive,
  stopped target and pending changes. Commits original observed inputs/marker
  before transport repeats guards and attempts the fixed configuration update.
- Missing/malformed response still permits exact independent settings observation;
  unproven results stay uncertain. Restart reconstructs only retained inputs and
  uses GET with new creation disabled and current catalog absent. Prior stages
  never replay. Settings success leaves full workflow running without result;
  start/later steps remain queued and guest_ready stays false.
- Verification: 3 targeted HTTPS executor tests passed. The new pre-marker gate
  fixture initially used an unsuitable sequential response order; changed it to
  path-based reads and reran that case successfully. Tests cover one PUT, actual
  dropped/malformed replies, applied/uncertain outcomes, disabled GET-only restart
  and new-write capability/catalog/scope/volume/revision/network/drive gates.
  Three affected transport tests pass after sharing their read-only guard. Scoped
  library Clippy and changed-file rustfmt/diff checks pass. No unrelated suite run.
- No schema/dependency, real infrastructure or credential mutation. Compatibility
  of generated seed and strict guest first boot remain unfinished; this slice does
  not reproduce the lab's corrected custom snippet or weaken readiness.
- Next single slice: inspect generated Cloud-Init seed via fixed read-only API,
  validate its user/key/network semantics and implement a pre-start compatibility
  gate. Evaluate the image default-user path before adding snippet transport; any
  change must retain the declared guest-user/key contract and strict readiness.


## Read-only generated seed semantic preflight — 2026-10-18

- User authorized continuation. Scope: fixed generated user/network previews,
  validated semantics and explicit rejection of known incompatible scalar users;
  no seed write, start implementation or first-boot/readiness claim.
- Reviewed official QEMU dump and Cloudinit generator: dumps are built-in generated
  previews, not custom snippet or attached-ISO reads. Existing applied-settings
  observer excludes overrides. Fixed user/network GETs are enclosed by full target
  settings observations with matching digests under one deadline.
- Strict typed YAML bounds payloads and rejects unknown/duplicate fields, commands,
  passwords, upgrades, wrong hostname/user/key material and unexpected network.
  Only exact network-v1 eth0/MAC/DHCP4 with bounded DNS is supported. Named-user
  shape is a compatible candidate; exact legacy scalar returns incompatible.
- Safe result exposes only revision/payload hashes/classification/compatibility
  and guest_ready=false. Does not expose key bodies, DNS addresses or seed text.
  Known upstream ciuser generation remains scalar and does not pass this check.
- Verification: two targeted HTTPS tests pass with multi-case safe/rejection
  coverage. Initial compile used borrowed query values rather than the client's
  owned query contract; fixed conversion before tests. Updated fixtures to actual
  upstream False spelling and reran only these tests. Scoped library Clippy,
  changed-file rustfmt and diff checks pass. No unrelated suite was repeated.
- No schema/dependency, actual provider write or infrastructure/credential change.
  ADR 0032 separates preview semantics from full schema, seed regeneration/attached
  contents and strict first-boot proof. Start integration remains pending.
- Next single slice: offline proof of an image-default-user generation strategy
  without the deprecated scalar, preserving catalog user and exact public keys.
  Require explicit template/default-user evidence before changing product behavior;
  evaluate corrected snippet lifecycle only if the default-user strategy is invalid.


## Offline image-default-user compatibility proof — 2026-10-18

- User asked to resume. Scope: determine whether omitting legacy user preserves
  declared user/keys when supplied image-default evidence matches; no product or
  provider behavior changes in this slice.
- Verified primary Proxmox generator and pinned Cloud-Init normalizer/reference
  sources. With ciuser absent, built-in generated user-data can use users=[default]
  without the deprecated scalar. This needs the exact image default to match the
  catalog account; distribution name alone is not sufficient evidence.
- Added isolated default-user candidate and check. Renders upstream Debian
  reference defaults (not an actual image), proves scalar/named/omitted-user
  normalization equality and candidate schema validity. Legacy emits one expected
  deprecation; omitted-user emits none. Exact SSH keys/declared user are selected
  by the actual upstream SSH handler with side-effect entry points mocked.
- Modified default resolves to another account and is rejected. Missing/unlocked/
  inherited-key evidence and unexpected password/commands/upgrade seed inputs
  also fail. No host SSH files are scanned or credentials applied by the check.
- Verification: only new check_default_user.py ran against the existing isolated
  upstream 25.1.4 venv/source and passed. No Rust/client/native, old converter or
  unrelated suite rerun. Diff review passes. ADR 0033 records proof limitations.
- No product dependency/catalog behavior, real VM/infrastructure/credential or
  installed system package changed. Strict first-boot and actual image-default
  evidence remain unverified for this alternative.
- Next single slice: explicit optional server-catalog image-default attestation
  bound to guest user/template version, preserving existing mapping hashes and
  behavior. Enable default-user transport/preflight only for opted-in mappings;
  retain strict first-boot/readiness and mark operator evidence as a declaration.


## First mobile journey: control-plane authentication — 2026-10-18

- User explicitly prioritized the smallest create-VM/SSH mobile vertical journey
  before extending features. Scope here: optional whole-API single-operator bearer
  auth, with main-process configuration and safe startup/failure behavior.
- VMANYWARE_ACCESS_TOKEN enables protection for reads, writes, health and fallback
  paths. Missing/invalid/duplicate headers return fixed 401; successful auth reaches
  real routes. Invalid configured tokens fail before database/listener startup.
  No token leaves Debug/errors/logs; request tokens are never accepted in URLs.
- Store only a SHA-256 verifier in middleware; use subtle constant-time comparison.
  subtle 2.6.1 already existed transitively; direct dependency/lock edge added,
  with no new resolved crate/version. Existing default local development remains
  usable without a token; service still binds loopback. TLS/remote profiles pending.
- Verification: 3 focused router/config cases passed, then actual binary fixture
  proved unauthenticated 401, authenticated reads, rejected unauthenticated write,
  safe logs and fail-closed invalid startup with no database created. Child env
  cleared; owned temporary database/token and loopback only, no provider access.
  Scoped lib/bin/auth-test Clippy and changed-file fmt/diff review pass. No unrelated
  server/client/native suite repeated. No real credentials/infrastructure changed.
- Tool discovery: adb/sdkmanager/xcodebuild unavailable on PATH; only host Linux
  Rust target is installed. This is not proof that no SDK exists elsewhere, and no
  mobile compilation/runtime is claimed. Phone-platform preference remains pending.
- Next single slice: native client secure remote endpoint/token connection profile
  and a visible connection check, preserving local mode and keeping secrets out of
  saved frontend state. Then connect the create workflow and mobile SSH backend.


## First mobile journey: remote native connection form — 2026-10-18

- Scope: visible remote HTTPS/token form and native in-memory connection check;
  existing VM controls remain local and are hidden in remote mode to prevent
  cross-server actions. This is not the complete create/SSH mobile journey.
- Native state retains a verified endpoint/client/sensitive bearer header only
  after fixed health succeeds. New attempt/disconnect invalidates old generations;
  stale replies cannot replace a newer connection. URL credentials/path/query/
  fragment/plain HTTP are rejected; redirects/retries and proxy discovery disabled.
- Require server's accepted-auth response marker, added only by the access-token
  middleware. An unauthenticated 200 health response cannot establish a secure
  remote connection. Tokens are cleared from the input on submission and never
  stored in frontend storage. Profile status and Debug omit token material.
- Enabled client's existing reqwest rustls feature. TLS dev fixtures add rcgen,
  tokio and tokio-rustls test usage; all already resolved in the workspace lock,
  with dependency-edge changes only and no new resolved crate/version.
- Verification: 3 filtered native tests pass using the owned Linux WebKit prefix:
  real generated HTTPS/correct bearer, untrusted TLS, redirect/rejection/malformed
  and missing auth marker, invalid URLs, disconnect and stale native generations.
  Affected server auth-marker route case passes. Frontend production build passes.
  Focused real Firefox/React fixture with simulated IPC passes form/token-clearing/
  no-storage, local-control isolation, disconnect and late-result suppression.
  Native/server library Clippy and changed-file fmt/diff checks pass. No broader
  native/server/controller/GUI suite or actual mobile build was rerun.
- No real connection/deployment/VM/credential changes. Native app launch for this
  new command, platform credential storage and actual phone runtime remain open.
- Next single slice: route fixed remote reads through the retained connection ID
  and show remote templates/environments, then attach the minimal create workflow.
  Do not expose local VM mutations through the remote mode or claim it is ready.


## First mobile journey: remote workspace reads — 2026-10-18

- Scope/acceptance: read only the connected server's fixed catalog/environment
  routes; show lists, empty states and independent errors; discard stale reads
  after disconnect. No new VM mutation or infrastructure access.
- Native commands require the retained connection generation, use verified HTTPS
  and bearer authentication, require the accepted-auth marker, bound response
  bodies and validate the supported response shapes. Only fixed GET paths exist.
  Connection locks are released before network waits; stale successes/errors are
  rejected after reads. Catalog sourceDiskGiB spelling matches the server contract.
- Remote UI explicitly loads configured templates and registered environments.
  Registration is labelled as live state not checked; a catalog is not proof of
  template availability. One failed list does not hide the other successful list.
- Verification: 3 new generated-HTTPS native tests pass (fixed authenticated paths,
  malformed/unauthenticated responses and in-flight disconnect). The 3 affected
  native connection tests also pass after GET refactoring. Focused Firefox/React
  simulated-IPC test passes empty/populated lists, partial failure, token clearing,
  local-control isolation and stale reads/connect responses. Frontend production
  build, native library Clippy and changed-file formatting/diff checks pass.
  No unrelated server/controller/full-workspace suite was repeated.
- These are native HTTP tests plus browser UI tests, not a real mobile/native-IPC
  launch or remote deployment. Creating/controlling VMs and SSH in remote mode
  remain unavailable. No real VM, credentials or infrastructure were changed.
- Next single slice: unblock strict first boot with explicit catalog default-user
  attestation, then wire the minimal durable create/start/readiness workflow into
  the server API/worker and remote UI. Defer additional management features.


## First mobile journey: attested image-default user — 2026-10-18

- Scope/acceptance: unblock compatible Cloud-Init user generation with explicit
  template-bound opt-in; preserve legacy mappings and test both transport modes.
- Optional imageDefaultUser declares matching guestUser/templateVersion, locked
  password and no inherited keys; root, mismatches, false/missing/unknown fields
  are rejected. Absent fields are omitted from serialization, preserving legacy
  mapping hash inputs. Present declarations are bound into the plan hash/snapshot.
- Opted-in transport deletes ciuser; exact observation requires absence. Seed
  inspection accepts omitted user only with attestation, rejects explicit null,
  named/scalar entries in that mode and retains exact keys/network/revision checks.
  Operator declarations and generated previews do not prove installed defaults,
  attached ISO contents or first boot. ADR 0037 records the choice.
- Verification: 8 existing catalog/API tests passed after the model extension;
  new targeted catalog/attestation case and generated-HTTPS transport/seed case
  pass. The 2 affected seed-preflight tests and 8 Cloud-Init transport/executor
  tests pass (the new HTTPS case is included in the latter filter). Scoped server
  library Clippy and formatting/diff checks pass. No unrelated
  client/full-workspace checks or real infrastructure operations were performed.
- Next single slice: durable seed regeneration/start stage and strict readiness,
  then production create API/worker and visible remote create controls. Existing
  clone/resources/disk/Cloud-Init executors are still standalone; do not claim
  the create/SSH journey works yet.


## First mobile journey: fixed seed regeneration transport — 2026-10-18

- Scope: implement the provider regeneration call and pre/post seed checks; durable
  journal and start gating remain the next slice. Upstream API source confirms a
  synchronous null response, no UPID and no digest parameter (ADR 0038).
- New fixed dispatch_seed_regeneration repeats compatible preview/revision/hash
  checks before one empty authenticated PUT to the exact target cloudinit route.
  Null success is followed by independent settings/seed checks; changed payloads
  fail. Only safe hashes/revision/format are returned, always guestReady=false.
- Verification: generated-HTTPS test passes seven variants: success, incompatible
  report, stale revision/hash, lost reply, unexpected task-like reply and changed
  postflight contents. Exact route/auth/empty body and zero-or-one writes asserted.
  Scoped server library Clippy and changed-file formatting/diff checks pass. No
  unrelated suite repeated, new dependency/schema or real infrastructure access.
- This is transport preparation, not durable regeneration or a usable create
  workflow. Callers must persist a dedicated dispatch guard and successful result;
  previews alone cannot recover unknown regeneration. Endpoint lacks atomic digest
  protection, so bounded pre/post observations cannot exclude concurrent edits.
- Next single slice: durable seed-regeneration journal and executor, retaining
  success across restart and forbidding replay after uncertain submission. Require
  this evidence for start, then implement strict readiness and wire create API/UI.


## First mobile journey: durable seed regeneration — 2026-10-18

- Scope/acceptance: persist regeneration intent/result, preserve success across
  restart and never replay an uncertain submission. Implemented as start_vm's
  first substep without changing saved plan steps/hashes. Migration 10 adds a
  bounded private report journal because stage results are terminal-only.
- Validated prior Cloud-Init settings now expose typed evidence tied to instance,
  plan/scope, exact resources/disk/network/user/keys, stopped state and revision.
  Seed claim requires this chain. Current capability/catalog/provider scope are
  checked before preflight. Expected compatible hashes/revision/format and stage
  dispatch marker commit before one fixed regeneration PUT; completed proof is
  retained separately. No raw keys/seed, fabricated UPID or new dependency.
- A successful regeneration leaves start_vm/workflow running, with no power task.
  regenerated_seed() is the necessary gate for the future start executor; it is
  not guest readiness. Before-marker interruption can resume. After-marker
  interruption/lost reply remains uncertain and is never claimed for another PUT.
  Successful journal proof survives recovery and does not trigger another request.
- Verification: 2 focused storage tests pass for approval/proof rejection, both
  disk shapes and restart before/after markers/success. Generated-HTTPS executor
  test passes 6 cases (success, lost reply, disabled, missing catalog/provider,
  scalar incompatibility), with exact write counts and no restart requests.
  Five provisioning schema checks, including 9->10, plus affected lifecycle/
  instance upgrade checks pass. Scoped server library Clippy and formatting/diff
  checks pass. No real VM/infrastructure/credentials changed or mobile launch.
- Next single slice: durable power-start substep that requires regenerated seed
  evidence, reconciles retained start tasks without resubmission, then strict
  readiness and production create API/worker/remote controls. ADR 0039 explains
  journal semantics; do not treat the start_vm marker as a power submission.


## First mobile journey: durable provisioning power start — 2026-10-18

- Scope/acceptance: start only after retained compatible regenerated seed; persist
  separate power intent/task before polling; never replay unknown submissions and
  reconcile known tasks through GET after restart. Implemented in the existing
  schema-10 journal with an optional backward-compatible power substep, no new
  migration/dependency or change to saved plan steps/hashes.
- A transactional claim distinguishes seed-only completion, claimed power work and
  submitted power. Current capability/catalog/scope and exact stopped-target seed
  revision/hashes/format are checked before a separate durable marker and fixed
  start POST. Task binding uses target node/VMID and qmstart. Corrupt seed/power/
  task evidence fails closed; claiming rolls back without changing the journal.
- Known-task recovery works with writes disabled/catalog absent. Unknown tasks
  never reclaim. Successful task plus independent running/QMP observation succeeds
  start_vm only, with guestReady=false; readiness remains queued and operation
  running. The standalone executor is not wired into the production worker/API.
- Verification: 2 filtered storage tests pass for approval/proof/task binding,
  both disk shapes and restart before/after power markers. Separate corruption
  test passes. Generated HTTPS runs actual seed then start executors across 7
  cases: success, lost start response, task recovery, changed seed, disabled writes,
  failed task and paused VM. Recovery asserts exactly two GETs and no new POST.
  Three affected seed-journal regression cases and server library Clippy pass;
  changed-file formatting/diff checks pass. No unrelated client/full suite, real
  VM/infrastructure mutation, credential access/change or mobile runtime claim.
- Next single slice: strict bounded guest-readiness execution and durable result,
  validating the retained start proof before any ready claim. Then wire the
  minimum create API/worker and visible remote creation controls. ADR 0040 records
  the start journal; retain honest separation of task, power and guest evidence.


## First mobile journey: fixed guest-readiness transport — 2026-10-18

- Scope/acceptance: bounded strict Cloud-Init status plus MAC-bound IPv4 observation;
  source preparation is now compiled/tested, not durable readiness or real guest
  validation. ADR 0041 records API/source findings and limitations.
- New plan-bound request retains clone/instance marker, expected revision and MAC.
  Fixed read-only cloud-init status --format json command uses QGA command-array
  POST without shell/stdin/wait/arbitrary arguments. Positive PID is observed with
  fixed GET exec-status and network-get-interfaces. Each method is limited to 20s.
- Guard exact marker/revision, running/QMP, no template/lock/pending changes before
  observations and after a ready candidate. Require exit 0, status/extended_status
  done, no global/per-stage fatal or recoverable errors and completed timestamps
  in all four Cloud-Init stages. Running/not-started/unfinished is pending; no
  stderr, signal, truncation, missing stage or exit-2 workaround is accepted.
- Only one usable IPv4 on the expected MAC can count. Wrong NIC, loopback/link-local/
  subnet/broadcast/unspecified address does not establish readiness; ambiguous IPs
  or matching interfaces fail closed. Results/errors omit raw guest stdout/stderr.
  IPv4 evidence does not prove SSH reachability, authentication or host identity.
- Verification: 3 scoped generated-HTTPS tests pass, including boolean/integer
  provider flags, fixed command/PID/routes, pending/error/truncation/address/
  revision/QMP cases, foreign markers, invalid PID and lost reply without retries.
  Scoped server library Clippy and changed-file formatting/diff checks pass. Pinned
  local Cloud-Init source and official provider source were inspected for actual
  formats. No unrelated checks, runtime dependency/schema or real VM changes.
- Next single slice: durable readiness PID/deadline/result and executor, validating
  retained successful power-start proof before probing or reporting ready. Then
  connect the minimum create API/worker and visible remote controls. Production
  readiness, overall deadline/restart behavior, actual guest and mobile remain open.


## First mobile journey: durable guest readiness — 2026-10-18

- Scope/acceptance: persist probe PID and fixed overall deadline, resume bounded
  observations after restart, reject stale/invalid evidence and never skip later
  declared stages. Implemented in the schema-10 journal's optional readiness
  field, preserving old reports/plan hashes without a migration/dependency.
- Claim validates the entire retained power-start/seed/settings chain. Read-only
  agent availability is checked before marking a new fixed command; PID persists
  before observation. Known PIDs recover with GET only. Lost submissions never
  re-execute. Completed pending probes clear their PID before another command,
  respecting QEMU's consuming exec-status semantics. No arbitrary command exists.
- Retain one 300-second window, five-second scheduling and maximum 60 commands;
  restarts do not renew it. Backward clock/expired deadline prevents ready claims,
  including late completed responses. Safe failure codes distinguish rejected,
  unconfirmed and timed-out readiness, retaining the VM for inspection.
- Clean scoped Cloud-Init/IP evidence completes readiness; overall success occurs
  only when it is the last declared stage. Packages/snapshots remain queued when
  declared. No SSH authentication/reachability or real first-boot claim follows.
- Verification: 4 focused storage tests pass (proof/PID guards, retained deadline/
  restart, completed-pending refresh, expiry and minimal-workflow completion).
  Generated HTTPS executor tests prove known-PID GET-only recovery, unknown-command
  non-replay, no provider calls after timeout and real fixture exit-2 rejection.
  Three affected guest transport tests and scoped server library Clippy pass;
  changed-file fmt/diff checks pass. Official QEMU semantics checked. No unrelated
  client/full suite, actual guest command, VM/infrastructure/credential changes.
- Next single slice: expose the minimum fully confirmed create enqueue and connect
  these standalone stages to the sequential production worker. Reject unsupported
  package/snapshot requests for that initial route explicitly, then add visible
  remote create controls. Do not start another isolated preparatory helper first.
  Mobile build/SSH backend and approved real disposable end-to-end test remain open.


## First mobile journey: confirmed create API and production worker — 2026-10-18

- Scope/acceptance: expose fully confirmed creation under explicit opt-in, connect
  durable stages to the actual worker and reject unsupported features explicitly.
  Implemented POST /api/v1/provisioning/create with one request key and complete
  Approval. Enqueue commits immutable definition/catalog/plan/association/stages
  before 202/Location and performs no provider call. Same-key retries retain one
  operation; target/scope/hash/feature/config errors reserve nothing.
- VMANYWARE_PROVISIONING_ENABLED defaults false, accepts exact true/false and fails
  startup safely before database creation when invalid. Main passes the same flag/
  catalog to router and sequential production worker. Lifecycle opt-in is separate;
  listener remains loopback and whole-API bearer middleware still covers creation.
- Minimum route requires empty packages, snapshotAfterProvisioning=false and an
  attested image-default mapping. Unsupported features return 422. New mutations
  recheck current capability; known tasks reconcile without replay. Every round
  runs one read, one lifecycle claim and at most one provisioning stage, prioritizing
  existing later-stage work. Unknown submissions never repeat or auto-clean up.
- Preview now exposes configured providerScopeId and runtime creation capability;
  no reservation/live verification claim. Absent scope preserves saved-plan JSON.
  Existing preview type accepts the boolean capability; remote creation UI is not
  yet present. Cloud-Init form serialization now leaves scope before await so the
  full production worker future is Send; explicit-user form behavior is retained.
- Verification: 5 targeted API/config/disabled-worker cases pass, including many
  rejection variants and an actual env-cleared binary invalid-startup fixture.
  Full generated-HTTPS Axum enqueue -> production worker -> clean guest evidence
  passes with 8 GiB/no resize and 40 GiB/resize, restart after clone, inventory
  progress and no repeated mutation on enqueue replay. Nine affected catalog/API
  checks and the explicit-user form check pass. Server lib/bin/new-test Clippy,
  changed-file formatting/diff and client TypeScript checks pass. No unrelated
  client/native/full-workspace suite repeated.
- No real VM/provider/guest commands, credentials or infrastructure changed. This
  proves local production-code integration, not actual Proxmox/phone/SSH behavior.
  ADR 0043 and the updated create API contract capture current limits.
- Next single slice: fixed remote native definition/plan/create/operation commands
  and a visible full-target confirmation flow, tied to current connection ID and
  retained idempotency key. Then phone build/SSH backend and the explicitly
  approved disposable end-to-end gate. Do not return to isolated preparatory helpers.


## First mobile journey: remote create form and native commands — 2026-10-18

- Scope/acceptance: fixed remote prepare/save/create/status commands and visible
  full-target confirmation, current connection generation and retained request key.
  Implemented form fields for definition/image/VMID/resources/public keys, with no
  packages or snapshots; native preparation saves or verifies matching definition
  hash before presenting a supported/capable server plan.
- Native session retains the plan/key and constructs Approval itself. Confirmation
  requires the matching key, generation and explicit true; concurrent submission is
  serialized. Lost responses keep the same immutable key/body for manual retry.
  New intent preparation is blocked after submission on that connection. Reads use
  the retained operation ID and reject another ID/kind or mismatched ready proof.
- Verified HTTPS/auth marker, bounded JSON and fixed paths remain mandatory; errors
  omit raw bodies/tokens. Disconnect clears session context and late replies, not
  the server operation. The UI displays complete server/target/template/storage/
  bridge/resources/user/key-count review and separate confirmation; manual status
  refresh distinguishes initialized guest from actual SSH access/trusted identity.
- Verification: 6 targeted native cases pass for plan/save/conflicts, confirmation/
  exact approval POST, safe failures/key retention, scoped ready validation, exact
  operation GET and disconnect during submission. One affected connection-generation
  case passes. Focused real Firefox/React simulated-IPC check passes confirmation,
  lost-response same-key retry, queued/ready display and stale read suppression,
  alongside existing remote list/connection guards. Frontend production build,
  scoped native library Clippy and changed-file fmt/diff checks pass.
- No server/full-workspace/controller suites repeated. No actual new-command Tauri
  IPC launch, deployed service, VM, credential/infrastructure changes, SSH or phone
  runtime is claimed. ADR 0044 records this boundary.
- Remaining limitation: one submitted intent per connection; context is memory only
  and GUI recovery after full restart/reconnect is not yet implemented. Users can
  retain key/operation ID for server-side inspection. Next single slice: explicit
  current-server operation recovery after reconnect/restart, then mobile build/SSH
  backend and an approved real disposable end-to-end check. Full roadmap remains
  active, including snapshots/recreate/delete and platform/release gates.


## First mobile journey: read-only remote creation recovery — 2026-10-18

- Scope/acceptance: reconnect to an explicitly authenticated server and find/read
  a retained creation and its target without submitting creation again. Added
  fixed native history/operation/environment GET commands and visible recent-list,
  saved-ID and manual-refresh controls. No credentials or intent persist locally.
- History examines the server's latest 100 operations and filters creations;
  older operations remain accessible by saved ID. Recovery works without the
  original prepared plan/session. IDs, kind, association, provider scope and clean
  successful guest evidence are validated before returning sanitized fields.
- Disconnect invalidates pending results. Recovery never resubmits/retries/cancels
  an operation or restores a native approval/key. Target associations and ready
  results are historical evidence, not current power, ownership or SSH proof.
- Verification: 3 focused native recovery tests pass, including malformed/wrong
  responses and disconnect during a read. Two affected prepare/ready regression
  tests pass. Real Firefox/React simulated IPC passes remount recovery, empty/list
  states, retained-target display, unchanged creation-submit count and stale-read
  suppression. Frontend production build, native library Clippy and changed-file
  formatting/diff checks pass. No server/unrelated/full-workspace suites repeated.
- No actual new-command Tauri IPC launch, phone build, real guest/VM/infrastructure
  or credential changes. Original prepared create/key remains session-only; an
  uncertain submission after restart can be inspected, not automatically retried.
  ADR 0045 records the recovery contract and supersedes ADR 0044's GUI limitation.
- Next single slice: choose and implement the smallest mobile SSH authentication
  path with strict host trust, then prove its native build/runtime. Existing agent
  authentication remains macOS-only; platform preference is still unanswered.
  Do not defer this behind snapshots or more recovery helpers. Real disposable
  end-to-end execution still needs explicit target/action approval.


## First mobile journey: encrypted application SSH identity — 2026-10-18

- Scope/acceptance: explicitly create a reusable Ed25519 key on the device, keep
  only encrypted private material on disk, reopen with the correct passphrase,
  refuse replacement and insert only its public key into remote-create YAML.
  Implemented native fixed-path create/unlock commands and visible form controls.
- Uses resolved ssh-key crypto, AES-256-GCM/OpenSSH and fixed bounded bcrypt KDF;
  private owned directory/file, no-follow reads, bounded input and atomic no-clobber
  persistence. Public-only return values; decrypted key lifetime is operation-local.
  No private-key import/discovery/export or passphrase/browser storage. Existing
  user-entered public keys are retained; insertion is explicit and deduplicated.
- Verification: 2 focused native tests pass for encrypted persistence/reopen,
  same fingerprint, no replacement, wrong phrase, busy/permission/symlink/size/
  corruption/plaintext rejection and valid-envelope GCM tampering. Real Firefox/
  React simulated IPC passes create, confirmation, cleared fields, failed/successful
  unlock and public-only definition submission, plus affected remote create flow.
  Frontend production build and native library Clippy pass; changed-file fmt/diff
  checks pass. No server/unrelated suites run.
- Source enabled for Unix, compiled/tested only on Linux. Not OS-keystore-backed;
  no biometric unlock, migration/backup or mobile storage/lifecycle proof. Only
  x86_64 Linux Rust target installed; adb/sdkmanager/xcodebuild unavailable in PATH.
  No actual new-command native IPC, phone launch or SSH authentication is claimed.
  All generated keys in checks were disposable; no user credentials/VMs changed.
- Next single slice: connect this persisted identity to explicit authenticated SSH
  and the existing bounded PTY/terminal with fresh strict host trust. Validate with
  generated localhost SSH fixtures, then mobile native build/runtime. ADR 0046
  records the storage decision and remaining platform gates. Full roadmap active.


## First mobile journey: application-key SSH authentication and terminal — 2026-10-18

- Scope/acceptance: use the retained encrypted app key through fresh strict host
  trust and the existing selected-identity/PTY flow, proving real local output,
  resize, Ctrl-C, close and reconnect. Implemented on Unix with explicit UI source,
  account/passphrase/consent, public fingerprint selection and check/open controls.
- Native selection verifies persisted host trust before decrypting; authentication
  verifies again before signing. Selection is window/account/endpoint-bound,
  single-use and expiring. Cancellation/replacement/destruction invalidate it;
  idle app tickets have a cleanup timer. No fallback key/agent/password is used.
  Store clones share the same nonblocking KDF mutex; private material stays native.
- Verification: 3 focused native SSH tests pass, including unknown/changed host,
  wrong phrase/fingerprint/window, cancellation/replay/expiry, rejected username,
  channel-free authentication, actual /bin/sh output, stty resize, foreground Ctrl-C
  with status 130 in under 2 seconds, close and reconnect with zero owned fixture
  connections left. Five affected host and five trust tests plus two PTY-policy
  tests pass. These tests use generated keys and owned localhost SSH fixtures only.
- Real Firefox/React simulated IPC passes application-key consent/unlock/failure,
  cleared secret fields, changed-host refusal, explicit identity, check/terminal,
  Ctrl-C/disconnect, no agent access/storage and stale unlock/start cleanup.
  Frontend production build, native library/test Clippy and changed-file fmt/diff
  pass. No server/full-workspace/unrelated suites repeated.
- Source now compiles the shared host/authentication/PTY path on Unix; Linux has
  protocol/real-shell evidence, not a new-command native GUI/IPC launch. The macOS
  agent fixture was adapted for explicit source choice but not run on this Linux
  host. No phone, real guest, OS-keystore/biometric or mobile lifecycle proof.
  No user credentials, VM or production infrastructure changed. ADR 0047 applies.
- Next single slice: Android native build/launch feasibility for the existing
  create-to-SSH screens using an isolated toolchain on this Linux host. Platform
  preference remains unanswered; Android first is a provisional host-driven choice,
  not cancellation of iOS. Then actual mobile input/lifecycle and an explicitly
  approved disposable real create-to-SSH gate. Full roadmap remains active.


## First mobile journey: Android packages and native emulator smoke — 2026-10-18

- Scope/acceptance: build the complete app for Android and verify actual packaged
  launch/bridge where this host permits. Generated editable Android project source
  is now retained, while generated glue/JNI, local paths, signing and outputs stay
  ignored. No product runtime crate or frontend/server behavior changed.
- Both aarch64/ARM64 and x86_64 Rust libraries compile and separate debug APKs build.
  JDK 21.0.2, SDK/build-tools 36, NDK 28.2.13676358, CLI 2.12.3, Gradle 9.6.1,
  AGP 9.3.1 and Kotlin 2.2.10 were used in an owned temporary prefix. Gradle
  distribution checksum is pinned. Configured minimum API 24 is not tested parity.
- Native Activity reserves system-bar/cutout space, sets a matching dark backdrop
  and forwards handled inset types as zero while keeping IME notifications. Initial
  padding/forwarding gave a mounted but unpainted native surface; revised settings
  pass cold visual capture. No keyboard/IME or physical-device claim follows.
- Explicit no-backup plus both cloud/device extraction exclusions are packaged.
  APK public signatures verify; native ABI and embedded frontend/terminal asset
  names are inspected. Actual Android backup/transfer and release signing untested.
- Verification: real Android 15/API 35 x86_64 (Pixel 6 AVD, WebView 124.0.6367.219)
  packaged app passes actual UI native-platform button, native SSH support/guard
  dispatch, encrypted identity creation, wrong phrase/no replacement, identical
  identity after force-stop/relaunch and 412px layout without horizontal overflow.
  The opt-in check inspects the actual framebuffer, not DOM readiness alone, and
  records a reviewed launch screenshot. No fake IPC or app fixture mode is used.
- Evidence: experiments/android-native/check.mjs and README; ignored
  .lab/android-native/report.json, launch.png and artifacts.json hold runtime and
  artifact hashes. Logs are /tmp/vmanyware-android-build-final.log and
  /tmp/vmanyware-android-native-smoke.log. ARM64 APK is at
  apps/client/src-tauri/gen/android/app/build/outputs/apk/arm64/debug/app-arm64-debug.apk;
  x86_64 is the analogous x86_64 path. Both are development/debug artifacts, not
  release packages. Current verified frontend assets were reused; no unrelated
  server/Linux/full-workspace suites repeated. ADR 0048 records decisions/limits.
- Reuse hint: toolchain is /tmp/vmanyware-android-reovczwi with env.sh; confirm it
  still exists before use. AVD is vmanyware-native-smoke in ANDROID_USER_HOME/avd,
  dedicated ADB port 5047, emulator serial emulator-5580, gRPC port 8558 with JWT.
  The emulator/app/ADB and owned Gradle daemon are stopped after verification.
  Download archives and the duplicate standalone NDK are removed; SDK/AVD caches
  remain for the next slice. No personal credentials or real infrastructure touched.
- Next single slice: actual Android SSH authentication/PTY with this app-owned key
  and strict host trust against an owned local SSH fixture. Then authenticated
  remote create/readiness-to-SSH integration and keyboard/lifecycle, followed by an
  explicitly approved disposable real VM gate. ARM64 physical runtime, release,
  iOS and the rest of the roadmap remain open; no requirement is silently dropped.


## Commit checkpoint: Android SSH and keyboard work — 2026-10-19

- Scope: preserve the accumulated roadmap work and push the current source at the
  user's request. This checkpoint does not close additional roadmap gates.
- Added an opt-in owned localhost SSH fixture and actual Android native SSH
  harness. One earlier emulator run completed authentication, strict host trust,
  shell, interrupt, reconnect and cleanup. Subsequent stronger input checks exposed
  duplicated/lost characters in Android xterm input; reliable mobile typing is not
  established by that earlier pass.
- Prepared a standard Android WebView terminal input adapter with special-key
  controls and keyboard-aware Activity insets. Its current source must still pass
  an actual rebuilt APK keyboard/SSH journey. Earlier APK evidence in the preceding
  section applies to the earlier source, not this adapter. No current APK rebuild
  result or temporary toolchain/log availability is assumed at this checkpoint.
- The host SSH fixture previously built and passed scoped Clippy; three affected
  application-identity tests passed. This checkpoint runs frontend production build,
  Android harness JavaScript syntax checks and diff whitespace review only; no
  unrelated server/workspace suites are repeated.
- No real VM, production infrastructure or personal credential mutation occurred.
- Next single task: rebuild the Android APK and validate the new keyboard adapter
  through the owned emulator SSH fixture, including Enter, resize and Ctrl-C.


## Android input transport-boundary regression — 2026-10-19

- Scope/acceptance: check the prepared Android input adapter's exact outbound
  bytes, composition and special-key segment boundaries before native validation.
  The previously recorded Android SDK/JDK/emulator prefix is absent on this host;
  no current emulator/build process or native runtime evidence is assumed.
- Added opt-in `--app-ssh --android-input` mode to the existing owned Firefox
  fixture. It mounts the actual React mobile component via a fixture-only user
  agent, with simulated native IPC and no production behavior changes/dependencies.
- Verification: `node experiments/instances-browser/check.mjs --app-ssh
  --android-input` passes exact typing/replacement bytes, composition held until
  commit without a duplicate final input, Unicode codepoint deletion, Enter,
  Tab/Esc/Backspace segment reset, Ctrl-C/disconnect and stale selection/session
  cleanup. The first run exposed an incorrect expected byte-count in the new
  test; corrected that assertion and the expanded final run passes.
- No application code changed in this slice; no unrelated suites or prior build
  checks repeated. Native WebView/IME/SSH, viewport behavior and the complete
  mobile journey remain unverified. No infrastructure or credential mutation.
- Next single task: restore an isolated Android toolchain, rebuild the x86_64
  debug APK and execute the owned native SSH/keyboard harness. The roadmap remains
  active; this browser check does not close Phase 0F or Phase 5 mobile acceptance.


## Android toolchain restored; APK rebuild in progress — 2026-10-19

- Previous goal turn made progress: browser input assertions and documented
  evidence were added; native acceptance remains open.
- Restoring the owned toolchain under /tmp/vmanyware-android-resume. JDK 21.0.2
  and Linux command-line tools 16111833 were downloaded from their official
  HTTPS repositories and verified against the published SHA256 / repository SHA1.
  No system Java/SDK configuration or personal credential file was changed.
- SDK installation completed successfully in exec session 35502, logging to
  sdk-install.log in that prefix. Packages are
  platform-tools, API 36, build-tools 36.0.0, NDK 28.2.13676358, emulator and
  API 35 google_apis x86_64. Package activation completed; no installer restart is needed.
- env.sh in that prefix sets the owned Java/SDK/AVD/Gradle paths. The newest
  command-line package delegates SDK installation to Android CLI; installed
  packages are staged under sdk/.sdk before activation. Tool output reports CLI
  1.0.16625370. KVM is present and accessible.
- Rebuilt the current frontend successfully for APK embedding; this is preparation,
  not Android runtime evidence. No application/backend suite repeated.
- Added the native SSH harness invocation/guards to its README and ADR 0049 for
  the prepared streaming input decision and its remaining editing/runtime limits.
- Created only vmanyware-native-smoke with Pixel 6/API 35 and a 2 GiB data
  partition. Owned emulator is live in exec session 92711, serial emulator-5580,
  ADB port 5047, JWT-protected gRPC 8558; sys.boot_completed returned 1.
- Current x86_64 rebuild is live in exec session 12748, logging to apk-build.log
  in that prefix. Rust Android compilation passed; Gradle distribution download
  finished and APK assembly is pending. Revalidate the exact handle/log before
  waiting; do not restart because output pauses.
- Next: finish this APK build, install it only on the guarded owned emulator,
  then run the native SSH/keyboard harness with the existing
  target/debug/examples/android_ssh_fixture executable. No real provider/VM or
  personal credential mutation. Native acceptance remains open.


## Current Android APK: native SSH/input/PTY — 2026-10-19

- Scope/acceptance: rebuild the prepared mobile input adapter and execute actual
  native SSH, shell input, resize, interrupt, reconnect and cleanup against the
  owned generated localhost fixture. This slice advances the mobile journey,
  not the complete roadmap or the real-VM gate.
- Restored isolated JDK/SDK/NDK/AVD/Gradle tooling at
  /tmp/vmanyware-android-resume; env.sh retains its paths. SDK install session
  35502 and APK build sessions 12748/26699 completed successfully. Frontend and
  x86_64 Rust compile plus Gradle debug APK assembly pass on the current source.
- An independent owned TCP listener probe returned `Network is unreachable`
  from the emulator toward 10.0.2.2. Two host-alias native attempts consequently
  failed before authentication. Added explicit opt-in
  VMANYWARE_ANDROID_SSH_ADB_TUNNEL=1: no-rebind mapping only for the owned
  ephemeral SSH port, report labels the tunnel, and finally removes that mapping.
  Direct emulator networking remains unresolved; no trust verification bypass.
- Initial tunneled native run passed. Actual screenshot review exposed vertically
  stacked special controls from a generic narrow-screen rule. One scoped CSS
  override now wraps those controls with 44px minimum targets. Rebuilt/reinstalled
  the updated APK. Strengthened the native layout assertion accordingly.
- Repeated resize checks exposed a fixture race: ADB typing reveals the keyboard
  after the earlier size sample. The harness now waits for size stability after
  typing and before submitting `stty size`. Final run (exec 3496) passes native
  trust/changed-key refusal, wrong phrase, auth-only/no channel, real shell input,
  rotation and actual stty dimensions, Ctrl-C/status130, reconnect, process-death
  shell/socket cleanup and persisted host trust. Three auth attempts/two PTYs;
  final connections zero, all shells closed. The terminal screenshot was reviewed.
- Evidence: ignored .lab/android-ssh/{report.json,ssh-report.json,artifacts.json,
  launch.png,ssh-terminal.png}. artifacts.json records current APK SHA256/size.
  Logs are under the owned prefix (frontend-build.log, apk-build.log,
  native-ssh.log). Native harness syntax and diff whitespace checks pass. No
  unrelated server/Rust/browser suite rerun. ADR 0049 records input decisions.
- Owned emulator session 92711 is stopped; dedicated ADB server and owned Gradle
  daemon stopped. Harness reverse mapping list was empty after cleanup. SDK/AVD
  caches remain for reuse; no personal credentials or real VM/provider touched.
- Limits: ADB injection with a visible software keyboard is not full IME,
  accessibility, physical ARM64, suspend/resume, iOS or release proof. The latest
  x86_64 APK is built/run; no new ARM64 build/run is claimed. Direct network and
  the authenticated create/readiness-to-SSH journey remain open.
- Next single slice: actual Android authenticated HTTPS control-plane connection
  and read-only recovery against an owned local service, then integrate confirmed
  creation/readiness to SSH. Resolve/report the emulator network boundary rather
  than treating the ADB tunnel as deployment evidence. Real VM writes still need
  explicit concrete-target approval; the overall roadmap stays active.


## Android authenticated HTTPS and read-only recovery — 2026-10-19

- Scope/acceptance: actual Android native secure connection, issuer/hostname/token
  refusal and read-only retained-operation recovery after restart against an owned
  local production service. Earlier keyboard/browser changes in the dirty tree
  retain their previously recorded checks; this slice does not repeat them.
- Found missing Android integration for reqwest 0.13's platform TLS verifier.
  Added Android-only JNI/verifier dependencies already present transitively in
  Cargo.lock, synchronous application-context initialization before Tauri startup,
  Cargo-matched Kotlin artifact resolution and ProGuard retention. Current APK
  launches successfully through this initialization; no TLS fallback/bypass.
- Added optional single public PEM CA input (16 KiB maximum), validated natively
  and scoped to the current profile. Android cannot merge extra platform roots:
  the first CA run exposed a client-builder failure, corrected by using only the
  supplied CA with normal WebPKI signature/validity/hostname verification. No
  device certificate/credential store changed. Public CA clears on disconnect;
  tokens still clear on submission and no connection persists across restart.
- New opt-in --remote Android harness launches the actual production server
  executable, a generated HTTPS proxy and a disposable SQLite database with one
  seeded failed creation/target. Child environment has no provider configuration;
  the proxy rejects non-GETs. This is recovery evidence, not VM creation evidence.
  TLS private keys/tokens stay in the owned fixture and are never printed; the
  generated directory is removed after stopping the owned service.
- Verification: current frontend production build and x86_64 Rust/Gradle APK
  build pass. Final actual Android run (exec 60023) passes invalid/private/
  bundled/oversize CA refusal, unknown issuer and hostname mismatch before HTTP,
  actual server token rejection, authenticated connection, environment/history
  reads, actionable missing-catalog error, force-stop/relaunch without request
  replay or restored credentials/profile, explicit reconnect/read-only recovery,
  and disconnect/CA clearing. All 11 recorded requests are GETs. Native recovery
  screenshot reviewed. Early fixture attempts also exposed a missing remote-mode
  selection and a screenshot-before-scroll race, both corrected.
- Targeted Firefox --remote-connection scenario passes (exec 77791), including
  forwarding/locking the optional CA and existing affected create/recovery callers.
  New harness JavaScript syntax, changed Rust formatting and diff whitespace pass.
  Attempted new Linux parser unit test could not compile: WebKitGTK 4.1/pkg-config
  prerequisite is absent on this host. It is not reported as a passing unit run;
  corresponding validation cases passed through real Android native IPC instead.
  No full server/workspace or unrelated keyboard suite rerun.
- Evidence: .lab/android-remote/{report.json,remote-report.json,artifacts.json,
  remote-recovery.png,launch.png}; artifacts.json hashes the current APK. Logs
  under /tmp/vmanyware-android-resume: remote-frontend-build.log,
  remote-apk-build.log, native-remote.log, remote-browser.log and ca-test.log.
  ADR 0050 records the security/runtime choice. Android SSH evidence above is for
  its earlier APK; no new SSH run is claimed for this TLS-only source change.
- Owned service, TLS sockets, generated keys/DB and both ADB reverse mappings are
  cleaned up; the mapping list is empty. Emulator session 79401 and owned Gradle
  daemon are stopped; the dedicated ADB server is stopped. Retained
  toolchain/env.sh allow reuse.
- Limits: explicit ADB tunnel remains necessary; direct emulator/deployed network,
  actual OS/public-root positive verification, complete create/readiness-to-SSH,
  physical devices, iOS, full IME/lifecycle and release remain unproven. No real
  Proxmox/VM or personal credential operation occurred. Full roadmap stays active.
- Next single slice: integrate confirmed creation/readiness with this native
  Android HTTPS connection against the owned generated provider fixture, then
  connect the resulting ready target to the app's SSH flow. Real VM mutations
  remain gated by explicit approval of a concrete disposable target/operation.


## Ready creation/recovery to explicit SSH target — 2026-10-19

- Previous goal turn made progress: native HTTPS verification, optional private CA
  and actual read-only recovery evidence were added. Full roadmap remains open.
- Scope/acceptance: connect the existing ready-result UI to the existing SSH
  inspection/authentication flow in remote mode, preserving explicit trust and
  clearing old state on reselection/disconnect. Inspection of current source showed
  that remote mode hid the local terminal and had no handoff action.
- Fresh succeeded/guestReady creation now offers Prepare SSH connection, carrying
  exact recorded IPv4 and native plan guestUser. Recovery offers the same explicit
  action with address only; account remains blank because its contract lacks a
  verified username. Queued/failed results offer no handoff. No automatic network
  request, trust, unlock or login occurs on readiness or target selection.
- Remote connection owns the selected target in memory. Selecting/remounting its
  host-trust flow clears old challenge/authentication/terminal state; disconnect,
  connection replacement and leaving remote mode remove it. Heading focus/scroll
  moves to the selected target. Existing local/default-account callers are intact.
- Verification: production frontend build passes (exec 37739); expanded targeted
  Firefox --remote-connection fixture passes (final exec 52378): queued/failed
  guards, no automatic inspection, exact address/account, independent unknown-host
  consent followed by fresh inspection, reselection/disconnect reset, recovery
  username left empty, and affected stale create/recovery behavior. Separate
  --app-ssh fixture passes for default-account callers, explicit identity,
  changed-host refusal and stale terminal cleanup. The broader browser scope is
  warranted by changing shared host/authentication component props. No server,
  Rust, unrelated Node/controller or Android suite repeated for this TS-only diff.
- Logs: /tmp/vmanyware-android-resume/handoff-build.log, handoff-browser.log,
  handoff-ssh-regression.log. Diff whitespace review passes. ADR 0051 and the
  provisioning/browser documentation record behavior and proof boundaries.
- Earlier dirty TLS/Android input files retain their separately recorded evidence.
  This slice does not rebuild/run an APK or establish native handoff, real VM
  creation, full software IME or direct remote-network support. Toolchain remains
  retained; no new emulator/service process, credentials or VM mutation started.
- Next single slice: exercise confirmed creation and readiness on actual Android
  against the production service plus an owned generated provider fixture, then
  use this explicit handoff to enter the native SSH flow. Treat browser IPC results
  as component proof only; retain the approved disposable real-VM gate and the
  rest of the original roadmap.


## Android native confirmed creation to SSH fixture — 2026-10-19

- Previous goal turn made progress: explicit ready-target/account handoff was
  implemented and browser-verified. This slice supplies native integration evidence.
- Scope/acceptance: actual packaged Android confirmed creation through the
  production service/worker against an owned generated HTTPS provider, followed
  by explicit handoff and same-key native SSH/PTY; no real infrastructure changes.
- Added opt-in --remote --create mode with a loopback, in-memory Proxmox protocol
  peer. Service starts from an empty disposable DB and clean fixture-only
  environment, with generated token/CA/catalog and execution enabled only there.
  Proxy accepts GET plus exact plan/definition/create POST paths in creation mode;
  normal remote mode remains GET-only. No production API/worker code changed.
- Rebuilt x86_64 debug APK with the already verified current frontend handoff
  assets (build exec 11552 passes). Native UI unlocks its existing Android key,
  inserts the exact public key, previews without creation, requires explicit
  consent and submits one create request. Production worker reaches guestReady
  through clone/resources/Cloud-Init/seed/start and strict guest-proof validation.
- Final native run exec 26350 passes: three API POSTs (plan, definition, create),
  exactly one clone/seed/start/fixed cloud-init-status execution and two config
  writes; 2 CPUs/2048 MiB/no ballooning and the same Android public key verified
  at the provider boundary. Exact 192.0.2.42/debian handoff review appears only
  after readiness and an explicit action. No automatic host inspection/auth.
- SSH then explicitly overrides that synthetic guest address with the owned ADB
  tunnel endpoint. The same key passes native strict host trust, wrong phrase,
  auth-only/no channel, real OS-shell input, rotation/stty, Ctrl-C/status130,
  disconnect/reconnect, process-death cleanup and changed-key refusal. Three auth
  attempts/two PTYs, final connections zero/all shells closed. Provider/API
  mutation counts remain unchanged across SSH and application restart.
- Early runs exposed fixture-only selector/route issues: header class mistaken
  for the key details panel; missing current=1 config query; and generic terminal
  Disconnect hitting the enclosing server button. Corrected selectors/fixture
  route and scoped shared SSH buttons to their trust section. Existing targeted
  remote Firefox scenario passes after its matching key-panel selector correction
  (exec 7834). No unrelated browser/server/Rust suites rerun.
- Native harness JS syntax/diff whitespace review passes. Evidence is ignored
  .lab/android-create/{create-report.json,ssh-report.json,report.json,
  artifacts.json,ssh-terminal.png,launch.png}; APK hash/size are recorded.
  Screenshot reviewed: terminal output and wrapped controls visible above keyboard.
  Logs: /tmp/vmanyware-android-resume/create-apk-build.log, native-create.log,
  create-browser-selector.log. ADR 0052 records boundaries. Earlier dirty source
  slices retain their separate evidence; no general roadmap completion claimed.
- Owned provider/service/SSH fixture, generated TLS/DB directory and reverse
  mappings cleaned up; reverse list is empty and no owned server/SSH process
  remains. Emulator session 2706, dedicated ADB server and owned Gradle daemon are stopped;
  retained SDK/AVD/env.sh support reuse.
- Limits: synthetic provider/guest readiness, explicit SSH endpoint override and
  ADB injection are not real VM provisioning, direct networking or full software
  IME proof. No physical-device, iOS or release claim. No real provider, personal
  key, production infrastructure or user credential store was touched.
- Next single slice: resolve the owned emulator host-alias/direct-network boundary
  and repeat the native HTTPS/SSH transport checks without ADB reverse mappings.
  Then complete the remaining mobile usability/lifecycle and explicitly approved
  real disposable VM gate while retaining the full original roadmap scope.


## Direct emulator HTTPS/SSH transport — 2026-10-19

- Previous goal turn made progress: actual native creation/API/worker/provider
  integration and same-key SSH passed through an explicit tunnel. This slice
  removes that transport limitation without changing production application code.
- Scope/acceptance: diagnose the owned host-alias boundary and validate native
  verified HTTPS and strict SSH/PTY without data-plane ADB mappings. The route
  now shows a validated default Wi-Fi network; a correctly bounded owned TCP
  marker probe reaches 127.0.0.1 through 10.0.2.2. No route/firewall/emulator
  feature/device trust mutation was needed. The exact cause of older transient
  network-unreachable failures is unproven; do not claim a permanent defect/fix.
- Added explicit VMANYWARE_ANDROID_HTTPS_DIRECT=1 to the opt-in harness, using
  host alias 10.0.2.2 and a matching SAN while preserving the separate wrong-host
  certificate test. SSH already defaults to the host alias without its tunnel
  opt-in. Creation mode now supports that direct transport and labels both paths
  accurately. No automatic fallback or TLS/SSH-verification bypass.
- Direct-mode startup now checks an empty reverse list and performs a bounded
  owned TCP readiness probe independently of sys.boot_completed. Combined direct
  creation checks an empty reverse list again at completion. ADB/CDP only drives
  and observes UI; API/SSH bytes use normal emulator TCP.
- Verification: first direct native journey passes (exec 3336), then stop/restart
  the exact owned emulator for a cold-boot repeat with the new readiness gate.
  Final exec 23770 passes current-APK TLS issuer/hostname/token refusal,
  confirmed creation through actual service/worker and generated provider,
  resource/key checks, exact ready-target handoff, direct host-alias SSH trust/
  real-shell input/rotation/stty/interrupt/reconnect/process-death/changed-key
  checks and no creation replay. Probe: one attempt, 439 ms, zero reverse mappings.
  HTTPS report is direct; SSH target is host alias; final SSH connections zero.
- APK unchanged from the previously verified handoff build; no frontend/Rust/APK
  rebuild or unrelated suite rerun for harness-only changes. Changed JS syntax
  and diff whitespace checks pass. Evidence: .lab/android-direct/{report.json,
  create-report.json,ssh-report.json,artifacts.json,ssh-terminal.png,launch.png};
  hashes are recorded. Logs under /tmp/vmanyware-android-resume:
  native-direct.log, direct-emulator.log; diagnostic connectivity.txt/wifi.txt
  capture only the owned AVD. ADR 0053 records boundaries and upstream reference.
- Owned service/provider/SSH sockets and generated TLS/DB data cleaned up;
  reverse mapping list remains empty. Emulator exec 20471 and its dedicated ADB
  server are stopped; toolchain remains reusable. No real provider/VM, personal
  key, host firewall, device trust/credential store or production service changed.
- Limits: synthetic guest endpoint is still explicitly replaced by the host alias
  and ephemeral SSH port. Direct emulator networking is not deployed/LAN/VPN or
  physical-device evidence. Real VM, full software IME/background lifecycle,
  OS/public-root positive verification, iOS/other platforms and release remain
  open. This evidence supersedes only the emulator host-alias blocker.
- Next single slice: actual software-keyboard taps and composition plus native
  background/resume terminal behavior, then the remaining mobile/platform and
  explicit disposable real-VM gate. Preserve the full roadmap, including pending
  snapshot/recreate/delete and deployment/release requirements.

## Android brief background/resume — 2026-10-19

Scope: verify HOME/background/return on the current Android native SSH terminal
without implicit reconnect or input replay. The optional
`VMANYWARE_ANDROID_BACKGROUND=1` native SSH fixture now observes the real WebView
visibility transition, waits two seconds while hidden, and compares server-side
authentication attempts, channel count and received input bytes before/after.
Explicit return resumes the same shell and produces fresh `RESUME_OK` output.

Earlier diagnostic runs passed this scenario, but a separate initial software
keyboard check proved insufficient: Android/Gboard reports the input view shown
while screenshots and native window geometry show only the bottom navigation
strip, with no letter grid. Later terminal screenshots do show the full grid;
the trigger is not isolated. Removed the exploratory capture-only check rather
than count it as successful software-key input. No product behavior was changed.

Limitations: two-second foreground/background transition only; no screen lock,
Doze, long suspension, process eviction or background-service guarantee. Text
commands still use ADB injection; software-key taps/composition remain open.
Next task: isolate initial Gboard letter-grid visibility and verify actual taps
with exact server-side byte/output assertions. Keep real VM operations gated.

Verification: final cleaned-up `--ssh` run with the background flag passed on
Android 15/API 35 x86_64, WebView 124, via direct 10.0.2.2 transport. Evidence:
`.lab/android-background/ssh-report.json` and terminal screenshot. JavaScript
syntax checks and `git diff --check` passed. No APK rebuild or unrelated suites
were needed for this harness/documentation-only slice.

## Android actual software-key taps — 2026-10-19

Scope: prove taps on displayed Gboard keys reach the real SSH shell, including
editing and sending, without interpreting the IME shown flag as sufficient proof.
Added an opt-in `VMANYWARE_ANDROID_SOFTWARE_KEYS=1` stage to the owned native SSH
fixture. It is confined to the reviewed Pixel 6 1080×2400 English QWERTY layout
and checks the Gboard window/touch region before tapping. Every letter/space and
backspace is checked against the actual controlled input. `echo imeox`, Gboard
backspace, `k`, and Gboard send must yield exactly 13 additional received SSH
bytes, a cleared input and real shell output `imeok`, without new channels or
authentication. The existing screenshot captures the full keyboard before taps.

The first run passed with the brief background/resume scenario also enabled.
This proves actual ASCII software-key input on this owned emulator after the
rotation/control-key sequence; it does not fix/prove initial-focus keyboard
visibility, Unicode/composition, other IMEs, iOS or physical devices. No product
code or APK change was needed. Next task: isolate the initial Gboard key-grid
visibility issue before extending the real-key composition scenario.

Verification: final run with the stricter Gboard-window guard and both keyboard/
background flags passed on Android 15/API 35 x86_64 and WebView 124, using direct
10.0.2.2 transport. Evidence: `.lab/android-software-keys/ssh-report.json` and
`ssh-terminal.png`. JavaScript syntax checks and `git diff --check` passed;
no unrelated suites or application rebuild were run. Emulator/owned ADB server
were stopped after verification.

## Scoped snapshot listing — 2026-10-20

Previous goal turn made progress: actual Gboard taps yielded exact SSH bytes and
shell output. This turn revalidated the workspace and found the prior temporary
Android SDK/emulator prefix absent, with no live process to wait for. No mobile
product fix or fresh native evidence is claimed. Continued with an independent
roadmap slice instead of recreating a large toolchain for another diagnostic.

Implemented read-only `GET /api/v1/environments/{id}/snapshots`, fixed to the
registered instance's provider scope/node/VMID. Added the fixed Proxmox read,
normalized sorted metadata, synthetic-current exclusion, duplicate/invalid-data
refusal, preserved incomplete provider state, safe permission/timeout errors and
static diagnostic route naming. No dependencies, schema migrations, operation
records or VM writes were added. Contract and limits: docs/snapshots.md.

Verification: `cargo test -p vmanyware-server --lib snapshot_list -- --nocapture`
passes all four targeted tests (85 unrelated tests filtered out). They exercise
the actual Axum route and verified generated-HTTPS provider. An initial timeout
case failed because it used the normal client timeout; corrected the fixture to
use an explicit 200 ms deadline, then reran the four tests successfully. Formatting
and diff review cover this slice's Rust/docs changes; prior dirty Android/client
changes were retained without rerunning their earlier checks. No real Proxmox
request was made. Roadmap listing stays unchecked pending client/provider proof.
Next task: add the explicit read-only snapshot list to the client, including
empty/error/incomplete states and the distinction from independent backups.

## Remote snapshot-list UI — 2026-10-20

Previous goal turn made progress: scoped server snapshot listing and four targeted
HTTPS/API tests passed. This slice adds the remote native read command and actual
React list under each registered environment. Reads remain explicit and bound to
the active connection/canonical instance ID; replies are discarded after native
disconnect and UI unmount. UI binding additionally checks instance/scope/node/VMID.
Displays empty/provider-error/unknown/incomplete states and parent metadata,
escapes descriptions, and distinguishes snapshots from independent backups.
No creation, restore, new dependencies, schema changes or real VM requests.

Verification: `npm --prefix apps/client run build` passed. The focused
`node experiments/instances-browser/check.mjs --snapshots` passed with simulated
IPC: explicit read, exact selected target, mismatched target rejection,
empty/error distinction, metadata, escaped description, pending-read disconnect
and no mutation. The first browser attempt clicked the workspace button before
it became enabled; fixed that fixture synchronization, then the scenario passed.
Three native generated-HTTPS tests are added for fixed authenticated path,
malformed/authentication/path rejection and in-flight disconnect suppression.
`cargo test -p vmanyware-client --lib remote_snapshot_read -- --nocapture` stops in
webkit2gtk-sys because WebKitGTK 4.1 pkg-config metadata is absent; the new native
module/tests have not compiled or run here. Rust formatting, JS syntax and diff
checks pass. Prior dirty Android/server/client slices were preserved; no unrelated
suites were rerun. No new APK/native launch or real-provider evidence is claimed.
Next task: restore an owned native WebKitGTK build prefix and run the filtered
snapshot HTTPS tests, then validate the actual native list-to-server flow.

## Native snapshot command validation — 2026-10-20

Previous turn made progress: remote snapshot React UI and focused browser checks
passed, while native compilation stopped at a missing system library. Revalidated
that boundary, then recreated the seven-package WebKitGTK 4.1 library prefix in
ignored `.lab/native-deps`, using verified HTTPS, system public-keyring signature
verification and extraction without installation hooks. Keyring integrity: 17
files, zero alterations. Exact package versions/URLs/hashes/signature output and
setup helper remain in that owned directory. No system packages or personal
credential files were changed. See docs/native-linux-prefix.md for reuse.

Verification: actual `vmanyware-client` crate compilation passed, followed by all
three `remote_snapshot_read` tests (51 unrelated tests filtered out), proving the
fixed authenticated HTTPS path, unknown/incomplete metadata, invalid data/auth/
path rejection and in-flight disconnect suppression. Because the prior snapshot
slice modified shared ReadPath code, also ran the three `remote_workspace` tests
and the one fixed-health-path/token test; all four passed. No source correction
was needed. Native snapshot log: `.lab/native-deps/snapshot-native-tests.log`.
Documentation diff review and `git diff --check` passed; no frontend/server suites
were rerun. Earlier blocked-test notes are historical, superseded for this prefix.

Limitations: generated HTTPS peer, no WebView-driven snapshot IPC/server journey,
real provider, VM operation, packaged startup or mobile rebuild in this slice.
The prefix contains libraries only, not the previous Xvfb additions. Next task:
validate the real native WebView -> production command -> actual server snapshot
list with disposable generated infrastructure, then proceed to snapshot creation.

## Native snapshot-list vertical journey — 2026-10-20

Previous goal turn made progress by restoring a verified native library prefix
and passing the three snapshot plus four shared-read tests. This slice adds an
optional `--snapshots` mode to the existing isolated native fixture and a focused
real-UI driver, without changing normal application startup or injecting mock IPC.
The fixture registers production remote commands and connection state, uses an
actual authenticated server behind a verified generated HTTPS proxy, and reads
only a generated HTTPS provider. Lifecycle execution is disabled. The verified
owned prefix now also contains Xvfb/xauth/libxmu; all ten package signatures and
archive details are retained in `.lab/native-deps/packages.json`.

Acceptance met: explicit native reads show empty, incomplete and denied states,
retained target, unknown metadata, escaped description and backup wording; error
clears prior results and explicit disconnect succeeds. Runner asserts exactly
three fixed provider snapshot reads, zero provider writes and unchanged local
association. TLS, sandbox and X11 cookie checks were not disabled. Evidence:
`.lab/native-deps/snapshot-ui-check.log` ends with the full real-native journey pass.

Verification: built the affected example with `tauri/custom-protocol` and actual
server binary; native scenario passed. The initial imported production module
needed explicit child paths, now fixed for both create/snapshots; all three
filtered native snapshot tests passed again after that change. A plain initial
example build waited for Vite and expired without reaching the server; the
embedded-asset build corrected the invocation. JS/Python syntax, targeted Rust
formatting and `git diff --check` pass. No unrelated suite/frontend rebuild was
run. Owned display/process groups, proxy/provider and generated temporary data
were cleaned up by the runner. Minimal-session portal/accessibility warnings
remain; they did not prevent the asserted journey.

Limitations: isolated Linux/generated infrastructure only, not installed startup,
real Proxmox, mobile rebuild, snapshot creation or restoration. Roadmap list item
stays unchecked until real-provider evidence exists. Next task: implement bounded
validated snapshot creation and durable task/operation tracking, with explicit
target/action confirmation; keep real VM operations behind concrete approval.

## Snapshot creation prerequisites — 2026-10-20

Previous goal turn made progress: real native snapshot-list journey passed. This
slice prepares creation with `POST /api/v1/environments/{id}/snapshot-preflight`,
not a provider write. Validates exact 2–40 character Proxmox-compatible names,
reserved current, bounded UTF-8 descriptions and no unknown request options.
The fixed disk-only path reads actual provider feature support, existing snapshot
names/incomplete states, retained scope/target and lifecycle identity guards, then
rechecks pending/configuration identity. Complete reads are bounded to 20 seconds.
The report binds request/instance/scope/node/VMID/definition/digest and always says
`executionEnabled: false`. No reservation, approval, operation or VM write occurs.
ADR 0054 records disk-only semantics and the durable no-replay requirements for
the next slice. No dependencies, schema migration or client changes were added.

Verification: four filtered `snapshot_preflight` tests passed, then the added
actual HTTP bound-report test passed separately (five distinct tests total).
The HTTP test was repeated only after adjusting the body limit to 32 KiB and
adding a valid 4 KiB description whose escaped JSON exceeds 8 KiB; it passed.
Coverage includes request grammar/bounds, retained scope, fixed GET-only target,
provider support/malformed flags, duplicates, incomplete snapshot work, final
configuration changes, complete report and no operation record. Rust compilation,
targeted formatting and diff checks pass. Existing lifecycle/list/native/browser
suites were not rerun: their behavior was not modified. Generated HTTPS/local
SQLite only; no real provider, guest consistency or write permission was tested.

Creation itself remains incomplete: durable approval/expiry/hash, conflict guard,
no-replay dispatch marker, task persistence/reconciliation and client confirmation
must exist before exposing the mutation. Next task: add the confirmed snapshot
creation operation to the durable journal and worker, with crash/lost-reply tests;
real VM operations still require concrete target/action approval.

## Durable snapshot creation storage — 2026-10-20

Previous goal turn made progress: creation preflight and five targeted tests
passed. This slice adds schema 11/shared `vm_snapshot_create` operation storage,
full confirmed target/request hashing, retained same-key identity/age, common
mutation exclusion, incomplete-provisioning blocking, claim/expiry/exact-approval
guards, irreversible dispatch marker, qmsnapshot task binding, guarded completion
and existing exclusive-startup recovery semantics. Unknown submissions remain
uncertain and cannot be claimed; known tasks retain their marker for read-only
reconciliation/backoff. No provider POST, runtime worker, enqueue API or client
creation control is exposed; preflight stays disabled. ADR 0055 records choices.

Migration rebuilds the shared operations table, retaining provisioning children,
indexes and guards. It temporarily disables foreign keys on the unopened migration
connection, checks the graph inside the transaction and restores enforcement
before exposing the store. Invalid graphs roll back including schema version.
Normal schema-11 opens do not rescan the entire foreign-key graph. Existing tests'
expected latest schema versions were updated from 10 to 11; historical migrations
were retained unchanged. No dependencies or real infrastructure changes.

Verification: 53 existing affected storage tests passed (10 instance, 9 lifecycle,
34 provisioning-intent). Seven of the initial eight new integration tests passed;
the corruption fixture initially failed to inject an orphan because foreign keys
were enabled, so explicitly disabled them only in that owned fixture, then both
migration tests passed. Added concurrency, corruption/claim-rollback and actual
store-connection foreign-key enforcement checks; these passed. The expiry test
was repeated after adding proof that same-key retry cannot renew age. Total:
11 distinct new tests passed across filtered runs (10 integration, 1 unit), plus
53 existing affected tests. Targeted Rust formatting and diff checks passed.
No unrelated provider/native/browser suite was rerun; the broader storage coverage
was justified by rebuilding their shared table. SQLite/owned fixtures only; no
claim of provider dispatch, successful snapshot or restore is made.

Next task: implement fixed snapshot transport and runtime executor with explicit
capability, repeated preflight, dispatch-marker-before-POST ordering, retained task
polling and GET-only restart reconciliation; verify generated HTTPS crash/lost-
reply scenarios before exposing confirmed HTTP enqueue/client controls.

## Snapshot transport/executor and production queue — 2026-10-20

Previous goal turn made progress: schema-11 journal and affected storage tests
passed. This slice connects a fixed one-shot disk-only snapshot POST transport to
the durable executor. It rechecks exact target/preflight/digest, commits dispatch
before POST, persists only a matching qmsnapshot/node/VMID task before polling,
then requires successful task plus independently verified retained marker and
exact complete disk-only snapshot metadata. Snapshot creation changes digest;
completion verifies marker/template/lock rather than the old digest. Unknown
submission remains uncertain/unclaimed; known-task reopen/recovery is GET-only
and works with new dispatch disabled. Added safe snapshot-specific failure codes.

The main binary now validates independent VMANYWARE_SNAPSHOTS_ENABLED (absent/
false by default, exact true opt-in), and the sequential worker services one
snapshot claim after existing queues. Existing public worker entry points retain
default-disabled snapshot behavior. Confirmed HTTP enqueue/client submission are
not wired, and preflight still reports execution disabled. No write permission,
real VM operation, dependency or schema change was added in this slice.

Verification: six generated-HTTPS executor/transport tests passed, then the added
production-round test passed separately (seven executor tests total). They cover
encoded Unicode/ampersand/plus/newline, one fixed POST, pre-dispatch guards,
unknown/wrong task replies without replay after reopen, retained-task GET-only
reconciliation even when disabled, task failure and foreign/RAM metadata refusal.
The independent flag unit test and two actual-binary configuration/queue tests
passed (10 distinct new tests total). The binary tests use owned SQLite and no
provider: default-disabled fails without a marker; true reaches provider-not-
configured without a marker. Invalid flag stops before database creation and
never echoes its input. Actual native/UI/client/server-provider process journey
for creation is not claimed. Targeted Rust formatting and diff checks pass;
previously passing storage/native/browser suites were not rerun without changes.

Next task: add runtime-capability reporting and a confirmed idempotent snapshot
HTTP enqueue route bound to the whole reviewed target/request, with disabled,
stale, conflicting and same-key retry coverage; then add the client confirmation
and validate the real native creation journey against generated infrastructure.

## Confirmed snapshot HTTP submission — 2026-10-20

Previous goal turn made progress: one-shot transport, executor, opt-in runtime
queue and 10 targeted tests passed. This slice adds confirmed
`POST /api/v1/environments/{id}/snapshots`, capability-aware router/main wiring and
preflight runtime reporting. Existing router constructors default snapshots off.
Approval must match route ID, saved definition/scope/node/VMID and exact validated
snapshot request, with explicit consent and exactly one bounded idempotency key.
Configured bearer protection and origin refusal apply. Disabled/invalid/unconfigured/
changed/conflicting requests do not enqueue. Identical retries preserve operation
identity/age; accepted requests return 202/Location and never contact the provider.
Fresh preflight/digest checking stays in the worker; HTTP acceptance is not success.

Verification: seven new targeted HTTP tests passed across the initial run and
one corrected assertion rerun. They cover queue/read/retry identity, independent
flag, target/consent/unknown options, duplicate/missing keys, origin/size/auth guards,
active mutation conflicts and concurrent retries. Two new filtered HTTPS/API tests
also passed: actual enqueue -> production queue -> generated provider -> result
(success vs stale digest with zero POST), and capability independent of failed
prerequisites. Total nine distinct new tests passed. Two initial test assertions
assumed the wrong existing contract (unknown-field JSON rejection is 422, and
operation.failure is a string); corrected only those assertions and reran the
failed cases. No implementation failure was masked. Rust compilation, targeted
formatting and diff review pass. No schema/dependency/client changes or real VM
requests; unrelated suites were not rerun. docs/snapshots.md now describes the
current coherent contract rather than accumulating superseded pending statements.

Next task: native retained snapshot creation intent/preflight/confirm/status and
client review/consent, with same-key lost-reply resume and stale result suppression;
then exercise actual native creation through generated infrastructure. Real
provider snapshots, restore and complete roadmap acceptance remain open.

## Native snapshot creation intent — 2026-10-20

Previous goal turn made progress: confirmed HTTP enqueue and nine API/HTTPS tests
passed. This slice adds production native prepare/confirm/read/clear commands.
Preparation GETs the exact saved instance then POSTs read-only preflight, binding
all retained target/request fields, disk-only mode and the complete known check
set. Native revision checks reject superseded prepares before follow-up or state
replacement. Server check messages/unknown metadata are dropped. Confirmation
requires an eligible retained review, explicit consent and its existing key;
renderer cannot replace approval fields. It retains submitted/inflight/operation
identity, blocks duplicate concurrent POSTs, allows unchanged unknown-response
retry only, then GET-only status once an ID is known. Completion requires matching
recognized task-backed disk-only evidence. Explicit clear is limited to an
unsubmitted review or a verified terminal result; uncertain work stays retained.
Connection reset clears this memory, not the server operation.

Verification: eight new filtered `remote_snapshot_create` tests passed. Added a
security response test (authentication marker/status/JSON/256-KiB limit) and an
actual socket-abort test; both passed (ten distinct new tests total). The success
validator and happy HTTPS flow were repeated after requiring the provider-task
known flag, with a negative flag variant added and verified. Exact POST body/key
retention, scope/target/request/check mismatches, failed/disabled prerequisites,
invalid input, in-flight disconnect and stale preparation are covered. Actual
Tauri crate/command registration compile using `.lab/native-deps`; targeted Rust
formatting/diff checks pass. A test-only HTTPS sequence helper's visibility was
widened for reuse; its implementation is unchanged. No frontend/server suites,
APK builds or unrelated native tests were rerun without a behavior change.

Limitations: native commands + generated HTTPS only, no creation form, actual
creation WebView IPC/server journey, persisted native intent or restart recovery.
The existing snapshot list UI/real native listing evidence is unchanged. No new
dependency, server schema or real VM mutation. Next task: add the remote snapshot
review/consent/status form, same-key retry and explicit acknowledgement; then add
read-only server operation recovery for reconnect/restart and verify the complete
native creation journey against generated infrastructure.

## Remote snapshot creation form — 2026-10-20

Previous goal turn made progress: retained native snapshot commands compiled and
ten HTTPS tests passed. This slice adds the actual remote React creation form,
selected explicitly from an environment row. A single workspace review/operation
matches the native one-intent contract. It shows target, exact name/description,
known prerequisites and disk-only/backup/consistency limits. Review is bound to
the selected environment/request; consent is unchecked by default and unavailable
for disabled/failed prerequisites. Inputs/target stay locked until unsubmitted
cancel or verified terminal acknowledgement. Unknown submission exposes identical
same-key retry only; known operations expose explicit GET status refresh. Uncertain
work cannot be acknowledged/replaced. Completion checks reviewed task-backed
metadata before showing success. Disconnect resets selection and suppresses late
responses; it never claims server work was cancelled.

Verification: `npm --prefix apps/client run build` passed TypeScript and Vite.
`node experiments/instances-browser/check.mjs --snapshot-create` passed the focused
real Firefox/React walkthrough under simulated IPC: no automatic work on target
selection, disabled/failed/mismatched review refusal, escaped description, consent,
input/target exclusion, same-key retry, queued/running/uncertain/success read states,
terminal acknowledgement and stale preparation after disconnect. JS syntax and
diff review passed. No Rust/native/server tests were rerun: those sources were not
changed. No new dependency, real VM operation or credential change was made.

Limitations: this is frontend/browser evidence; no native creation window, APK
rebuild or real-provider creation has been tested. Intent/operation recovery after
closing/reconnecting is not implemented; operation ID is shown for preservation.
Next task: exercise the actual native WebView -> retained commands -> authenticated
server -> generated HTTPS provider creation path, then add explicit read-only
server operation recovery for reconnect/restart. Restore and real-provider gates
remain open; the full roadmap is still active.

## Actual native snapshot creation journey — 2026-10-20

Previous goal turn made progress: the React creation form/build and focused
Firefox scenario passed. This slice adds a distinct --snapshot-create mode to
the existing isolated native fixture and registers production creation commands
in the example. The owned server alone opts into snapshots. A verified generated
HTTPS proxy forwards the real preflight/confirmed request, token and idempotency
key to the actual server; its worker contacts only the generated HTTPS provider.
The example embeds the current frontend assets and invokes real native commands.
No application/product source change was needed for this integration proof.

Acceptance met: review/cancel, fresh unchecked review/consent, one confirmed
submission, explicit operation GET refresh, task-backed metadata success,
independent list, acknowledgement and disconnect. The provider accepts exactly
one fixed target/name/description/vmstate=0 POST. Unicode/ampersand/plus/newline
content is checked. The runner requires five snapshot reads, at least one task
poll, unchanged instance association and exactly one succeeded durable operation
with providerTaskKnown, successful task status and complete disk-only metadata.

Verification: targeted native example build with tauri/custom-protocol and actual
server binary build passed. First native scenario passed; repeated only after
adding independent task-poll and server-journal assertions, then passed again.
Evidence: .lab/native-deps/snapshot-create-ui-check.log. Targeted Rust formatting,
JS/Python syntax and git diff checks pass. Frontend/native/server unit suites were
not repeated without source changes. Fixture listeners, process groups and
generated temporary state were cleaned up. TLS, WebKit sandbox and X11 cookie
checks stayed enabled; minimal-session portal/accessibility warnings remain.

Limits: isolated Linux/generated provider only, not a real VM, Android rebuild,
physical device, normal packaged startup, application consistency or restore.
Next task: explicit read-only snapshot-operation recovery from the server after
reconnect/restart, without resubmitting the snapshot or inventing current VM state.
Full roadmap and real-provider snapshot/restore acceptance remain open.

## Scoped server snapshot recovery — 2026-10-20

Previous goal turn made progress: actual native snapshot creation passed with one
provider POST and independent task/journal evidence. This slice adds GET-only
per-instance operation history/detail recovery. It filters instance/kind before
reading another journal, returns recorded target and original request without
approval/grant/task-secret fields, checks retained approval/task identity, and
normalizes successful evidence against exact saved target/request/disk-only
metadata. Unexpected success fields are discarded. Reads are consistent SQLite
transactions; no provider, capability, prepared client intent, claim or enqueue
is needed. History filters before its newest-100 limit and omits large request/
result fields. Bearer protection remains common to all API routes.

Verification: six new HTTP/SQLite tests passed. Added a saved-success check after
introducing typed result normalization; it also passed (seven distinct new tests).
Coverage: database reopen with provider/capability absent, unchanged queued/
running/uncertain operations, scope/kind/ID isolation, old target visible despite
another target's large history, ordering/bounds, corrupted approval rejection
without cross-target disclosure, valid success/unknown-field removal, mismatched
success refusal and authentication. Targeted Rust formatting and git diff checks
pass. Added Deserialize only for existing normalized evidence types; no prior
write behavior, schema, dependency, frontend or native command changed. Existing
passing write/native/browser suites were not rerun without related changes.

Limits: API/journal recovery only. This does not contact a real provider or prove
current snapshot existence/restore, launch/restart the production server process,
or add client recovery controls. Next task: native fixed GET history/recovery
commands and an explicit client history/saved-ID flow that works after connection
reset, binds the selected instance, ignores stale replies and never submits the
saved snapshot again.

## Native saved snapshot recovery — 2026-10-20

Previous goal turn made progress: seven scoped server recovery tests passed.
This slice registers native read_remote_snapshot_history/recover_remote_snapshot
commands. Both use canonical instance/operation IDs, fixed GET paths and the
retained verified HTTPS/token profile with response/authentication bounds. History
checks target binding, 100-entry bound, uniqueness, names and status/failure/task
consistency. Saved detail validates requested identities, recorded target/request,
submission marker and exact task-backed complete disk-only success evidence.
Unknown fields are dropped and raw read failures are replaced by static messages.
Old connection replies are discarded after disconnect or reconnect. Reads do not
create/replace a prepared approval or retain new submission/retry state.

Refactored the existing creation operation validator/name grammar into shared
internal helpers for this working read use case. Creation still validates its
own retained Review; recovery supplies recorded target/request directly, without
inventing a preflight. No dependency, server schema or frontend change.

Verification: seven filtered remote_snapshot_recovery tests pass against generated
verified HTTPS: fixed authenticated GETs without intent, historical states,
identity/target/metadata mismatch refusal, bounded/unique history, disconnect and
reconnect suppression, unknown-field removal, private error handling and invalid
IDs before network access. Actual Tauri crate/command registration compiles with
.lab/native-deps. The initial test build exposed a local helper-name shadowing
error, corrected before execution. Re-ran only the two affected existing creation
validator/happy-path tests after sharing validation; both pass. Targeted Rust
formatting and git diff checks pass. No server/browser/Android or unrelated native
suite was rerun without changes.

Limits: native command/HTTPS evidence only. Recovery is not yet exposed in the
React UI and no fresh recovery WebView/process-restart journey is claimed. No
real provider contact, mutation, credential change or persisted client state.
Next task: per-instance read-only history and saved-operation-ID UI, with explicit
saved/current-state wording, stale result suppression and no submission controls;
then verify it through the actual native fixture after reconnect/restart.

## Snapshot recovery UI — 2026-10-20

Previous goal turn made progress: native GET-only recovery compiled and seven
HTTPS plus two affected creation tests passed. This slice adds a per-instance
React recovery panel below snapshot listing: explicit history, row inspect,
saved-operation-ID lookup and read-only refresh. Selected target/request identity
is checked before display; successful records need matching task-backed disk-only
evidence. The panel distinguishes recorded completion from current snapshot
existence/restore and handles unknown submission/failure without mutation controls.
ID editing invalidates pending results; disconnect/unmount ignores late replies.
A new connection starts empty but can read server records without prepared consent.
No client state persistence, approval restoration, write or automatic read occurs.

Verification: npm --prefix apps/client run build passed TypeScript/Vite.
node experiments/instances-browser/check.mjs --snapshot-recovery passed the focused
Firefox/React simulated-IPC scenario: explicit history/empty/error states, row/
saved-ID reads, saved/current wording, escaped description, unknown/failed states,
wrong-target refusal, stale edited-ID detail and disconnected history-error
suppression, fresh-connection recovery and zero creation commands. JS syntax and
git diff checks passed. Rust/server/native/Android suites were not repeated because
those sources did not change. No new dependency or real VM operation was used.

Limits: browser UI plus earlier native HTTPS/server evidence, not a completed
native recovery WebView/process restart journey. Next task: extend the owned native
fixture to reconnect/restart the app after snapshot creation, recover the saved
operation/history through real commands/server and prove no additional provider
POST; then continue snapshot restore with explicit destructive confirmation and
durable task tracking. Real-provider gates and the full roadmap remain open.

## Actual native snapshot recovery after app/server restart — 2026-10-20

Previous goal turn made progress: client read-only recovery built and passed the
focused Firefox scenario. This slice adds a --snapshot-recovery combined mode to
the owned native fixture and registers production recovery commands in the example.
It creates once through the real native UI, closes that application, saves the
operation, stops the actual server cleanly, then launches new server/app processes
on the same database. The new server explicitly disables snapshots and removes
all generated Proxmox configuration. New native connection uses the supplied
fixture HTTPS/token/CA; old token/operation ID/creation form are not restored.

Acceptance met: history lookup, copied saved-ID read and refresh show the correct
recorded target/request/task metadata and historical/current distinction, without
submission controls. Recovery proxy traffic is GET-only; provider read/POST counts
stay unchanged and the full succeeded operation equals its pre-restart value.
The creation phase still proves exactly one disk-only POST, independent task
polling and one succeeded journal entry. All infrastructure/certificates/tokens
are generated and disposable; no personal credentials or real VM are accessed.

Verification: targeted embedded-asset native example and actual server binary
builds passed. The combined native scenario passed on its first run; log:
.lab/native-deps/snapshot-recovery-ui-check.log. JS/Python syntax, targeted Rust
formatting and git diff checks pass. No product source, dependencies or schema
changed; unrelated frontend/unit/Android checks were not rerun. Owned display,
server/app process groups, proxy/provider and temporary data were cleaned up.
TLS, sandbox and X11 cookie verification stay enabled; minimal-session portal/
accessibility warnings remain without preventing the assertions.

Limits: succeeded saved operation after clean app/server restart only, in the
isolated Linux fixture. A live provider crash, physical/mobile/installed runtime,
current snapshot existence and actual restore are not proven here. Next task:
read-only restore preconditions and explicit destructive approval bound to the
selected complete disk-only snapshot and retained target, followed by durable
no-replay rollback task tracking and generated-provider/native restore checks.
Real-provider snapshot/restore and the full roadmap remain open.

## Read-only restore preconditions — 2026-10-20

Previous goal turn made progress: actual native app/server restart recovery
passed with GET-only traffic and unchanged journal. This slice adds bounded
snapshot-restore-preflight API/domain/provider reads. It binds the exact named
snapshot, live target digest and canonical SHA-256 of API-visible saved config,
checks stopped power before/after, metadata/config stability, marker/template/
lock/pending guards, complete disk-only snapshot work and provider snapshot feature
for the selected config. No dedicated snapshot revision digest is exposed by the
upstream config API, so hash canonical JSON locally and never forward the raw
configuration. Added recursive ordering and domain/name separation. ADR 0056
records limits, including RAM snapshots that may auto-start during rollback.

The report is explicitly destructive, startsVm=false and executionEnabled=false,
even with creation/lifecycle flags enabled. There is no rollback/power transport,
operation or approval in this slice. Shared identity flag decoding and snapshot
name grammar were extracted without changing existing contracts; current VM
digest still requires the provider's 40-character format. No schema/dependency/
client change or real provider operation was made.

Verification: six filtered snapshot_restore_preflight tests passed against owned
generated HTTPS/API fixtures, including fixed 12-GET positive sequence, stable
canonical fingerprint, 16 blocked conditions plus malformed RAM data, exact
retained target, independent disabled execution, invalid/scope/permission failures
and raw-data suppression. The initial new test build had a temporary borrow in
fixture setup; binding the scope string fixed it before running. Because identity/
name helpers are shared, reran five affected lifecycle_preflight tests and the
existing name/input-validation test; all six passed. Targeted formatting and
git diff checks pass. Unrelated storage/native/browser/Android suites were not run.

Next task: full destructive restore approval (including data-loss acknowledgement,
live digest and selected-config fingerprint), shared mutation exclusion and durable
no-replay rollback task storage. Then fixed rollback transport/executor, explicit
client confirmation and generated/native restore verification. Stopped/disk-only
preflight does not prove storage rollback, write permissions, application
consistency or recovered guest contents; real infrastructure acceptance stays open.


## Durable destructive restore journal — 2026-10-20

Previous goal turn made progress: six restore-preflight HTTPS/API tests and six
shared identity/name checks passed. This slice adds mandatory confirmation plus
explicit data-loss acknowledgement bound to the full target, current digest,
selected snapshot name and canonical saved-config hash. Unknown/start fields are
rejected. Restore has its own input-hash domain and operation kind. Same-key
retries keep the original identity/age; dispatch needs exact fresh approval and
an irreversible marker. Only qmrollback tasks with matching node/VMID can be
retained. Unknown submissions never reclaim; known-task recovery cannot dispatch
again. Storage success needs task success, matching snapshot/hash and a stopped
observation of the exact target; this does not prove guest contents.

Schema 12 adds restore to shared mutation exclusion and the incomplete-provisioning
trigger. Migration retains old snapshot/lifecycle/provisioning rows and child
journals, checks the reference graph atomically and restores FK enforcement before
returning a store. The slice adds no dependency, client control, provider transport or runtime
write opt-in. ADR 0057 records contract and limits.

Verification: eleven distinct new restore-storage tests passed, including consent,
idempotency/expiry, concurrent connections, target/snapshot creation exclusion,
crash windows, corrupt-input rollback, task identity, completion guards, actual
schema-11 snapshot/child-journal preservation and atomic migration refusal. The
corruption fixture initially failed to inject its orphan because its connection
correctly enforced foreign keys; explicitly disabled enforcement on that owned
fixture connection before injecting corruption, then the migration refusal test
passed. The eight initial tests passed first; reran only changed/new filters after
adding migration and stronger completion cases.

Because the operation-table schema and active-mutation constraints affect existing
callers, reran 9 lifecycle-storage, 34 provisioning-intent and 10 snapshot-storage
tests; all 53 passed. The affected v2 instance migration and store FK-enforcement
checks also passed. Latest-schema assertions changed from 11 to 12; historical
migration files remain unchanged. Targeted Rust formatting and diff checks pass.
No application/native/browser/Android or real-provider suites were repeated.

Next task: fixed one-shot disk-only rollback transport and an opt-in executor that
revalidates the reviewed target/snapshot before dispatch, retains the task before
polling and reconciles known tasks through GET only. Then HTTP/client explicit
confirmation and generated/native checks. Restore preflight remains disabled;
no actual rollback, guest-content recovery or real-infrastructure acceptance is
claimed. Full roadmap remains open.


## Opt-in durable rollback executor — 2026-10-20

Previous goal turn made progress: restore approval/storage, mutation exclusion and
schema-12 migration passed 11 new and 55 affected existing checks. This slice
connects fixed one-shot rollback transport and the durable restore executor to the
sequential production worker. It revalidates all stopped/disk-only target/snapshot
checks plus the confirmed live digest and saved-config hash before committing the
marker. POST has only start=0 at the fixed retained snapshot path. The matching
qmrollback task is persisted before polling. Task success then requires fresh full
preflight, original saved hash and stopped owned target; the pre-write live digest
is not required after rollback because rollback changes configuration. The final
power observation is included in the report/result. No raw saved config escapes.

VMANYWARE_RESTORES_ENABLED is a separate exact true/false flag, default false.
Existing creation/lifecycle/provisioning opt-ins do not grant restore. Invalid
configuration stops startup before database creation. Known tasks reconcile GET
only with all write flags off; lost/invalid submission responses stay uncertain
and never reclaim. No restore HTTP submission or client controls exist yet;
HTTP preflight stays execution-disabled until that submission route is added.
Updated ADR 0057 and snapshots documentation, preserving the open roadmap gates.

Verification: seven new filtered restore_execution tests passed against generated
verified HTTPS. Positive worker journey has exactly 26 requests: 12 preflight
GETs, one fixed POST, one task GET and 12 postcheck GETs. Covers 17 pre-dispatch
conditions, four lost/wrong node/VMID/kind response cases across reopen, 13-GET
known-task restart with writes off, 14 failed task/poll/postcheck conditions,
invalid names and default queue behavior. The final transport filter separately
checks 401/403/302/500/malformed responses, private error suppression and one POST
without redirects/retries. No prior passing executor filters were repeated after
adding this independent test.

One new exact opt-in unit test and two new actual-server process tests passed.
The two affected existing snapshot process tests and one existing restore
preflight positive test also passed. Targeted formatting and diff review pass.
No dependency/schema/storage implementation changed in this slice; earlier dirty
changes and unrelated native/browser/Android/storage suites were not repeated.
Owned fixtures/processes were cleaned up. No real VM, credentials or production
infrastructure were accessed or changed.

Next task: confirmed restore HTTP submission bound to path/target/snapshot and
one idempotency key, independent capability reporting and scoped safe responses;
then native retained review/data-loss confirmation/status and actual generated
native restore journey. Provider completion here does not prove restored guest
contents, write permissions on real storage or phase-0D/phase-4 acceptance. Real
modify/restore/readback and the rest of the roadmap remain open.


## Confirmed restore HTTP submission — 2026-10-20

Previous goal turn made progress: fixed rollback/executor and opt-in production
worker passed 13 targeted checks. This slice exposes POST environments/{id}/
snapshots/{snapshotName}/restore with a 4 KiB approval limit, both path identities
bound to the complete confirmed target/request and mandatory data-loss agreement.
Exactly one idempotency key is required; Origin-bearing requests are rejected.
The handler checks the independent restore capability, provider scope and saved
instance, then only enqueues. Same-key retries retain identity/age and return
terminal/uncertain records without replay. Location points to operation GET.
Static errors/diagnostic route templates omit selected names and raw request data.

The actual production router now receives the same restore flag as its worker.
Old router constructors default false; creation/lifecycle/provisioning flags do
not grant restore. Preflight reports this capability independently of eligibility;
true never bypasses stopped/disk-only checks. No schema/dependency/native/client
change, new runtime flag or real provider operation was introduced in this slice.
Earlier dirty source changes remain intact. ADR 0057/API/snapshots docs updated;
roadmap acceptance remains open.

Verification: eight new restore_api tests passed: exact approval/path binding,
mandatory confirmations/fingerprint, unknown options/malformed/oversized bodies,
Origin/key policy, independent capability, scope/target guards, saved operation
lookup, concurrent idempotency, changed-input and mutation conflict, bearer auth
and safe errors. The first test compilation found two incorrect format arguments
in the new adapted fixture; corrected before running. Two new filtered restore_http
verified-HTTPS tests passed: HTTP enqueue -> production round -> saved result for
success, stale live digest, stale selected hash and lost submission reply; same-key
terminal/uncertain retries do not write again, and query retargeting is ignored.
Preflight capability is independently checked with both eligible and failed checks,
exact 12 GETs, no journal/write, and true/false flag states.

Because shared router construction and production flag forwarding changed, reran
seven affected existing snapshot API tests, the existing restore-preflight target/
creation-flag test, and the actual restore process test enhanced to send a real
HTTP request. All nine pass (19 distinct checks in this slice). Actual server
returns disabled 403 or configured-provider-absent 503 as expected; generated
processes clean up. Targeted formatting/diff checks pass. Earlier executor/storage,
unrelated UI/native/Android and broad suites were not repeated.

Next task: native retained restore prepare/confirm/read/ack commands that bind the
reviewed snapshot/configuration, require separate data-loss consent, preserve an
unknown same-key retry, reject stale/disconnected replies and never replay a known
operation. Then explicit React restore review/status and generated/native journey.
Provider completion is not real guest-content proof; no actual rollback or phase
0D/phase-4 infrastructure acceptance is claimed. Full roadmap remains open.


## Native retained destructive restore commands — 2026-10-20

Previous goal turn made progress: confirmed restore HTTP and independent capability
passed 19 targeted checks. This slice registers native prepare/confirm/read/clear
restore commands and retains one restore review/key in the current connection.
Prepare reads the saved target then fixed preflight, binds target/request/revisions,
requires all 13 distinct check codes and validates destructive/stopped/disk-only
report consistency. Missing-snapshot failure can be reviewed but not submitted.
Confirm requires separate displayed-target and data-loss booleans; full approval
is constructed from the native retained review, never renderer revisions or URL.

Submitted intent is retained before network await. Unknown response only permits
same-key/body retry; known operation blocks POST and uses GET. Success validates
exact target/name/hash plus task-backed stopped observation. Inconsistent definite/
uncertain status combinations fail validation; uncertain state cannot be cleared.
Only unsubmitted or verified terminal intent can be acknowledged. Generation
changes clear local restore memory and reject late prepare/submission/status
replies without server cancellation. No persisted client consent or credential,
React controls, server/schema/dependency changes or real VM operation were added.

Reuse the creation name grammar and bounded authenticated snapshot HTTPS helper;
changed its denial text to generic snapshot action instead of creation. Added
Clone to normalized snapshot metadata for retained restore review. Existing
creation/recovery behavior remains otherwise unchanged. ADR 0057 and snapshot
workflow docs describe the native boundary and remaining gates.

Verification: actual Tauri/native command registration compiles using the signed
.lab/native-deps prefix. Fourteen filtered remote_snapshot_restore tests pass with
generated verified HTTPS: full prepare/dual-confirm/read/ack request sequence,
identical unknown-response retry, disabled/failed reports, 15 malformed target/
selection/check/power/fingerprint variants, exact task-backed success, status
uncertainty, auth/status/malformed/size limits, in-flight disconnect, stale prepare,
actual socket loss, missing-snapshot review, known uncertain GET-only completion
and late disconnected status refusal. Initial ten tests passed; strengthening the
shared new restore validator justified rerunning this slice's fourteen after
adding uncertainty cases. One initial unused test-helper warning was resolved by
asserting reset behavior in the disconnect test. Import cleanup compiled with the
subsequent checks, without warnings.

Because the existing HTTPS helper denial text is shared, reran only the affected
creation happy path and authentication/status/size response test; both pass.
Targeted Rust formatting and diff checks pass. No server/storage/browser/Android
or unrelated native suite was repeated. No real provider or infrastructure contact.
Next task: explicit React restore review showing VM/snapshot and current-data loss,
independent unchecked consent, same-key unknown retry and GET-only status, with
stale-reply suppression; then actual generated native restore journey. Native
command evidence alone does not prove WebView/mobile behavior or restored guest
contents. Full roadmap and real-provider gates remain open.


## Explicit remote client restore review — 2026-10-20

Previous goal turn made progress: native retained restore commands compiled and
passed 14 new plus two affected HTTPS tests. This UI slice adds restore selection
from the exact named snapshot row and a single retained review panel. Selection
makes no request until explicit review. The panel displays VM/name/node/VMID and
explains current-data/settings loss, stopped/disk-only requirements, power limits
and snapshot dependence on original storage. It binds reviewed target/name and
requires the full 13-check passed report, exact fingerprint, stopped observation
and disk-only selection before enabling consent.

Target confirmation and loss acknowledgement are separate initially unchecked
checkboxes. Both must be checked. Confirm sends only native connection/key/booleans;
revisions and approval remain native. Unknown response retains the same key/consent
and only offers same-request retry; known operation exposes GET refresh only.
Success evidence must match target/name/hash/task/stopped observation, with explicit
unverified guest-content wording. Uncertain work cannot be acknowledged. A parent
selection/lock prevents switching VM/snapshot or snapshot creation while retained.
Disconnect resets selection/consent and generation checks discard late replies.
No persistence, dependency, Rust/server/schema or real VM mutation was introduced.

Verification: npm --prefix apps/client run build passed TypeScript/Vite.
The new focused Firefox --snapshot-restore scenario passed first run using actual
React with simulated IPC. Covers no automatic request on list/selection, disabled/
running/mismatched review refusal, escaped description, both independent unchecked
consents, target/creation exclusion, same-key retry after response loss, queued/
running/uncertain GET refresh, invalid success fingerprint refusal, terminal ack,
fresh connection and late prepare/status suppression. It checks confirm arguments
cannot introduce URL/revision/hash fields and no creation command is invoked.
Because shared environment/list controls changed, reran the existing focused
--snapshot-create and --snapshots browser scenarios; both pass. JS syntax and
repository diff checks pass. Earlier native/server/storage/Android and unrelated
frontend suites were not repeated without source changes.

Updated snapshot workflow/ADR/browser-fixture docs; existing dirty work remains
intact. Next task: extend the isolated native fixture to exercise the actual
WebView -> retained native restore commands -> authenticated API/worker -> generated
HTTPS provider rollback, proving one fixed POST, independent completion evidence
and no replay. Browser/simulated IPC does not prove this native journey, deployed
or Android behavior, real storage rollback or restored guest contents. Real
phase-0D/phase-4 acceptance and full roadmap remain open.


## Actual native restore with lost HTTP submission response — 2026-10-20

Previous goal turn made progress: explicit restore UI built and focused Firefox
restore/creation/list scenarios passed. This slice adds a distinct mutually
exclusive --snapshot-restore native fixture mode and registers production restore
commands in the owned example. Actual WebView, native retained HTTPS state,
authenticated API/SQLite/worker and generated provider all participate; no IPC
mock or direct fixture submission bypass is used. Normal application startup does
not link fixture drivers. No product source, dependency or schema changed.

The driver selects the existing named disk-only fixture snapshot. First read-only
review observes a running VM and cannot consent; subsequent synthetic reads show
it stopped without power mutations. It cancels an eligible review, prepares again
with two unchecked consents, requires each independently and confirms. The HTTPS
proxy drops the first response only after real API 202/commit. Native retains the
key/review; exact retry returns the same operation. Provider receives exactly one
fixed rollback POST with start=0; creation/lifecycle flags are absent. Status/list,
terminal ack and disconnect work, without a new provider write.

Independent assertions pass: exactly two identical API path/key/body submissions;
one succeeded vm_snapshot_restore journal entry with known successful task; exact
instance/scope/node/VMID/name; canonical saved-config fingerprint independently
computed in Python; stopped observation; unchanged instance association. Counts:
12 snapshot-list GETs, 10 saved-config GETs, 10 power GETs and at least one task
poll. Live digest changes after rollback, while selected saved-config hash stays
stable. This is provider completion evidence, not restored guest-file contents.

Verification: current embedded-asset native example and actual server builds
passed. Initial scenario failed in its new driver before connecting because the
CA textarea was filled with an input-only setter; corrected element dispatch,
rebuilt only the affected example and reran the same scenario. It passed with all
independent assertions. Final log .lab/native-deps/snapshot-restore-ui-check.log;
earlier driver failure .lab/native-deps/snapshot-restore-ui-check.initial.log.
Python/JS syntax, targeted Rust formatting and diff review pass. Existing unit/
UI/Android suites were not repeated without product changes. TLS, sandbox and X11
cookie checks remain enabled; minimal-session portal/accessibility warnings remain
nonfatal. Owned processes, listeners and temporary state were cleaned up.

Next task: scoped read-only restore operation history/detail with recorded target/
request, so reconnect/restart recovery never submits a rollback or recreates
consent; then native/client saved-ID recovery and actual app/server restart proof.
No real VM/provider contact, mutation, credentials or production change occurred.
Real storage rollback, recovered guest state, physical/mobile/packaged runtime and
phase-0D/phase-4 infrastructure gates remain open. Full roadmap remains active.


## Scoped read-only saved restore recovery — 2026-10-20

Previous goal turn made progress: actual isolated native restore passed with lost
HTTP response, identical API retry and one provider rollback POST. This slice adds
GET-only per-instance restore operation history/detail. History filters instance/
kind before newest-100 ordering/limit and omits request/result bodies. Detail
returns original target/request/config fingerprint and dispatch marker, without
approval/grant flags, live digest, request key, confirmation age, input hash, raw
task ID or saved configuration. The existing immutable target DTO is reused.

Reads use consistent SQLite transactions, resolving scope/kind before decoding
other records. Validate saved approval/hash and qmrollback task, dispatch/status/
failure consistency and exact task-backed successful target/name/fingerprint/
stopped observation evidence. Normalize success through typed public evidence to
strip unknown nested fields. Refuse corrupt/inconsistent success or unrelated
workflow failures without changing records or disclosing their raw data. No
provider, preflight, claim, consent renewal, state recovery, write capability or
new submission is needed. Configured bearer protection remains common.

Verification: eleven distinct focused restore_recovery HTTP/SQLite tests passed.
Initial seven passed, then added/strengthened only affected filters for known
running/uncertain/interrupted/auth-refusal/task-failure states, wrong task/dispatch
combinations, same-instance creation isolation, saved success after reopen,
unknown nested field suppression and six mismatched completion fields. The final
failure-code whitelist change was checked with its new corruption test and the
two affected failed-record history/isolation tests, without repeating unrelated
passing filters. Coverage also includes queued reopen with no provider/flags,
unchanged journal, instance/kind/ID guards before corruption errors, independent
100-row history limit/order and bearer auth. All tests use owned SQLite; no real
infrastructure or credential files are accessed.

Targeted Rust formatting/diff checks pass. No dependency/schema, existing executor
or creation behavior, runtime configuration, native/frontend code changed; earlier
passing storage/native/browser/Android and broad suites were not rerun. This proves
read-only API/database reopen, not a new application/server-process restart test.
Updated API/snapshot/ADR docs and left roadmap gates open.

Next task: native fixed authenticated GET restore-history/detail commands with
recorded target/request/fingerprint validation, bounded private responses, current
connection checks and no prepared intent/submission; then explicit React saved-ID/
history recovery and actual app/server restart verification. Read completion is
historical provider evidence, not current VM state or real guest-content recovery.
Full roadmap and real phase-0D/phase-4 acceptance remain open.


## Native saved restore history/detail recovery — 2026-10-20

Previous goal turn made progress: eleven scoped restore-history/detail API/SQLite
tests passed. This slice registers native read_remote_restore_history and
recover_remote_restore commands. Canonical IDs select fixed GET paths through
the retained verified HTTPS/token profile, with bearer marker, exact status and
response size limits. History validates scope/count/unique IDs/names/timestamps
and status/failure/task consistency. Detail validates requested identities,
recorded target/request/config fingerprint, submission marker and exact task-backed
successful stopped observation. Unknown nested fields and raw error bodies are
not exposed. Read failures use static read-oriented text.

Connection generation is checked after await; disconnect/reconnect rejects late
reads. Recovery creates no review, grant, key, retry state or submission. Share
existing restore request/operation validation for recorded context; added a
dispatch-consistency check and tightened pre-dispatch failure codes to the server's
restore vocabulary. Historical success remains provider evidence, not current
power or guest recovery. No server/schema/dependency or frontend change occurred.
Earlier dirty changes remain intact.

Verification: actual Tauri/native command registration compiles with the isolated
.lab/native-deps prefix. Nine focused remote_restore_recovery tests pass with
generated verified HTTPS: two fixed authenticated GETs without prepared intent;
queued/running/unknown/terminal and known-task states; exact target/request/hash/
power/dispatch checks; history bounds/uniqueness; read auth/status/malformed/size/
private errors; nested unknown field removal; invalid IDs before network; old
connection reply suppression. Initial seven passed, then reran this slice after
extending restore-specific guards and adding two status/known-task checks.

The shared existing restore operation validator changed, so reran only its status
validation and retained prepare/confirm/status happy path; both pass. Targeted Rust
formatting and diff checks pass. No server/storage/React/browser/Android or unrelated
native suite was repeated. No real provider or credential files were accessed.

Next task: per-instance React saved-restore history/manual-ID/read-only refresh,
with recorded/current wording, target binding, stale ID/disconnect suppression and
no consent/submission controls; then actual app/server restart recovery fixture.
Native command evidence alone does not prove WebView/mobile/restart integration
or recovered guest contents. Full roadmap and real-provider gates remain open.


## Client saved restore history and manual-ID recovery — 2026-10-20

Previous goal turn made progress: native GET-only saved restore recovery compiled
and passed nine new plus two affected HTTPS tests. This UI slice adds a separate
per-instance saved-restore panel below creation recovery. Explicit history, row
inspection, manual saved ID and read-only refresh work without snapshot listing
or prepared consent. Detail binds selected instance/scope/node/VMID and operation
ID/kind; success also needs matching saved fingerprint/name/task/submission and
stopped target evidence. Saved completion is explicitly distinct from current VM,
snapshot existence or guest-file recovery. Uncertain/failed outcomes remain clear.

ID edits invalidate in-flight results and reset detail/error/busy state; unmount/
disconnect suppress stale responses or failures. Fresh connection starts empty but
can read the server record. Recovery exposes no consent, prepare, submit, retry
write, cancel or acknowledge action. Existing creation history remains separate.
No native/server/schema/dependency or real provider changes occurred in this slice;
earlier dirty work remains intact.

Verification: TypeScript/Vite build passed. New focused Firefox --restore-recovery
passed, then was extended with wrong-success fingerprint refusal and rerun after
small restore wording corrections; passes all cases. Actual React/simulated IPC
covers no automatic reads, history empty/error distinction, row/manual-ID lookup,
recorded/current/guest-state wording, unknown/failed states, wrong target/hash,
stale edited-ID detail, late disconnected history error and fresh-connection
lookup. Assertions prohibit any creation or restore mutation command. The shared
snapshot area now mounts another panel, so ran only the affected existing
--snapshot-recovery scenario too; it passes. JS syntax and diff review pass.
Earlier Rust/storage/native/Android or unrelated UI suites were not repeated.

Next task: actual isolated native restore followed by clean app/server restart,
provider configuration removed and write flags off, recovering history/saved ID
through real native commands with GET-only traffic and unchanged operation. This
UI/browser evidence does not prove that integration or restored real guest state.
Full roadmap and phase-0D/phase-4 infrastructure acceptance remain open.


## Actual native saved restore recovery after app/server restart — 2026-10-20

Previous goal turn made progress: client saved-restore lookup built and focused
restore/creation recovery browser checks passed. This slice adds mutually exclusive
--restore-recovery combined mode to the owned native fixture, registers production
GET history/detail commands in the example and adds a second-process recovery
driver. The initial actual native restore retains running-target refusal, cancelled
review, dual consent and same-key API retry after a deliberately lost response.
Exactly one rollback POST and independent completion evidence remain required.

The first app exits. Runner saves the succeeded operation, stops the actual server
cleanly, then starts a new server process on the same SQLite database with all four
write flags explicitly false and all Proxmox settings removed. A second native
app process starts empty and receives the generated connection explicitly. It
finds history/row detail, reads the copied saved ID and refreshes it through actual
commands/API. Old token/input/restore intent/consent are not restored automatically.

Acceptance passed: new recovery proxy traffic is GET only; provider read/POST
counts stay unchanged; the whole succeeded operation equals its saved value.
The initial restore still proves two identical API path/key/body submissions,
one provider rollback, exact selected-config fingerprint and stopped observation.
No product source, dependency/schema, real VM or credential change was needed.

Verification: targeted embedded native example and current server builds pass.
The combined actual native scenario passes on first run; log
.lab/native-deps/restore-recovery-ui-check.log. Python/JS syntax, targeted Rust
formatting and diff checks pass. Unrelated unit/browser/Android suites were not
repeated without changes. Owned listeners, process groups and temporary data were
cleaned up. TLS, sandbox and X11 cookie checks remained enabled; nonfatal minimal-
session portal/accessibility warnings remain.

Limits: clean succeeded-record recovery only, isolated Linux/generated provider.
No live rollback crash, real disk/guest restore, deployed/mobile/installed runtime
or phase-0D/phase-4 infrastructure acceptance is proven. Next task: read-only
retained-instance deletion prerequisites with ownership/power/lock/pending guards
and exact disk/deletion scope for later explicit delete/recreate approval. Roadmap
still requires delete/recreate, real-provider acceptance, remaining platform work
and security/packaging/release gates; full goal remains active.


## Read-only deletion target and candidate volume review — 2026-10-20

Previous goal turn made progress: actual native app/server restore recovery passed
with GET-only traffic/provider absent/writes off. This slice adds GET delete-preflight
for a retained instance. It reuses unchanged lifecycle guards and adds stopped
power, protection, live/unused/Cloud-Init/EFI/TPM and saved snapshot/RAM references,
storage-inventory owner VMID, canonical two-read scope stability and final target/
pending/power checks. Candidate volumes are deduplicated with source references;
recognized ISO CD-ROM media is separate. No raw configs/private inventory fields or
host paths escape. Current definition/association/journal remain unchanged.

Overall result remains unknown if all implemented checks pass: HA/replication
management has not yet been read. Existing flags never enable deletion. No delete,
power, purge, approval or new operation transport is added. ADR 0058 records this
explicit preparation boundary, primary upstream contracts and the need to review
backend cleanup effects/independent volume removal before execution. API-visible
scope is not full deletion authorization. Bounds: 20s, 64 snapshots, 256 images,
1024 inventory entries/storage and existing provider response limits.

Verification: five distinct targeted deletion_preflight generated-HTTPS/API tests
pass, including five owned candidate types and separate ISO, 15-GET positive
sequence, fourteen failed conditions, duplicate inventory/scope rejection, safe
errors, retained association/definition, zero journal and off capability despite
all existing flags on. The last error-fixture test initially failed compilation
on a temporary scope borrow; bound the owned scope string and its filtered rerun
passed. Earlier four passing filters were not repeated without related changes.
Rust formatting/diff checks pass; existing lifecycle/snapshot helpers are unchanged,
so unrelated storage/native/browser/Android and broad suites were not rerun.
No dependency/schema or real provider/VM/credential change occurred.

Next task: read-only HA/replication dependency checks and complete provider-effect
scope, then explicit data-loss deletion approval with durable mutation exclusion,
no-replay dispatch/task/independent absence checks and retained definition/history
for recreation. Actual deletion/recreation and real infrastructure acceptance remain
open; full roadmap remains active.


## Permission-backed deletion HA/replication review — 2026-10-20

Previous goal turn made progress: five read-only target/volume deletion checks
passed. This slice reads HA/replication membership before and after candidate scope,
blocks configured resources/jobs even if disabled or removal-pending, rejects
membership changes and binds normalized target membership into scopeHash. Report
adds target haManaged/replicationJobs without exposing unrelated cluster settings.
No permission, HA or replication configuration is changed.

Verified upstream contracts: HA index requires Sys.Audit on /; replication index
silently filters jobs by VM.Audit on the guest. Therefore require effective target
VM.Audit before/after each replication list. Permission values describe propagation:
0/false with a present key still grants audit. Missing/revoked/malformed visibility
is an error, not empty success. Target path is fixed from retained VMID; no userid
or token override. Validate canonical HA IDs and replication id/guest consistency,
duplicate IDs and 4096-entry bounds. Unknown fields and unrelated entries remain
private. Execution remains off and overall status unknown while remaining provider
cleanup effects are not fully reviewed; this does not grant deletion authorization.

Verification: all five affected deletion_preflight filters passed with the new
23-GET sequence. Three new generated-HTTPS management tests and one full-preflight
management blocker test passed: exact GETs, nonpropagated audit, missing/wrong-path/
revoked/malformed permissions, forbidden read, malformed/duplicate/oversized HA/jobs,
canonical numeric/string guests, disabled/pending jobs, newly appearing dependency
and private data suppression. Nine distinct checks passed. Targeted formatting and
diff review pass. No schema/dependency/native/frontend or existing non-deletion
behavior changed; unrelated suites were not repeated. No real VM/provider access,
credential file inspection, permission or production change occurred.

Next task: complete provider cleanup-effect/candidate scope review (including
unreferenced owned/fleecing artifacts and non-disk VM-specific cleanup), then bind
that explicit scope into durable destructive approval. Definition/association/
history retention, no-replay delete and independent target/volume absence evidence
must precede recreation. Actual deletion/recreation and real infrastructure gates
remain open; full roadmap remains active.


## Fixed deletion effects and recorded cleanup scope — 2026-10-20

Previous goal turn made progress: nine target/volume/permission-backed management
checks passed. This slice binds a fixed versioned deletion effect contract:
configuration/snapshot metadata, VM ACL/firewall, IPAM and recorded fleecing cleanup;
no job purge or bulk unreferenced-disk destruction, and definition/history retained.
Parse current special-section fleecing references and prove owner VMID before any
future write. Unknown sections or malformed/foreign cleanup block. Preserve
non-Cloud-Init CD-ROM media and report extra owned images observed on inspected
storages, without claiming global orphan discovery. Candidate/preserved images
are jointly bounded to 256. Scope v2 hashes effects and preservation/config evidence.

Upstream destroy does not promise removal of every snapshot-only disk reference;
classify it explicitly and block this fixed delete path unless a live/unused/RAM
reference covers its destruction. Moved the earlier positive old-snapshot-only
disk into an explicit refusal case; supported positive uses the live disk shared
by its snapshot. Cloud-Init filename recognition is only drive kind; storage
inventory still proves ownership independently. Fully checked read preconditions
can now pass, while execution is always false and no deletion consent, journal,
transport, power action or provider mutation exists.

Verification: six affected deletion_preflight tests passed with updated candidate/
preservation expectations. Four new effect-scope test groups pass: valid fleecing,
foreign/unsafe/changing cleanup, unknown sections, snapshot-only refusal, media
preservation, unsupported slot and aggregate bounds. After adding the aggregate
bound, reran only its new filter and the affected positive review. Ten distinct
checks passed. Formatting/diff checks pass. No schema/dependency/native/frontend
or non-deletion behavior changed, so unrelated suites were not repeated. Primary
upstream effect/config/drive contracts are linked in ADR 0058. No real infrastructure,
credential inspection/change or VM mutation occurred.

Next task: complete-target/current-digest/scope-hash deletion approval with explicit
loss acknowledgement and durable shared mutation exclusion/no-replay journal;
then fixed DELETE/task tracking and independently verified VM/volume absence while
retaining definition/history for recreation. Real storage cleanup, backend warnings,
platform/deployment/release gates and full roadmap remain open.


## Full-scope deletion approval and no-replay journal — 2026-10-20

Previous goal turn made progress: ten deletion-effect/preflight checks passed.
This slice retains complete target/digest/scope hash and normalized public plan
(volumes/effects/references, snapshots, preserved media/images and fixed effects)
in mandatory dual-confirmed deletion approval. The 64 KiB shape validator rejects
unsafe/unverified/ambiguous/duplicate/overlapping scope or changed backend effects.
A real generated-provider preflight plan is directly checked as valid approval.
Input hash has its own deletion domain; same-key identity/age never renews.

Schema 13 adds vm_delete to common mutation exclusion and retains existing operation
and provisioning child graphs. Active/uncertain provisioning blocks; failed
provisioning is retained but does not prevent explicit cleanup. Migration is atomic,
checks FKs before commit and restores enforcement before exposing the store. Exact
fresh approval/claim precede the irreversible dispatch marker; matching qmdestroy
task is retained once. Unknown submission never reclaims; known tasks only reconcile
with backoff and cannot dispatch again. A failed destroy task remains uncertain
and holds target exclusion because partial deletion is possible.

There is deliberately no success/retirement transition before independent absence
proof is implemented; generic operation completion cannot bypass the guard. Current
association, definition and history remain intact. No HTTP submission, runtime
capability/worker, provider DELETE or client controls are connected yet. ADR 0059
records the plan, failed-provision cleanup exception and partial-failure semantics.
No dependency or real provider/VM/credential change was made.

Verification: eight new deletion_storage tests passed, covering mandatory consent/
full-scope identity, mutated keys, lifetime/clock rollback, dispatch once, mutation
exclusion, reopen/crash windows, no-task uncertainty, known task/no redispatch,
partial task failure retention, concurrent identity/corrupt claim rollback, actual
schema-12 restore/task/child-graph preservation and atomic orphan refusal. Five
initial tests passed first; subsequent filtered migration/concurrency tests passed.

The operation-table rebuild affects existing callers: all 64 targeted lifecycle
(9), provisioning-intent (34), snapshot-storage (10) and restore-storage (11) tests
passed. Affected instance v2 migration and fresh/migrated/reopened FK enforcement
checks passed, including v12/v13. The affected generated-provider public-plan
preflight check passed. Total: eight new plus 67 affected checks, no unrelated
native/browser/Android or full suite. Latest-version assertions changed to 13;
historical migration files are unchanged. Formatting and diff checks pass.

Next task: provider VM/volume absence observation with permission/identity-safe
semantics and exact plan coverage, then atomic completion/retained-association
retirement without deleting definition/history. Follow with fixed one-shot DELETE,
opt-in worker, HTTP/native/UI confirmation and recreation from retained inputs.
Actual deletion, partial-cleanup reconciliation and real infrastructure/platform/
release acceptance remain open; full roadmap remains active.


## Independent deletion absence observation — 2026-10-20

Added a GET-only observer bound to validated retained approval/provider scope.
Explicit cluster-wide VMID availability must return the exact integer before and
after storage reads. Each candidate storage requires effective Datastore.Allocate
before/after its unfiltered-by-VMID image list, because Audit alone can hide images
after VM ACL removal. Permission values are propagation flags, including false/0.
No permissions are changed. Remaining candidate IDs are retained even when owner
VMID changes. Errors never mean absence; malformed/duplicate/wrong-storage entries,
4096-entry inventory bounds and an overall 20-second deadline fail closed.

Verification: three targeted generated-HTTPS tests pass, covering exact five-GET
positive/partial-cleanup traffic, approval hash, owner changes, missing/Audit-only/
revoked/malformed grants, unsafe/duplicate/wrong-storage/oversized inventories,
changed or mistyped VMID, 401/403/404/500, invalid scope and missing consent before
network access. Targeted formatting and diff checks pass. No existing helper,
schema, dependency, native or frontend behavior changed; unrelated tests were not
repeated. Official cluster/content/volume visibility contracts are linked in
docs/deletion.md. No real provider, credential or VM mutation occurred.

Limitations: point-in-time observation only; no preservation/auxiliary-cleanup
proof, success transition, retirement, DELETE, worker or client capability yet.
Next task: atomically retire the retained association after separately verified
task and approved VM/volume absence, preserving definition/history and mutation
exclusion on partial cleanup. Real infrastructure and full roadmap remain open.


## Atomic deletion completion and retained identity retirement — 2026-10-20

Previous goal turn made progress: GET-only VM/candidate-volume absence observation
passed three HTTPS tests. This slice requires a provider-created non-deserializable
completion proof: matching qmdestroy task independently succeeded, then complete
approved VM/candidate-volume absence. Full approval hash/exact linked task and
30-second monotonic freshness prevent rebinding/stale completion. No task status
boolean or caller-supplied empty-volume list can authorize retirement.

Schema 14 retains the whole instance/foreign-key graph and adds deleted status.
An immediate transaction writes succeeded evidence and retires the association
atomically; injected failure rolls both back. YAML/definition/target/history remain.
Deleted identities reject new operations and live target resolution. Registered
scope/VMID uniqueness allows a later new identity on that VMID without letting
old associations read or mutate it. Local/remote native list decoders and both
lists accept/display deleted rows without active VM controls. ADR 0060 records
these consequential schema/status/proof choices.

Verification: four generated-HTTPS/storage completion tests pass (positive, partial/
failed/running task refusal, mismatched approval/task, missing marker, injected
rollback, reopen, retained YAML/history, same-key replay and reused-VMID identity).
The proof expiry/rebinding unit check passes. Actual schema-13 marked-delete upgrade
and atomic orphan refusal pass one new storage test. Shared instance-table rebuild
justified all 82 affected association/lifecycle/provisioning/snapshot/restore/delete
storage tests; all pass, plus the fresh/migrated/reopened FK enforcement check.
Historical migration files are unchanged; latest-version assertions now say 14.

Both client native enums compile; three affected remote workspace HTTPS tests and
one new local HTTP retained-status reader test pass. Frontend build passes. Both
local and remote Firefox simulated-IPC fixtures pass, including retained deleted
rows and no active controls for those identities. These are generated/local
checks, not real infrastructure, mobile or native end-to-end deletion acceptance.
Initial added tests needed ownership/SQLite-type fixes and explicit FK disabling
only to inject a corrupt historical fixture; corrected targeted reruns pass.
Targeted formatting/diff checks pass; unrelated native/browser/Android/full suites
were not repeated. No dependency, credential, real provider/VM or production change.

Limits: absence is point-in-time, not a reservation; preservation/auxiliary cleanup
are not independently certified. No DELETE, runtime capability/worker, HTTP deletion
submission or client deletion confirmation exists yet. Next task: fixed one-shot
DELETE with purge/unreferenced destruction disabled and no-replay executor using
revalidated approval, durable dispatch/task linkage and independent completion.
Then opt-in worker/API/native/UI, recreation and real/platform/release gates remain.
Full roadmap remains active.


## Fixed deletion transport and standalone executor — 2026-10-20

Previous goal turn made progress: schema-14 atomic retirement and retained deleted
identities passed local task/absence/storage/client checks. This slice adds one
fixed retained-target DELETE with query purge=0/destroy-unreferenced-disks=0, empty
body, no skiplock/power/extra-volume operations, verified TLS, bounded response,
redirect refusal and no retry. Returned node/qmdestroy/VMID must match exactly.
Only explicit 401/403 refusal is definite after the durable dispatch marker;
missing/invalid replies remain unknown and non-reclaimable.

The standalone executor requires explicit capability for a fresh dispatch. It
checks retained identity/scope and permission-backed future absence visibility,
then repeats full stopped/protection/pending/lock/owned-volume/snapshot/effect/HA/
replication review. Exact digest and full public plan must match recorded consent;
the original approval expiry is checked before committing the once-only marker.
Known tasks only read, even after writes are disabled. Failed tasks, read failures,
remaining volumes/VM or missing/changed provider stay uncertain and hold exclusion.
A later full absence proof can retire the same association through GET-only recovery.
No runtime flag/worker, HTTP submission or native/client deletion controls are wired.

Verification: server library cargo check passes. Eight new generated-HTTPS tests
pass: exact one DELETE/32-request executor sequence and independent retirement;
18 before-dispatch guards with exact failure codes; unknown/lost/wrong replies
across reopen; known-task restart with dispatch disabled; seven partial/failure
outcomes with retained identity/exclusion and no private error text; later cleanup
reconciliation with the same task and seven GETs; missing/changed provider recovery;
explicit auth/server errors and relative redirect refusal without retry.

One new reconciliation fixture initially omitted its intended remaining-volume
variant and completed immediately; fixed its variant selector, and the targeted
reconciliation rerun now proves uncertainty then GET-only success. The affected
public-preflight positive check passes after extracting its unchanged route builder.
Total nine distinct targeted tests; unrelated storage/native/browser/Android/full
suites were not repeated. Targeted formatting/diff checks pass. No schema/dependency,
credential, production or real provider/VM mutation occurred.

Primary upstream destroy API and HTTP DELETE query parser were reviewed and linked
in ADR 0061. The provider offers no digest compare-and-delete: external actors can
change the target between our review and its mutation. Local operation exclusion
is not an external lock; no atomic infrastructure consent claim or real acceptance
is made. Absence remains point-in-time and auxiliary cleanup/preserved media are
not independently certified.

Next task: independently default-off deletion runtime config/capability and fair
worker integration, with actual isolated server process checks. Follow with exact
HTTP/native/client consent and recovery, recreation, real disposable-target review
and remaining platform/deployment/release gates. Full roadmap remains active.


## Production deletion capability and worker — 2026-10-21

Previous work added the fixed one-shot executor; its interrupted runtime slice
is now verified and complete. VMANYWARE_DELETIONS_ENABLED independently defaults
to false, accepts exact true/false only and fails safely before DB creation for
invalid input. The production service passes it to the router and worker; old
constructors remain default-off. A full sequential round services existing read,
lifecycle, provisioning, snapshot and restore queues before one deletion claim.
Known-task reconciliation remains read-only with writes off. GET preflight reports
the flag separately from prerequisites; no HTTP deletion submission exists yet.

Verification: six new targeted checks pass: strict config/flag independence;
invalid-startup/no-input-echo/no-database; actual worker with absent/false/true flag
and all other write flags on; a round servicing read/lifecycle/snapshot/restore/
delete without short-circuit starvation; actual generated-HTTPS DELETE/retirement
plus forced process death during a running linked task and a second server's
seven-GET completion with writes off; authenticated actual-router GET capability
without operations or mutations. The last two interrupted test results had no
recoverable process handle, so those exact filters were rerun and passed.

Three affected checks pass: existing all-other-flags-on deletion preflight remains
disabled through the legacy router, and both restore startup/production worker
checks pass through the changed main/constructor call chain. Total nine distinct
targeted tests. Rust format/diff checks pass. No schema/dependency/native/frontend
change warranted broader storage/browser/Android or full-suite reruns. Owned
processes, temporary SQLite, generated public CA and provider listeners are cleaned
up by their fixtures. No real infrastructure, personal credential files, permission
changes or real VM mutation was involved.

Next task: confirmed deletion HTTP enqueue with retained path/target/full-plan/
dual-consent/idempotency binding and independent capability checks; then native
commands/UI and GET-only recovery, recreation and real acceptance. ADR 0062's local
provider extension remains part of the requested scope; its proposed hypervisor/
concrete-target choices are unchanged by this independent Proxmox slice. Remaining
platform/deployment/release gates and the full roadmap remain open.


## Full-scope confirmed deletion HTTP enqueue — 2026-10-21

Previous goal turn completed the independent default-off production worker and
actual process crash/restart evidence. This slice adds POST environments/{id}/delete
with the complete retained target/digest/public cleanup plan and mandatory dual
consent. It requires exactly one safe idempotency key, independent capability and
matching provider scope. JSON is bounded to 64 KiB with unknown fields rejected.
The handler makes no provider request: all review/write/absence calls remain in the
worker. Same-key/full-input retries retain original operation/time, including after
retirement. Changed input, competing mutations and new requests for retired
identities conflict. Response 202 includes the provider-independent operation GET
location. Diagnostic route labels stay static, without instance IDs/payloads.

Verification: five new targeted HTTP/SQLite tests pass with an unreachable provider:
full-scope persistence/key identity/unchanged expiry/GET result and retained instance;
independent capability/provider/scope/saved-target/path refusal; consent, unsafe or
unknown effects/fields, JSON syntax/shape/content-type/size; duplicate/unsafe/missing
keys and auth before journaling; changed scope/media/snapshot/references and cross-
mutation exclusion. Added the content-type case and reran only its focused filter.

One actual authenticated server/generated-HTTPS-provider test passes HTTP ->
journal -> worker -> single DELETE -> independent retirement. It verifies same-key
retries before/after completion, exact approval hash/original confirmation time,
known-ID GET recovery, one journal and precisely the executor's 30 requests (no
handler provider reads). Auth refusal adds no journal/provider traffic. Six distinct
new targeted tests pass. Formatting/diff checks pass. No dependency/schema/native/
frontend or shared existing behavior changed, so unrelated suites were not rerun.
Owned process/temp SQLite/public CA/provider listeners are cleaned by fixtures.
No real provider/VM, personal credential files or production changes occurred.

Limits: HTTP source/runtime acceptance is locally proven, not a native user journey
or real backend cleanup. POST replay needs current capability/provider; known-ID
GET reads do not. Scoped deletion recovery, unknown-ID history, client consent,
recreation and infrastructure/platform/release acceptance remain open. ADR 0062's
local provider extension remains pending its proposed choices/concrete test target.
Next task: retained native deletion prepare/review, mandatory dual consent, same-key
submission retry and exact known-operation status validation; then client UI and
scoped GET-only recovery. Full roadmap remains active.


## Retained native full-scope deletion consent — 2026-10-21

Previous goal turn proved confirmed HTTP enqueue and actual server/worker deletion
against a generated provider. This slice registers four native prepare/confirm/
read/clear commands. Preparation binds a registered saved instance and fixed GET
preflight to the current connection/revision. Typed bounded volume/reference/
preservation/effect/check validation prevents incomplete passed reports and unsafe
scope from becoming eligible. Confirmation takes the retained key and both explicit
consents only; IPC cannot replace the plan/target. First submission expires after
120 monotonic seconds. Unknown replies retain key/body and prohibit replacement;
known IDs only read. Partial failures stay uncertain; native success requires the
exact full approval hash and all absence/retirement/definition evidence. Disconnect
clears local memory without cancelling server work; late prepare/submit/read replies
cannot revive it. ADR 0063 records these decisions.

A direct sha2 0.10 dependency now supplies the portable full-input hash use case.
Cargo.lock changes only the native package's dependency edge; no new resolved
package/version was introduced. Shared synthetic approval JSON/hash fixtures cover
live/snapshot/fleecing/preserved scope and are independently checked by server/native
code. Server production behavior and schema are unchanged; its obsolete module
comment is corrected to describe the existing review/consent/evidence contracts.

Verification: fourteen new native checks pass: full hash/JSON interoperability,
fixed review/submit/read paths and complete body, mandatory dual consent/key,
unknown identical retry, disabled/failed/expired review refusal, sixteen unsafe/
inconsistent review variants, wrong scope/target/deleted identity, thirteen invalid
success variants plus partial-task status consistency, stale preparation,
disconnect in flight, auth/status/response-size bounds, actual socket loss,
GET-only known partial recovery, late disconnected read and invalid local inputs.
Initial two checks passed, then eight added checks passed; four final filters pass.
After making generation/revision validation atomic, reran only its positive/stale
preparation filters. The server golden-contract filter passes.

Native state/handler registration and the direct dependency edge warranted five
affected HTTPS checks: three workspace reads/disconnect tests and retained restore
positive/disconnect tests; all pass. Total 15 new plus five affected checks. Linux
native crate compiles/links with .lab/native-deps's verified package prefix.
Formatting/diff checks pass. No GUI/native app launch, browser/Android/real provider
or unrelated full suite was run or claimed. Generated listeners/requests are owned
fixtures; no personal credential file, infrastructure or real VM mutation occurred.

Next task: explicit remote-client deletion review, visible reviewed/preserved scope,
fixed destructive effects, dual consent, same-key retry and read-only status/clear
controls with stale-result guards. Then scoped GET-only deletion recovery, actual
native/mobile journey, recreation and real/platform/release gates. ADR 0062's local
provider extension remains in scope with its proposed choices/concrete target open.
Full roadmap remains active.


## Explicit client deletion review and status — 2026-10-21

Previous goal turn compiled/verified retained native deletion commands. This slice
adds remote registered-row selection and a dedicated React deletion form. Selection
alone makes no preparation or submission. Native review displays the node/VMID,
reviewed volume references/fleecing/snapshots, preserved media/observed extra images,
fixed cleanup effects and retained definition/history. Two independent consents
start unchecked; disabled/failed/stopped-target/unknown-effects guards prevent
submission. Native consent is never replaced by a renderer-supplied plan.

Unknown responses freeze consent and retain the key for explicit identical retry.
Known IDs have read-only refresh, no further POST; partial cleanup remains uncertain
and cannot be acknowledged away. Success additionally checks target/task/absence/
retirement flags from native-validated result evidence. Old-identity mutation
controls disappear after retirement; acknowledgement does not enable it again.
Shared snapshot/restore/deletion selectors remain locked while work is retained.
Disconnect remains available, clears local consent and suppresses late prepare/read
replies without cancelling server work. No automatic localStorage consent/key restore.

Verification: production frontend build passes. The new Firefox simulated-IPC
scenario passes disabled/running/retargeted/unknown-effect refusal, scope/effect/
preservation presentation, dual unchecked consent, selector exclusion, identical
unknown retry, known-ID reads, failed-task uncertainty, invalid success refusal,
retired controls, acknowledgement and late prepare/read after reconnect. Affected
snapshot-create scenario passes. A parallel restore run timed out before selection;
its exact sequential rerun passes, without a proven cause for the first failure.
Run these browser modes sequentially; no concurrency-safety claim is made.

Three distinct browser scenarios and the build are verified. JS syntax/diff review
pass. No server/native/dependency/schema change justified repeating those suites or
Android/full checks. Browser fixtures simulate IPC and contact no provider/VM; their
owned browser/server/temp data are cleaned. Actual native GUI/mobile launch,
provider cleanup, scoped saved-deletion recovery and real acceptance remain open.

Next task: instance-scoped read-only deletion history/detail from the durable journal,
including retained deleted associations and strict saved task/full-plan/result
consistency, without provider access or runtime write capability. Then native/UI
saved-ID recovery, actual native journey, recreation and infrastructure/platform/
release gates. ADR 0062 local-provider scope/choices remain unchanged and open.
Full roadmap remains active.


## Instance-scoped read-only deletion recovery — 2026-10-21

Previous goal turn implemented/tested explicit React deletion scope/dual consent/
status. This slice adds GET deletion-operations history/detail for retained IDs,
including deleted associations, independently of provider/write configuration.
Readers resolve instance/kind before decoding, validate full saved approval/hash/
target/qmdestroy/status/result consistency, and require retired association plus
exact task/absence/definition evidence for success. Known failed destroy tasks stay
uncertain; historical failed/interrupted attempts remain readable after a later
retirement. New identities on a reused VMID never inherit the old history.

Reads are transactional SELECTs only: no claim, enqueue, reconciliation, consent
renewal or state change. Check persisted approval/result sizes before parsing
(64 KiB/4 KiB), bound timestamp fields and reject extra result fields. History
filters before its descending time/ID limit of 100 and returns compact summaries;
detail returns saved target/digest/complete plan/hash/submission context, without
consent flags, key or raw UPID. Saved digest/plan are for historical full-hash
verification, not current target observation or new consent. ADR 0064 records this.

Verification: eight targeted HTTP/SQLite tests pass: queued reopen with provider/
flags absent and no forwarded consent/key; unchanged queued/running/unknown/known/
partial-task-failure/interrupted/failed/refused states; cross-instance/kind privacy
before corrupt decode; filtered 100-entry order and older detail; auth/empty/missing
scope; 16 saved-result/task/hash/retirement/size consistency cases; old failed/deleted
history alongside replacement VMID; and saved-target mismatch. Corruption tests
verify operation rows and SQLite data version unchanged. Synthetic successful
encodings test reader consistency only and are not provider-completion evidence.

The affected actual authenticated server/generated-provider deletion scenario now
also kills its completed first server and starts a new process on the same DB with
all writes default-off and no provider settings. The retired association's scoped
history/detail return the exact unchanged succeeded operation/full plan/hash and
add no provider requests. The complete scenario passes. Nine distinct targeted
checks and library compilation pass; formatting/diff checks pass. No new schema/
dependency/native/frontend behavior justified unrelated suite reruns. Owned
processes/listeners/temp SQLite/public CA are cleaned by fixtures. No personal
credential files, real infrastructure or real VM mutations were involved.

Next task: native fixed GET saved-deletion history/detail commands with independent
saved-context/full-plan/hash/status validation, without preparing consent; then
client history/manual-ID UI, actual native restart journey, recreation and real/
platform/release gates. Local provider scope in ADR 0062 remains in the full roadmap
with its proposed choices/concrete target unresolved. Goal remains active.


## Native fixed GET saved-deletion recovery — 2026-10-21

Previous goal turn proved scoped server recovery without provider/write flags,
including actual server restart. This slice registers native deletion history and
saved-detail GET commands with canonical IDs and no prepared intent. Saved detail
validates target/digest/complete plan/full approval hash and exact operation/task/
absence/retirement evidence, plus recorded dispatch consistency. Reuse of the native
result validator keeps submission/status/recovery aligned. History validates <=100
unique IDs, hashes, timestamps and permitted status/failure combinations. Known
failed destroy tasks remain uncertain; terminal failed known-task records are refused.

Typed approval reconstruction is internal deterministic hash verification only:
it never installs native prepared state, returns consent flags/key or sends POST.
Saved-context DTOs remain separate from live preflight/review. Reads check connection
generation around validation, suppress old disconnected/reconnected replies, drop
extraneous fields and return safe read-only errors without raw details or mutation
retry guidance. No new schema or dependency is needed. ADR 0064 records this extension.

Verification: eight new native checks pass: fixed authenticated GETs without intent;
16 changed target/plan/hash/task/absence cases; 13 recorded status/dispatch/failure
combinations; seven wrong/duplicate/oversized/inconsistent history cases; stale
history/detail across disconnect/reconnect; sanitized extra fields/auth/status/
response-size errors; invalid IDs before profile/network; and empty/partial history
plus saved-detail reads without consent. Initial test compilation needed a test-only
private-state getter and helper-name fix; corrected targeted run passes.

The shared result/hash validation changed, so all 14 affected native deletion
review/submit/status tests were rerun and pass. A mistyped intermediate filter
selected zero tests and is not counted; the corrected module filter ran all 14.
After final generation guards, only positive and stale-reader filters were repeated;
both pass. Total eight new plus 14 affected checks. Linux native crate compiles/links
using the existing verified .lab/native-deps prefix. Targeted format/diff checks
pass. No frontend/server/dependency/schema behavior justified unrelated suites,
browser/Android/full checks or GUI launches. Fixtures use owned generated HTTPS
peers and synthetic tokens only; no personal credentials, infrastructure or real VM
mutation occurred.

Next task: explicit client saved-deletion history/manual-ID/refresh panel, available
for registered and retired associations, with saved-versus-current wording, complete
scope/evidence and stale-result guards, and no mutation command calls. Follow with
actual native process/server restart journey, recreation, real disposable-target
acceptance and remaining local-provider/platform/deployment/release gates. Full
roadmap remains active; no real or cross-platform completion is claimed.


## Client saved-deletion history/manual-ID recovery — 2026-10-21

Previous goal turn compiled/verified native read-only saved-deletion readers.
This slice adds a separate remote React panel for every retained association,
including deleted rows. History, row/manual-ID inspection and refresh are explicit
GET-reader commands only; no prepare/confirm/clear or consent restoration occurs.
Saved cleanup volume references/effects/snapshots and excluded media/other images,
recorded submission/task status and result evidence are visible. Wording separates
historical absence from current VM/storage state and possible VMID reuse. Target/
request/full result hash/task/absence/retirement checks prevent wrong success display.
Partial and old failed records remain readable. Empty history differs from errors.
Editing an ID or disconnecting invalidates pending replies; fresh connections have
no restored ID/result/consent. Retiring a row does not remove the recovery panel.

Verification: frontend production build passes. New Firefox simulated-IPC scenario
passes explicit registered/retired history and empty states, row/manual-ID reads,
saved reviewed/preserved/fixed-effect presentation, historical/current distinction,
partial/failed outcomes, wrong target/result hash refusal, error/empty separation,
stale edited-ID detail, disconnected history error and fresh-connection lookup.
It explicitly asserts no mutation-intent commands. After adding fixed-effect copy,
reran only its affected scenario and the build; both pass.

The shared row mounting changed, so existing deletion submission and saved-restore
browser scenarios were run sequentially; both pass. Total three distinct browser
scenarios plus frontend build, JS syntax and diff review. No native/server/schema/
dependency changes warranted those suites, Android or unrelated full checks.
Browser IPC is simulated: no native application, provider or real VM was launched.
Fixture-owned profiles/processes/listeners/temp data are cleaned by the runner.

Next task: actual isolated Linux WebView -> production native deletion commands ->
authenticated server -> generated HTTPS provider, then new application/server
processes recovering the unchanged succeeded record with provider removed and
writes off. Keep real infrastructure/platform/mobile acceptance separate. Recreation,
local-provider choices/implementation and remaining deployment/release gates stay
in the full roadmap. Goal remains active.


## 2026-10-21 — Actual native deletion and restart recovery

Scope: extend only the owned Linux native fixture with confirmed deletion and
fresh app/server saved-result recovery. Acceptance passed against generated HTTPS
infrastructure, not a real VM. Production UI/native/server commands are used.

- Running-target refusal, cancel/re-review and independent target/data-loss consent
  pass through actual WebKit/Tauri IPC.
- The authenticated proxy deliberately loses the first accepted API response;
  two identical path/key/body submissions produce exactly one fixed provider
  DELETE with purge and bulk unreferenced-disk removal disabled.
- Task and VM/reviewed-volume absence evidence succeed; the association becomes
  deleted while the complete retained definition is unchanged.
- A second app/server process recovers history, a copied operation ID and refresh
  using GET only. All five write flags are false, Proxmox configuration is absent,
  provider counters are unchanged and the complete saved operation is unchanged.
- Embedded native example build, JS/Python syntax and Rust format checks pass.
  The affected restore/restart scenario is checked because the launcher is shared.
  Logs: `.lab/native-deps/deletion-recovery-ui-check.log` and
  `.lab/native-deps/restore-recovery-ui-check.log`.

No real VM operation, credential access or dependency change. Real cleanup,
packaged/mobile validation and full roadmap acceptance remain open. Next slice:
read-only recreation planning from a retained definition; a fresh association must
have a new identity and independently reviewed target/creation consent.


## 2026-10-21 — Retained-definition recreation preview

Scope: read-only `POST /api/v1/environments/{id}/recreate-plan`, accepting only a
bounded explicit target VMID. ADR 0065 records source identity and saved-definition
semantics. The source must be deleted; its archived YAML must match its recorded
hash. Current catalog planning is reused, without provider access or mutation.

The response binds source ID/hash/revision and exact YAML, explicitly distinguishes
fresh provisioning from old disk restoration and requires a new instance identity.
Recreation/creation capabilities and live verification/reservation remain false.
The prior VMID can be proposed but is not claimed available. No operation or new
association is created, and repeated previews leave the source unchanged.

Verification: targeted `cargo test --locked -p vmanyware-server --test recreation`
covers deterministic saved-YAML planning without a provider, unchanged source and
empty operation journal, active/unknown/malformed source refusal, invalid target,
source-template VMID refusal, extra/replacement-YAML fields, missing catalog,
Origin rejection, body limit and corrupted archived-definition hash refusal.
All three focused tests pass. The initial corrupt-record fixture attempted to
edit an already retired row and was correctly blocked by the existing retention
trigger; it now installs the corrupt fixture before retirement. Source format/diff
review is limited to this slice. No unrelated suites repeated.

Limitations: planning alone does not recreate a VM. Live availability/preconditions,
durable source lineage, fresh consent/idempotency, native/client flow and real
infrastructure acceptance remain open. Next: durable recreation source lineage
and confirmed submission through existing provisioning execution, allocating a
new instance ID while retaining the deleted source and its history.


## 2026-10-21 — Durable recreation source and fresh identity

Scope: schema-15 immutable recreation lineage and an atomic storage entry point,
reusing the ordinary provisioning journal/stages/executor. No HTTP mutation or
provider transport was added. ADR 0066 records key/source and identity semantics.

`enqueue_recreation` requires a deleted canonical source, explicit fresh approval
and request key. It reads exact saved YAML/name/hash/revision and validates the
current catalog plan before inserting a new association, operation/stages and
source reference in one transaction. Same-key replay requires the same source
and approval; normal creation/recreation cannot replay each other's keys. Target
conflicts roll back without partial rows. Source/target definition identity and
retired/distinct source status are guarded on insertion and job loading. SQL
update/delete guards retain lineage. The existing worker can claim the ordinary
provisioning job with its recorded source; no old task/consent is reused.

Three focused storage tests pass: reopen/replay, distinct identity with prior VMID,
unchanged source/YAML, lineage and ordinary-key conflicts, SQL immutability,
worker claim, invalid/active/missing source, unconfirmed/mismatched approval and
rollback. Because the schema and shared enqueue/read path changed, verification
also covers the affected provisioning and migration/storage tests: 90 tests pass
in total (3 recreation storage, 3 preview, 34 provisioning intent, 10 deletion,
10 instances, 9 lifecycle, 11 restore and 10 snapshot storage). An explicit
schema-14-to-15 upgrade preserves an ordinary provisioning job and foreign keys.
Historical fixtures initially retained schema-15 triggers while rebuilding old
operation tables; those fixtures now remove the new objects before downgrade.
Production migration/retention guards remain enabled. Format/diff checks pass.

Limitations: confirmed HTTP/native/client submission and saved-source recovery
presentation remain pending; no real VM or generated provider was provisioned in
this slice. Next: confirmed HTTP recreation enqueue and source-scoped GET recovery
with current runtime/provider/catalog validation and the existing executor.


## 2026-10-21 — Confirmed recreation HTTP enqueue and scoped saved detail

Scope: `POST /api/v1/environments/{id}/recreate` and source-scoped saved operation
GET, using existing schema-15 lineage and provisioning worker contracts. The body
binds source ID to path and a fresh full provisioning approval. Exactly one request
key, no Origin, explicit provisioning enablement, current provider scope/catalog,
retired archived YAML/hash/plan match and supported minimal workflow are required.
The handler performs no provider request; it only atomically queues the existing
provisioning journal. 202 Location points to the source-scoped GET detail.

GET reads the validated saved job/lineage, returning source ID, ordinary operation,
retained plan and explicit fresh-identity/no-old-disk semantics. It exposes no
request key, approval consent or raw task. Wrong-source and unrelated operations
are hidden. Reopening the store with no provider/catalog or write capabilities
returns unchanged saved detail without claiming or dispatching work.

Four focused API tests cover same-key identity/consent-age retention, new instance,
wrong-source refusal, unchanged archive and no partial records, restart/read-only
recovery, runtime/provider/scope/catalog gates, source-path/plan/consent validation,
Origin/duplicate/missing keys, active/missing source, auth/body bounds and minimal
workflow refusal. Ordinary create tests are selected because the existing error
mapping now serves both routes; preview tests cover adjacent routing. All 12
selected tests pass (4 recreation API, 3 preview, 5 ordinary create), with
source format and diff checks passing. No schema,
dependency or provider transport changed in this slice.

Limits: HTTP enqueue is not actual recreation/readiness. The read-only preview
still grants no recreation capability. Dedicated live preflight, history discovery,
generated-provider worker completion, native/client journey and real VM acceptance
remain open. Next: generated HTTPS recreation enqueue-to-worker/result evidence
with source preservation and a fresh target identity, reusing existing execution.


## 2026-10-21 — Recreation API through production worker/generated HTTPS

Scope: extend the existing owned creation integration fixture with recreation,
sharing setup/worker assertions rather than copying the provider implementation.
No product source, dependency or database change. A retired source is constructed
in the fixture; no deletion or real VM is performed.

The actual recreation router queues a fresh identity from the archived definition.
The production worker executes clone, resource/optional-disk configuration,
Cloud-Init/seed regeneration, start and clean simulated guest/IP readiness over
verified generated HTTPS. The clone marker must contain the new identity, never
the source. Source YAML/hash/revision and current definition remain exact. Both
8 GiB/no expansion and 40 GiB/one expansion complete with all stages succeeded.
Provider request authentication, exact mutation counts and no unexpected mutation
are asserted. A separate inventory operation still completes fairly.

SQLite reopens after clone and after completion preserve journal progress and
lineage. Completed HTTP replay returns the same operation and the worker remains
idle with no provider traffic. Source-scoped GET recovery without provider/catalog
or write flags returns the unchanged succeeded operation/source and does not
contact the provider. Exactly one new instance and provisioning operation remain.

Verification: focused `cargo test --locked -p vmanyware-server --lib
proxmox::tests::create_flow` covers recreation plus the affected ordinary creation
helper, each with the two disk-size cases. Format and diff review cover only this
slice. Evidence is in-process Rust router/worker plus real local TLS transport,
not actual server-process/native/mobile or real Proxmox/guest acceptance.

Next: dedicated source-bound live recreation preflight so the native client can
review exact saved definition/current target checks before fresh confirmation.
Native/client review, saved-history discovery and real infrastructure gates remain
open; the full roadmap is not complete.


## 2026-10-21 — Source-bound live recreation preflight

Scope: `POST /api/v1/environments/{id}/recreate-preflight`, accepting only bounded
targetVmid, reusing exact retired-source/YAML/hash and current-catalog validation
from the offline preview. The existing four-GET provisioning preflight observes
template/storage/network/available VMID and returns matching-plan-hash checks.
No operation, association, reservation or provider mutation is created. Offline
preview keeps its prior behavior and cannot grant execution capability.

The live response binds source ID/hash/revision and saved YAML to the current
endpoint-scoped plan. Runtime/minimal-workflow capability flags remain distinct
from check results; an occupied target fails a check even if capability is on.
Denied reads are unknown and redacted. Template revision, write permissions and
actual guest readiness remain unknown. The worker still revalidates before write.

Verification: three focused generated-HTTPS tests pass, covering both runtime
flags, exact source/plan binding, four GETs only, unchanged source/empty journal,
busy VMID and redacted denied reads. Four HTTP preview/preflight tests and four
affected recreation enqueue/recovery tests cover routing/shared source validation.
No dependency/schema/provider transport change. Format/diff review is scoped here.

Limits: current source template/permission/real guest acceptance remains open;
this is read-only review, not actual allocation or consent. Next: native retained
recreation prepare/confirm/read/clear with exact source/plan and new-instance
validation, followed by client review and isolated native journey.


## 2026-10-21 — Native retained recreation review

Scope: read-only native prepare/clear commands, registered in the production
Tauri handler. ADR 0067 records exact source/plan/check and cancellation semantics.
Preparation GETs the selected retired instance, then POSTs targetVmid only to its
fixed preflight path, through current authenticated verified HTTPS. It never sends
an approval or replacement YAML. Existing ordinary creation plan validation and
bounded authenticated response reading are reused through module-scoped helpers.

The native review binds saved ID/YAML/hash/revision to the returned supported plan,
requested VMID and preflight hash. Fresh identity/no-old-disk semantics, ten unique
check codes/statuses, coherent capability and no-reservation flags are checked.
Disabled/failed/unknown results remain displayable without granting confirmation.
Server check messages/unknown fields are discarded. One preparation read runs at
a time; clear requires the retained key and increments the review epoch. Disconnect
clears state and late replies cannot install a review on another connection.

Verification: five generated-HTTPS native tests pass (fixed paths/auth/body,
retain/clear, changed source/plan/check/semantic fields, disabled/failure display,
invalid/active source, missing auth marker, disconnect). Native crate compiles and
links with `.lab/native-deps`. A test-fixture call initially used the wrong gate
signature and was corrected before these passes. The affected ordinary creation
prepare test is selected because its shared plan/response helpers are exposed to
the new sibling module. Format/diff checks are scoped to this slice.

Limits: no confirmation/status command or client button yet; native compilation
and local HTTPS tests are not a WebView/mobile or real infrastructure proof.
Next: native fresh confirmed submission with retained-review eligibility, same-key
unknown-response retry and source-scoped GET status/new-instance validation.


## 2026-10-21 — Native fresh recreation confirmation and status

Scope: production native confirm/status commands using the retained recreation
review, plus submitted-state clear/replace guards. Existing ordinary creation
approval reconstruction and readiness validator are reused via sibling helpers.
No client UI, dependency or schema change. ADR 0067 records exact state semantics.

Fresh consent requires enabled capability, first seven metadata checks passed,
no explicitly failed checks and review age at most 120 seconds. Native constructs
source/full approval from retained data and marks submitted before dispatch. Errors
or invalid responses keep the original key/body; an explicit identical retry is
possible even after expiry. Once an operation ID is known, only scoped GET status
is offered; IDs must keep a distinct new instance and the same source/plan/result
scope. Success must include clean guest readiness. Active/unknown requests cannot
be cleared/replaced, in-flight submissions cannot duplicate, and disconnect drops
late replies without cancelling server work. Pending unsubmitted review reads can
still be cancelled with epoch invalidation.

Server saved recreation detail now sets the returned plan scope from its durable
approval, needed to bind native recovery when current provider config is absent.
Four focused API tests and two production-worker/generated-HTTPS journeys pass,
including restart/saved scope and both disk-size variants of create/recreate.
Twelve native focused tests pass across the retained workflow; expiry/additional
failed-check gates are specifically rerun after tightening eligibility. The
existing ordinary creation readiness validator test covers its shared contract.
Native Linux crate builds/links using `.lab/native-deps`; format/diff review is
scoped here. No unrelated suites repeated.

Limits: these are native command HTTPS tests, not actual GUI/mobile or real VM
acceptance. Stateless saved-source recovery/history and full client UX remain open.
Next: React deleted-row recreation target/source review, explicit confirmation,
unknown same-key retry, GET status and handoff to the new instance/SSH flow.


## 2026-10-21 — Client recreation review, confirmation and new-guest handoff

Scope: deleted-row selection and a single retained recreation form. No mutation is
caused by selecting a source. An explicit target VMID is required before native
review. The UI displays source identity/YAML/hash/revision and current template,
resources/storage/network/guest user/key count, with check/capability and fresh-
disk/no-old-snapshot semantics. Source/hash/target/preflight mismatch is refused.

Consent starts unchecked. Known metadata checks must pass with no explicit failed
check; disabled/occupied/unknown prerequisites cannot submit. Cancel/review resets
consent. Shared mutation selectors and ordinary creation are locked during review
or submission. Unknown replies offer only identical-key confirmed retry, while
known operations offer GET status. Uncertain work remains retained; false readiness
or reuse of the old instance cannot display success/SSH. Matching completion hands
the new guest to existing SSH host-trust setup without accepting its key. Terminal
acknowledgement and reconnect drop old review/consent. Generation guards suppress
late replies. No credential, key or consent is automatically persisted.

Verification: TypeScript/production frontend build passes. Focused Firefox
`--recreation` passes actual React controls with simulated IPC, including disabled/
occupied/changed-source refusals, exact retry, status/partial work, false success,
new identity, SSH trust handoff, acknowledgement and stale reply suppression.
Affected `--deletion` and `--snapshot-restore` modes cover shared selector/locking
changes; runs are sequential. No Rust/server/provider code changed in this slice.
JS syntax and diff review cover this slice rather than unrelated suites.

Limits: browser simulated IPC is not native command/WebView integration, deployment,
fresh mobile or real VM evidence. Stateless saved-source recreation history and
full roadmap gates remain open. Next: actual isolated Linux WebView -> production
native -> authenticated server/worker -> generated HTTPS recreation journey.


## 2026-10-21 — Actual native retained-definition recreation

Scope: extend the owned native lifecycle fixture with `--recreation`; no product
behavior, dependency or schema change. The example registers production recreation
commands and embeds the real UI driver. A small Node adapter reuses the existing
Android generated HTTPS provider rather than copying its protocol implementation.
Source retirement is fixture setup only; this mode performs no deletion or real VM
operation. It uses a public demonstration key, not a personal/private identity.

Actual React/WebKit -> Tauri IPC -> verified HTTPS proxy -> authenticated Axum
process/production worker -> generated HTTPS provider passes. The UI reviews the
retired source and target, cancels/reviews with unchecked consent, explicitly
confirms, encounters a deliberately lost accepted HTTP response, retries identical
path/key/body, reads the scoped operation to guest-ready success, and prepares SSH
host trust for the new guest. No SSH session/key acceptance is claimed.

Independent assertions require one succeeded provisioning operation/all stages,
a new distinct registered instance, unchanged source YAML/hash/definition and
immutable source lineage. The provider requires the expected resources/key/seed/
start order and exactly six mutations, without clone/seed/start replay. Both app
and server exit successfully; the generated provider report/input/listeners and
all owned processes are closed. TLS, WebKit sandbox and X11 authentication remain.

Verification: current server binary and embedded native example build; the actual
`--recreation` journey passes, log `.lab/native-deps/recreation-ui-check.log`.
Initial fixture failure was a demo-key comment mismatch (production strips SSH
comments correctly); the adapter now expects its canonical public form. A report
pipe cleanup timeout was fixed by explicitly closing/destroying the owned stdin.
No product workaround or verification bypass. Shared deletion/restart fixture
also passes because the launcher/remote bootstrap/proxy condition changed.
JS/Python syntax, Rust format and diff review pass for this slice. The existing
module declaration order was normalized after format review; no behavior changed.

Limits: no real Proxmox/template/guest/SSH, normal packaged/mobile startup or live
provider crash/restart proof. Separate worker tests cover SQLite reopen; native
saved-source history/recovery remains open. Next: source-scoped saved recreation
history discovery/recovery without provider or write capabilities.


## 2026-10-21 — Bounded saved-source recreation discovery

Scope: server GET history for one retired source, alongside the existing scoped
saved detail. ADR 0068 records discovery and historical-evidence semantics. No
schema, dependency or provisioning-execution behavior changes.

The reader selects exactly the latest 100 source-linked operations, creation time
and ID descending, and validates each through the existing provisioning/source
reader after bounding saved approval/plan/catalog/definition/result payloads.
It returns only source ID and operation/new-instance/status/time summaries. Other
sources and ordinary creation are excluded. Active/missing/invalid sources are
404, valid empty history is explicit, and corrupt selected journals fail safely
rather than silently disappearing. No provider/catalog/write capability is needed;
GET never claims, dispatches, renews consent or changes source/journal state.

Verification: seven focused recreation HTTP tests pass (three new history cases,
four affected enqueue/detail cases), including exact newest-100 over 103 source
jobs, unrelated ordinary creation, reopen without provider/writes, source isolation,
empty versus missing, active source/auth refusal, unchanged operation/source and
queued dispatch state, corrupt approval hash and oversized plan rejection. Tests
use isolated local SQLite only, no credentials or provider. Format/diff review is
limited to this slice; unrelated native/browser/worker suites were not repeated.

Limits: this is server discovery, not native/client reconnection acceptance or a
current VM/guest read. Next: stateless fixed native history/detail GET readers,
without preparing/restoring consent, before client lookup and native restart proof.


## 2026-10-21 — Stateless native recreation history/detail recovery

Scope: two fixed GET commands registered in the production native bridge,
without restoring prepared review/key/consent. History uses the exact source path;
detail validates the selected retired source/archive first, then its scoped saved
operation. Current authenticated verified HTTPS and bounded response reads apply.

History rejects wrong source, duplicate or reused source/new-instance identities,
unsupported statuses, oversized lists, invalid timestamps and wrong ordering.
Saved detail binds source/archive definition identity to plan and operation ID,
requires a distinct new instance and fresh-identity/no-old-disk semantics, and
validates recorded success with the existing guest/scope/plan evidence checks.
Nonterminal/failed records and disabled execution remain readable. Unknown raw
key/consent fields are dropped. Late replies after disconnect cannot be accepted;
no recovery call installs any review, request key or submission state.

Verification: eight focused native generated-HTTPS recovery/history cases pass,
plus twelve affected retained recreation workflow tests and the shared ordinary
creation readiness validation test. The initial tests attempted direct access to
private sibling state; a test-only empty-state observer now verifies no recovery
state is installed without exposing mutable production state. Native Linux crate
compiles/links using `.lab/native-deps`. Format/diff review is scoped here; no
unrelated server/browser/worker checks repeated. No dependency/schema change.

Limits: native HTTP tests are not actual WebView/client recovery or server restart
acceptance, and do not observe present VM/guest/SSH readiness. Next: explicit React
saved-source history, manual operation-ID lookup and read-only refresh with clear
historical wording; then actual app/server restart recovery without provider/writes.


## 2026-10-21 — First local Linux server delivery artifact

Scope: actual release server archive, service/config/template files, copy-only
staging installer and operational documentation. ADR 0069 records boundaries.
No runtime crate, server behavior, real credential or provider mutation changed.

`cargo build --locked --release -p vmanyware-server` passes. Allowlisted packager
produces `.lab/packages/vmanyware-server-0.1.0-linux-x86_64.tar.gz` plus SHA-256
companion. The archive contains the optimized ELF, systemd/sysusers configuration,
root-only environment example with all writes false, template example, install
script, licenses/build metadata/manifest and INSTALL.md. It contains no local
state or credentials. Build-host architecture/libc is recorded; no portable Linux
baseline, signature or release-candidate claim is made.

The installer only copies files to an explicit root. It does not create users,
replace actual server.env, change database state, create credentials, enable/start
a service or install/configure TLS. Docs cover reviewed host installation,
loopback/authenticated startup, HTTPS proxy/client connection, provider/catalog
preparation, stopped backups/upgrades/rollback, troubleshooting and retaining data
on uninstall. systemd/Caddy examples were checked against official documentation.

The targeted package check passes outer/inner checksums, fresh staging/reinstall
with existing config/state intact, file permissions, systemd-analyze syntax using
staged ExecStart, shell preflight refusal without a token, actual packaged process
health/authentication and clean termination, exact saved definition after restart,
redacted logs and invalid-token rejection before DB creation. All processes and
state are temporary/owned; no real unit/account/firewall/provider changed.
Syntax/diff checks apply only to this slice. Application/native/browser suites
were not rerun because no application behavior changed.

Limitations: actual systemd-manager/sandbox and HTTPS deployment on a selected host
remain open, so full service installation acceptance is not marked complete. Mobile
artifact distribution, real template/ACL/VM/SSH and other-platform/release gates
remain open. Next: rebuild/package a testable Android client artifact for the
mobile allocation/SSH milestone; recreation-history UI remains queued.


## 2026-10-21 — Current Android test APK delivery

Scope: restore an owned build toolchain, build current ARM64/x86_64 debug APKs and
verify/copy manually installable artifacts. ADR 0070 records exact-artifact versus
historical runtime evidence. No application behavior/dependency/version or tracked
Android project configuration changed.

The old `/tmp` SDK/JDK prefix was absent. Official HTTPS JDK 21.0.2 and SDK package
archives were downloaded/verified into the owned persistent prefix
`/home/moha/.local/share/vmanyware/android-build-20261021`. SDK 36/build-tools 36.0.0,
NDK 28.2.13676358 and platform tools are present; NDK symlinks restored correctly.
Public download URLs/checksums are in that prefix's verified-downloads.json. SDK/
signing/Gradle caches are outside the repo, ANDROID_USER_HOME is owned and no
personal key/HOME override was used. Reuse paths in `.lab/android-toolchain/env.sh`.
Build log: `.lab/android-toolchain/apk-build.log`. Owned Gradle daemon was stopped.

The normal Tauri Android build completed fresh TypeScript/Vite embedding and both
Rust/JNI targets, then produced both single-ABI APKs. `scripts/package-android.py`
verifies each signature, package dev.vmanyware.client, version 0.1.0, debuggable
flag, API 24/36 declarations and expected ABI. The actual embedded native library
must equal its current target-build hash. Public metadata includes signer/APK/
native hashes; no private signing material is copied. Both use the same owned
development signer. A new aapt2 output label (`minSdkVersion`) was accommodated
without changing the required API value.

Deliverables: `.lab/packages/android/vmanyware-0.1.0-android-arm64-v8a-debug.apk`
(243784580 bytes), x86_64 variant (247953121 bytes), artifacts.json, SHA256SUMS and
INSTALL.md. Both copied APK checksums pass. `make package-android` reproduces the
build/package with explicit owned tool paths; script syntax/diff checks are scoped
to this slice. No unrelated application/server/native/browser suites repeated.

Limits: these are debug-symbol-bearing development artifacts, not signed release
or fresh runtime/physical-device proof. Historical emulator/create/SSH reports do
not apply automatically to their new hashes; runtimeValidatedForTheseApks is false.
No device/app data, provider, real VM or host service was changed. Next: fresh
exact-APK Android 15 x86_64 owned-emulator native launch/create/readiness/SSH proof
against generated infrastructure, before approved real-target testing.


## 2026-10-21 — Fresh Android runtime prerequisites, target approval pending

Scope: prepare the exact-artifact runtime bench, without touching any unrelated
QEMU process/device or real Proxmox VM. Previous artifact-build turn made progress;
this turn restored actual emulator/image tools and current host fixture evidence.

Qualified ADB query on port 5047 found no emulator-5580; no device/app was changed.
An unrelated QEMU process was left untouched. Google stable emulator 37.2.12.0
(build 16428233) and Android 15/API 35 google_apis x86_64 image r09 were downloaded,
checked against official repository checksums and extracted into the existing owned
persistent SDK. Public URLs/checksums are in its verified-emulator-downloads.json.
No AVD was created or launched. Dedicated console/gRPC ports 5580/5581/8558 were
available, and `/dev/kvm` is accessible. Current android_ssh_fixture compiles with
the existing verified native prefix; no unrelated tests repeated.

The concrete plan is `.lab/android-toolchain/emulator-plan.json`: AVD
vmanyware-native-smoke, 2 CPU, 2048 MiB RAM, 2 GiB data, owned ADB 5047/serial
emulator-5580, JWT gRPC 8558, current x86_64 APK, app changes restricted to
`dev.vmanyware.client`. Production control-plane release binary and generated HTTPS
provider/localhost SSH fixture will be used; no real Proxmox mutation is proposed.

AGENTS.md explicitly requires approval of concrete VM targets/operations. A user
approval question is pending for creation/start of this Android VM plus app
installation/reset in it. SDK/image downloads and compilation are complete; no
approval is inferred from tool acceptance or elapsed time. The goal stays active,
not paused or complete. Other source work (saved-recreation UI, release/security
and platform gates) remains open independently.

Next: after actual target approval, create/start only this disposable AVD, verify
its name/QEMU marker/serial, install the exact current APK and execute native
verified HTTPS -> generated allocation/readiness -> strict SSH using the existing
direct host-alias mode. No fresh-APK runtime claim is made yet.


## 2026-10-21 — Client saved recreation history and manual lookup

Scope: read-only saved-source recreation panel for deleted rows. Android VM target
approval remains pending and no emulator/device/VM operation was performed. This
independent source work advances reconnection recovery without crossing that gate.

The panel explicitly lists history, selects saved rows or accepts a copied canonical
operation ID and refreshes only the saved record. It binds source/definition/name,
operation/new-instance identity and recorded guest/scope/plan success evidence.
Historical/current-state wording prevents recorded initialization being offered as
current SSH readiness. No checkbox, prepared consent/key or direct SSH action is
restored; no creation/confirmation/clear command is used. Failed/uncertain records
remain visible. Editing an ID/reconnecting invalidates late replies, and errors
remain distinct from valid empty history.

Verification: production frontend/TypeScript build passes; Firefox
`--recreation-recovery` passes actual React controls with simulated IPC, including
explicit empty/history reads, row/manual ID, invalid IDs, wrong source/false success,
uncertain evidence, stale edited-ID/history replies, reconnect and read-only command
assertions. Affected `--recreation` and `--deletion-recovery` scenarios are selected
for parent retired-row rendering. Runs are sequential. JS syntax/diff checks are
scoped to this slice; no Rust/server/Android suite repeated.

The delivered APK files and their metadata/checksums were not changed; they are
build snapshots before this new history panel, not proof of the updated source UI.
No fresh APK runtime or actual native restart claim is made. Next independent
slice: isolated Linux app/server saved-recreation restart recovery with provider
removed and writes off. The explicit Android emulator creation/app reset approval
question remains unanswered; automatic continuations do not grant it.


## 2026-10-21 — Actual native saved-recreation restart recovery

Scope: combined owned Linux `--recreation-recovery` fixture. No product behavior,
source UI, schema, dependency, emulator or real VM change. Explicit Android target
approval is still pending and was not inferred from automatic continuation.

The first real native UI completes the existing recreation journey with retained
review/consent, a deliberately lost accepted reply and identical retry. Before
restart the runner snapshots the succeeded operation, source/target and the live
generated provider's complete request/error lists. The actual server is restarted
on the same database with all five writes false and provider/catalog configuration
removed. A second app process begins without restored token, operation ID or intent.

Production native GET commands and the real history panel pass explicit history,
row selection, copied-ID lookup and refresh. Historical evidence is labelled and
no consent/form or direct historical SSH action appears. The proxy requires GET
only. Full saved operation and source/target identities remain unchanged; provider
request/error lists match the pre-restart snapshot, with six original mutations
only. Node provider snapshot/report lifecycle stays owned and closes on completion.
All app/server/fixture processes are cleaned up; TLS and sandbox stay enabled.

Verification: server and embedded-example builds pass; actual combined native
recreation/restart recovery passes, log
`.lab/native-deps/recreation-recovery-ui-check.log`. Shared deletion/restart also passes for changed launcher/recovery branching
(`.lab/native-deps/deletion-recovery-ui-check.log`). JS/Python syntax, Rust example
format and diff checks are scoped here; unrelated application suites were not run.

Limits: clean isolated Linux app/server restarts are proven, not live-provider
crash recovery, installed startup, current real guest/SSH state or fresh Android
runtime. Existing delivered APKs are unchanged snapshots preceding the source
history panel. Next independent slice: rebuild/package updated Android snapshots
with the completed source UI while exact-emulator execution awaits actual approval.


## 2026-10-21 — Updated Android delivery snapshots

Scope: rebuild and package both Android debug architectures from the latest
source, including saved-recreation history/manual-ID UI. No VM, app installation,
credential, dependency or production configuration change.

`make package-android` passes with the isolated owned SDK/JDK/NDK/signing cache.
The frontend build transforms 59 modules; both native targets and Gradle APK
packaging finish successfully. The packager verifies signatures, matching public
signer, package/version, SDK declarations, architecture and each embedded native
library against its current target build. APKs, INSTALL.md, artifacts.json and
SHA256SUMS are refreshed in `.lab/packages/android/`. Build log:
`.lab/android-toolchain/apk-update-build.log`.

ARM64 APK SHA-256:
`1be13a590527ab2997de9389bde0ce1bf5595c0acfdbe5df38995accb7cf1b5d`
(480318228 bytes). x86_64 APK SHA-256:
`331ef9b4dbe5a1ff7bb18b24f58c6ed45344c252ba3240237e082d14df5a691c`
(488664689 bytes). These development packages are large and retain native symbols;
size optimization is not release acceptance. Frontend assets are embedded in the
native Tauri library, not standalone APK JavaScript entries.

No runtime claim: metadata still records runtimeValidatedForTheseApks=false and
releaseCandidate=false. Earlier APK runtime evidence does not validate these hashes.
Next task: execute the exact x86_64 artifact allocation/SSH journey on the explicitly
approved owned Android target. Emulator creation/start approval remains pending;
real infrastructure and other-platform roadmap gates remain open.


## 2026-10-21 — Bounded production HTTP admission

Scope: phase-6 request admission, production wiring and API documentation. No new
runtime dependency, provider/VM operation, credential or deployment change.
Monotonic token bucket (burst 120, refill 20/second) and 32 handler permits share
bounded state across all connections. Limits wrap authentication; refused work
returns safe HTTP 429 plus Retry-After: 1 without entering route handlers.
No forwarding-header trust, per-client map or excess-request queue is introduced.

Verification: three focused request_limits unit/router tests pass, covering timed
refill/cap, shared admission/handler refusal and release, auth/fallback ordering.
All four affected auth integration tests and production entrypoint compilation
pass; see `.lab/request-limits-check.log` and `.lab/request-limits-auth-check.log`.
An actual debug-binary subprocess on temporary SQLite with no provider also passes:
180 rapid unauthenticated health reads produce 120 HTTP 401 and 60 HTTP 429;
all rejections have the safe code and Retry-After: 1. After a 1.1-second refill,
a read reaches authentication again (401). The owned process is terminated and
temporary state removed; log `.lab/request-limits-process-check.log`.
Changed Rust formatting and diff checks pass. API notes and ADR 0071 explain
process restart resets, global exhaustion, response-header concurrency scope and
remaining proxy/network protections. Phase-6 overall size/rate/deployment acceptance
is not closed by this slice. The previous Linux release archive is now a historical
snapshot; next task is rebuilding/rechecking that delivery artifact. Android binaries
and their pending exact-artifact runtime gate are unchanged.


## 2026-10-21 — Refreshed server delivery with request admission

Scope: rebuild the optimized Linux server archive, extend exact-package verification
for admission/refill, and include operator capacity notes. No host installation,
service/account change, provider call or VM mutation.

`make package-server` passes. Packaging is repeated after the installation notes
change so the archive includes the current documented request policy. Exact archive:
`.lab/packages/vmanyware-server-0.1.0-linux-x86_64.tar.gz` (5345000 bytes), SHA-256
`1b7782eee64c193bb84ca39688baa9d3d62205df1b4d7d07ebd44df4e8a58aab`. Its companion checksum and internal hashes pass.

`make check-server-package SERVER_PACKAGE=/absolute/path/to/archive.tar.gz` passes:
isolated copy-only staging/reinstallation preserves configuration and state; staged
unit syntax and token prestart guard pass; the exact packaged optimized process
passes authentication, definition persistence across two clean process starts,
unauthenticated-burst 429/Retry-After/safe error and successful authenticated read
after refill, plus invalid-token refusal before database creation. Temporary process
and files are cleaned up. Python syntax and diff checks pass. Logs:
`.lab/server-package-update.log`, `.lab/server-package-update-check.log`.

Limits: no service-manager sandbox/startup, real deployment TLS, real provider,
Android runtime or release-candidate acceptance. Android snapshots are unchanged.
Next independent source task: phase-6 threat-model/trust-boundary review against
current API/native/provider implementation; the exact Android journey continues
to require the already-requested owned-emulator approval.


## 2026-10-21 — Current-source threat model review

Scope: phase-6 documentation review of assets, attackers and trust boundaries.
`docs/security.md` maps ten boundaries to current source, existing contract tests,
actual recorded evidence, residual risks and remaining release work. It explicitly
distinguishes passphrase-encrypted application SSH keys from OS-backed key storage,
static bearer authority from separate identity/expiry, checksums from publisher
trust, provider readiness from SSH host authentication, and admission limits from
proxy/connection protection. Route diagnostics do not cover outer auth/429 refusals.
No credential file, private key, provider or VM was accessed.

The threat-model review checkbox is completed with this review artifact; other
security/platform/deployment gates remain open. Verification is document link,
content and diff review only; no unmodified application suites were repeated.
Existing server/Android artifacts are unchanged and retain their recorded evidence.
Next independent task: audit custom native-command exposure and release permissions,
then address demonstrated gaps with a small implementation slice. Exact-current
Android execution still awaits the already-requested owned-emulator approval.


## 2026-10-21 — Enforced custom native-command window permissions

Scope: current native command audit and demonstrated permission gap. Tauri's default
custom-command exposure was not restricted by the core-only main capability.
`build.rs` now enrolls 68 reviewed commands from app_commands.rs in AppManifest;
the explicitly selected default capability grants their generated permissions only
to local main. No wildcard/remote/extra WebView grant is enabled. Generated output
is ignored. Handler behavior and confirmation/trust policies are unchanged.

Owned example reporting/host rotation has a separate permission, ungranted and
unregistered in production, dynamically granted only by owned example setup.
`scripts/check-command-permissions.py` checks handler/manifest/grant drift and
production configuration. ADR 0072 and docs/security.md record the boundary and
its limits. Main-window compromise still carries the main window's authority.

Verification: production/native library and lifecycle/command-ACL example builds
pass with the verified native prefix. The new actual Linux two-window WebKit/IPC
fixture passes: main gets runtime_platform, untrusted gets a permission refusal.
The affected full native recreation/app+server restart fixture also passes through
real IPC, authenticated server, worker and generated HTTPS provider. No other
application suites repeated. Logs: `.lab/native-deps/command-acl-ui-check.log`,
`.lab/native-deps/command-acl-recreation-check.log`, and command-acl build logs.
The first lifecycle build needed the Manager trait call qualified for example-only
dynamic grants; that compile error is fixed and the subsequent build/run passes.

Limits: this is Linux runtime evidence, not other-platform or release acceptance.
No VM/provider/credential changes, no real key-store access. Current Android APK
snapshots now predate this native ACL source change; next task is rebuilding and
packaging both architectures. Exact Android runtime still awaits the previously
requested explicit owned-emulator creation/start approval. The Linux server package
is unaffected by this client-only slice.


## 2026-10-21 — Android snapshots include explicit command ACL

Scope: rebuild/package both Android debug targets after ADR 0072. No VM/emulator
creation/start, app installation, credential access or real provider operation.

`make package-android` passes with the isolated owned SDK/JDK/NDK/signing prefix.
The frontend build and both native target/Gradle builds pass; package verification
checks signatures, shared public signer, manifests/SDK/ABI and each exact native
library hash. The generated default capability matches the reviewed source grants,
is local-main only and has no remote grant. All 68 declared permission identifiers
are present in each target library. This is compilation/content evidence, not
Android runtime enforcement proof. The existing Linux two-window native acceptance
remains the only new runtime evidence for this ACL.

arm64-v8a APK: 480915012 bytes, SHA-256
`ec17d953c34ea5bd4eaf7029c0f061d59b0cfb4f67c3a5c455810f2ec2fbe45d`.
x86_64 APK: 489206593 bytes, SHA-256
`1a04fbfe7de6b01e507968a059cfcf0af313f6d60a8e1828ad9b74ff8976a57d`.

Artifacts, public metadata, SHA256SUMS and installation notes are refreshed in
`.lab/packages/android/`. Build log `.lab/android-toolchain/apk-acl-build.log`.
The metadata retains releaseCandidate=false and runtimeValidatedForTheseApks=false.
Checksum verification and scoped diff review pass; no unrelated suites repeated.
A one-off binary inspection initially used Python mmap membership instead of byte
substring search; corrected find() assertions pass without rebuilding anything.

Next independent source task: review platform secret-storage support/decision and
address a demonstrated gap in a bounded slice. The exact Android allocation/SSH
journey still awaits the previously requested owned-emulator approval; no automatic
continuation grants it. Server package and its recorded evidence are unaffected.


## 2026-10-21 — Remove unused Android sharing authority

Scope: review Android private-data/backup boundary and remove the demonstrated
unused broad FileProvider/path resource inherited from the scaffold. The manifest
already disabled backups and excluded cloud/device-transfer data, so those rules
are retained. No real key/credential file, VM, app install or provider was accessed.
No OS Keystore/Keychain completion is claimed.

The provider was unexported and no URI-sharing product call was found; this is
removal of unnecessary potential authority, not evidence of an existing leak.
ADR 0073 records the decision. The packager now checks each compiled manifest for
provider absence, disabled backup flags and extraction-rule reference, and rejects
the former broad path resource. It rejects the previous actual APK before copying
its first output. Fresh `make package-android` succeeds for both architectures,
including signature/ABI/SDK/native-library checks and the new manifest checks.
The updated installation notes are recopied by the verified packager afterward.

arm64-v8a artifact: 480914944 bytes; SHA-256
`062372dfe0fe537adf127439af647305ddf2945ba35e6d2ed7e49e3dae9b6657`.
x86_64 artifact: 489206525 bytes; SHA-256
`f559bebb9f9d89f36a34f5652f7e76857a7e91d9ca46869672534b224722e2a5`.

Evidence logs: `.lab/android-toolchain/storage-policy-old-apk.log` (expected refusal),
`.lab/android-toolchain/apk-storage-build.log`,
`.lab/android-toolchain/apk-storage-package.log`. Python syntax, checksum and diff
checks pass; no unrelated application suites repeated. Public metadata keeps
releaseCandidate=false and runtimeValidatedForTheseApks=false. Server archive is
unchanged. No fresh runtime/backup-transfer evidence is implied.

Next independent task: improve client handling of the new server's HTTP 429 so
capacity refusal gives an actionable wait/retry message without clearing retained
mutation scope, identity or idempotency. Android exact-artifact runtime continues
to await the previously requested owned-emulator approval.


## 2026-10-21 — Actionable native remote capacity errors

Scope: phase-5 actionable operation errors for the new server admission contract.
The four remote HTTP response decoders (ordinary reads, creation/recreation,
snapshot/restore, deletion/recovery) now recognize 429 before requiring the bearer
response marker. Admission is outside authentication, so a genuine capacity refusal
has no marker. A fixed message asks the operator to wait at least one second and
retain the same mutation key or read the known operation. This is not authentication
proof, an upstream-body display, a parsed arbitrary retry instruction or automatic
retry. No state transition or endpoint/permission changes were introduced.

Verification: two focused native HTTPS tests pass, including all four decoders
with/without the auth marker and private/malformed upstream content; a refused
creation preserves plan/key/submitted state, blocks replacement and accepts an
explicit identical-key/body retry. Affected auth-marker/status/oversize refusal
checks, existing creation-error retention and known-operation-only status checks
also pass. Logs `.lab/native-deps/client-capacity-check.log` and
`.lab/native-deps/client-capacity-affected-check.log`. Rust formatting/diff pass.
No unrelated server/browser/platform suites repeated; UI already renders native
errors. No VM, real provider, credential or app installation operation.

Limits: Linux native HTTPS tests, not new Android runtime proof. Existing APK
snapshots now predate this client-source message change; server archive is unchanged.
Next task: refresh Android delivery snapshots with this completed native diff, then
execute exact-artifact acceptance after the already-requested owned-emulator approval.
The rest of the roadmap, OS-backed key storage and real target gates remain open.


## 2026-10-21 — Android delivery includes capacity refusal handling

Scope: refresh both Android debug APKs for the completed native 429 diff. No new
application behavior, dependencies, VM/emulator operation or app installation.
`make package-android` passes: frontend, both native targets and APK assembly,
shared signature/version/ABI/SDK/current embedded-native checks, compiled backup
policy and unused-file-sharing-provider absence. Both native libraries contain the
new fixed wait/retry message. SHA256SUMS verifies the exact delivered APKs.

arm64-v8a: 480919344 bytes; SHA-256
`9f8bc7401fa93331e38018fc5c8c0fb19e8eb0075fb8427e2e37520a2f65f201`.
x86_64: 489215789 bytes; SHA-256
`250be0b64cde070b2c17ae9e8e06bbad8e0a3ee9c1f440cfa81895c8d5d75491`.

Artifacts/metadata/instructions are in `.lab/packages/android/`; log:
`.lab/android-toolchain/apk-capacity-build.log`. Scoped diff review passes. No
unmodified application tests repeated. Metadata still records releaseCandidate=false
and runtimeValidatedForTheseApks=false: compilation/content does not establish
Android behavior. Existing Linux native HTTPS evidence remains separately recorded.
Server package is unchanged.

Next independent task: exercise the documented server database backup/restore
procedure against the current packaged process and isolated temporary state.
Exact-current Android allocation/SSH runtime still awaits the previously requested
owned-emulator creation/start approval, which automatic continuation does not grant.


## 2026-10-21 — Exact-package stopped backup and restore acceptance

Scope: extend the package fixture to execute the documented stopped-state copy and
restore using three actual packaged processes. No product source/dependency, VM,
provider, host account/service or credential change; no APK rebuild is needed.

The first process creates a definition and an actual inventory-refresh operation
through authenticated HTTP. With no provider configured, the worker records the
expected provider_not_configured failure. After clean SIGTERM/exit, the fixture
copies the complete temporary runtime directory to a private backup, including
SQLite/lock files and an extra retained-file sentinel. The second process reads
identical definition/operation/history and creates a later definition. After another
clean exit the entire runtime directory is replaced by the backup. Before startup,
all copied files match byte-for-byte and directory mode remains 0700.

The third process reads the original definition and exact terminal operation/history,
returns 404 for the post-backup definition, and returns the same terminal operation
for its original idempotency key without creating another history entry. The extra
retained file is restored; a post-backup file is absent. All owned processes and
temporary files are cleaned up. Existing package hashes, staging/reinstall,
authentication, unit syntax, admission/refill and invalid-token checks still pass
as part of this changed package fixture, not unrelated application suites.

Verification: `make check-server-package` with the current Linux archive passes;
`.lab/server-backup-restore-check.log`. Python syntax and diff review pass. An initial
fixture assertion expected a failure object rather than the actual serialized enum;
corrected to the real string contract before the passing run. The release archive
and APK hashes are unchanged. The archive already contains the tested stopped-copy
instructions; source installation notes now add the precise evidence/limits.

Limits: current-version state under the current OS user, not secret-config backup,
service-account ownership/systemd sandbox, live backup, prior-version upgrade,
uncertain provider mutation or VM recovery. Next task: previous-schema upgrade with
the exact packaged server on a disposable database. Exact Android runtime remains
pending the previously requested owned-emulator approval.


## 2026-10-21 — Previous-schema migration in the packaged process

Scope: opt-in `--previous-schema` package fixture, not product schema/source changes.
The fixture constructs schema 14 with the actual checked-in migrations and inserts
one saved definition (revision 3, exact YAML/hash) and terminal inventory operation.
Other old tables exist but are empty; this does not cover populated mutation journals.
A stopped pre-upgrade copy is saved before the first packaged process starts.

The actual current archive migrates the database to schema 15 on startup. Across
three processes (including the stopped-copy backup/restore scenario), old rows are
retained exactly, legacy definition/API/hash/revision and terminal operation agree,
original idempotency key returns the same operation, SQLite integrity/foreign keys
pass, recreation_sources starts empty and its three triggers exist. An invalid
lineage insert is rejected by validate_recreation_source and rolled back. The
pre-upgrade copy's hash remains unchanged. Existing package staging/auth/admission
checks run within this affected fixture and pass; no unrelated suites repeated.

Verification: `python3 scripts/check-server-package.py <current archive> --previous-schema`
passes; log `.lab/server-previous-schema-check.log`. Python syntax and diff review
pass. No host unit/account, credentials, real provider, VM or Android changes.
All temporary processes/files are cleaned up. Archive/APK hashes are unchanged.

Limits: SQL fixture -> current packaged binary, not an actual older binary upgrade,
populated provisioning/snapshot/restore/deletion journals, supported release matrix,
service account migration or app upgrade. The phase-6 upgrade item stays open.
Next task: current package must refuse a future schema without modifying its logical
data/version; verify that failure mode on a separate disposable fixture. Exact
Android runtime still awaits the previously requested owned-emulator approval.


## 2026-10-21 — Future-schema fail-closed package acceptance

Scope: opt-in `--future-schema-only` exact-package check, separate from the normal
backup/restore and previous-schema modes. No application behavior, dependency,
artifact, credential, service or VM changes.

After existing archive/staging/unit checks, the fixture creates an owned schema-16
SQLite database with an application ID, witness table and full-byte-range BLOB.
Two actual current packaged-process starts both exit 1 with only the fixed
server_failed/database_unavailable event and no token output. Exact database bytes,
user_version, application_id, schema and witness rows remain unchanged; SQLite
integrity passes and no rollback journal remains. Both failures reach the same
storage check (the service lock is released), and the selected port can still be
bound afterward. The companion lock pathname may be created; no claim is made
that absolutely no filesystem entry changes.

Verification: `python3 scripts/check-server-package.py <archive> --future-schema-only`
passes; `.lab/server-future-schema-check.log`. Python syntax and diff review pass.
Normal process/backup/previous-schema scenarios were not rerun: the new mode exits
before those branches and does not change their behavior. No live configuration,
private key, real provider or Android access. All temporary state is cleaned up;
existing archive/APK hashes remain unchanged.

Limits: synthetic future schema, not an actual later application release, concurrent
writer or corrupt-database recovery. The full upgrade/security/platform roadmap is
still open. Next task: reconcile remaining roadmap requirements with recorded
source/artifact/runtime evidence and identify actual unfinished product work versus
external acceptance gates. Exact-current Android runtime still awaits the already-
requested owned-emulator approval; automatic continuation does not supply it.


## 2026-10-21 — Roadmap readiness review identifies missing remote controls

Scope: compare product/MVP/phase/security/testing requirements with current source
and recorded evidence, without rerunning unmodified suites or declaring completion.
`docs/roadmap-status.md` maps every §8 MVP outcome, phases 0–6 and additional
schema/API/security/platform/local-provider obligations to evidence and open work.
No percentage or closure based on counts is used.

The primary demonstrated product gap is remote existing-VM start/shutdown:
RemoteConnection renders no lifecycle action, and LifecycleActions/control_plane
uses a local port/loopback rather than the retained HTTPS profile. Remote rows also
lack explicit live status/detail and existing-VM SSH handoff. These are prioritized
ahead of secondary refinements and unchanged artifact rebuilds. Server lifecycle
API/executor and local native tests already provide a working contract to reuse.
Other actual gaps include rejected package/automatic-snapshot declarations,
OS-backed secret storage/platform implementation and recorded local-provider scope.
Real endpoint/guest/storage, exact APK, other-platform and installed service gates
remain separate and open. Conditional/unimplemented definition update/delete API
proposals are explicitly tracked rather than silently counted complete.

Verification: inspected current RemoteConnection/LifecycleActions/control_plane,
server minimum-create gate, lifecycle API, roadmap, platform evidence, phase-0 and
ADR 0062; document content/path and diff review only. No credential, provider, VM,
artifact or product source change. Next slice: native retained remote lifecycle
preflight/clear with fixed scoped HTTPS paths and validation, no provider mutation.
Follow with confirmed same-key submission/status/recovery and remote UI; the full
roadmap is retained. Existing emulator approval question is still pending.


## 2026-10-21 — Native remote lifecycle preparation and clear

Scope: first functional remote start/shutdown slice, fixed GET review only.
Adds lifecycle.rs native module, retained connection state, two main-only commands,
manifest/capability grants and tests. No server/API/provider mutation, UI or dependency
change. Selected target/action/request key are validated before network access.
Fixed registered-instance detail and lifecycle-preflight reads bind definition hash,
instance/scope/node/VMID/marker/digest; five unique known checks and power/QMP-derived
aggregates must agree. Failed/unknown/disabled reports remain review evidence, not
consent. Untrusted server check messages are stripped.

Monotonic preparation time is retained for the next confirmation slice. Wrong-key
clear is refused; matching clear invalidates outstanding reads; reconnect/disconnect
resets state. Concurrent preparation is refused. Generation/epoch/key checks after
the first response prevent continuation after clear/disconnect, and final checks
prevent stale evidence from reappearing. No arbitrary path/URL or write command.

Verification: five native HTTPS tests pass, covering fixed GETs, validation/refusals,
blocked states, retention/clear and overlapping/disconnect races. One shared capacity
regression passes. Logs `.lab/native-deps/remote-lifecycle-review-check.log` and
`.lab/native-deps/remote-lifecycle-shared-check.log`. Native library compilation,
70-command main-only permission drift, changed Rust formatting and diff review pass.
No unrelated server/browser/Android suites repeated. ADR 0074 records the contract.

Limits: Linux native protocol evidence only; no actual UI/native-window/Android or
real target behavior for these new commands. Existing delivered APKs predate this
slice and remain snapshots; avoid rebuilding before the complete lifecycle flow.
Next slice: explicit fresh-review confirmation, retained same-key/body retry and
matching known-operation GET status. Then remote UI/recovery/native integration.
The full roadmap and earlier pending Android emulator/real-target approvals remain.


## 2026-10-21 — Native confirmed remote lifecycle and status

Scope: add explicit confirmation and known-operation status to retained remote
start/shutdown review. Reuses local LifecycleApproval/LifecycleOperation validation
through narrowly crate-visible fields/functions, avoiding another operation schema.
Shared bounded response transport is reused and its interrupted-action wording is
made generic. Two commands join the main-only ACL (72 total).

First submission requires consent, enabled/passed selected action and review age
under 120 monotonic seconds. The complete approval is built from retained target/
definition/digest/action and sent with the retained key to fixed start/shutdown POST.
Unknown replies preserve submitted scope and permit only explicit same-key/body
retry; review replacement and nonterminal clear are blocked. Once ID is known,
confirmation/status issue GET only. Matching ID/action/instance/task/result/power
validation uses the existing local validator, with an additional impossible-queued-
provider-task guard. Uncertain and known-task interruption stay retained; safe
terminal evidence permits clear. Inflight overlap and stale disconnected replies
are refused without restoring state. No auto retry or real provider mutation.

Verification: nine native HTTPS lifecycle tests pass, including both actions,
malformed reply after complete request receipt, identical retry, GET-only confirmation
and explicit status reads, fresh/consent/capability refusal, result mismatch and
submission/disconnect races. Three affected local lifecycle and two shared capacity
checks pass. Logs `.lab/native-deps/remote-lifecycle-confirm-check.log` and
`.lab/native-deps/remote-lifecycle-confirm-affected.log`. Native library compilation,
permission drift, changed Rust formatting and diff review pass; no unrelated suites.

Limits: Linux native protocol tests only. UI, saved operation recovery after losing
native state, actual native window/worker and Android/real-target acceptance remain
open. Existing server/APK archives are unchanged; APKs predate this unfinished feature.
Next slice: remote lifecycle review/confirmation/status UI with explicit target/action,
mutation lock, same-request retry and separate power-vs-readiness wording. Then add
saved recovery and actual native integration before Android artifact refresh.


## 2026-10-21 — Remote workspace lifecycle controls

Scope: remote start/graceful-shutdown selection and React retained review/consent/
retry/status flow. Adds RemoteLifecycle and mounts one selected action beside existing
remote workflows. Selection itself makes no request; only registered rows expose the
buttons. Reviews bind target/action/hash/digest/marker/observation, show known check
labels and disabled/unknown prerequisites, and start with unchecked consent.

Review/inflight/submitted state locks competing lifecycle/snapshot/restore/delete/
recreate/create controls. Submission IPC carries only connection ID, retained key
and explicit consent; unknown replies retain the same request and disable clearing.
A known operation removes retry and exposes explicit status reads. Matching ID/kind/
instance and task/result/scope/power evidence are required before success; uncertain
or known-task interrupted work cannot be acknowledged. Power state never implies
guest/SSH readiness, and graceful shutdown warns about unsaved work. No SSH handoff
is added from power-state evidence. Terminal acknowledgement clears consent/key.

Fixed native pre-submission expiry/blocked errors reset unsubmitted/unchecked state
so cancellation/review remains possible; unknown transport/server errors stay retained.
Cleanup/generation guards discard stale replies after selection/disconnect/reconnect.

Verification: frontend build passes; focused Firefox --remote-lifecycle simulated-IPC
flow passes both actions, capability/unknown/target refusals, expiry before submission,
unchecked consent, cross-action target lock, identical unknown retry, known reads,
unrelated success refusal, uncertain retention, terminal acknowledgement and stale
reconnect. Shared parent --recreation and --snapshot-create checks also pass, selected
because target clearing/locks changed. Logs `.lab/remote-lifecycle-ui-build.log`,
`.lab/remote-lifecycle-browser-check.log`, and remote-lifecycle-parent-*-check.log.
JS syntax and diff review pass. No unrelated Rust/server/platform suites repeated.

Limits: simulated IPC in Firefox, not native-window/server/worker/provider or Android
runtime for the new controls. Existing APK snapshots are unchanged and predate this
feature. Next slice: actual Linux native remote start/shutdown against the generated
HTTPS provider with same-key loss recovery and matching power evidence. Saved native
lifecycle recovery remains open; refresh APKs after integrated acceptance. No real
VM/provider operation or credential access; earlier emulator/real-target approvals
remain pending and are not inferred from continuation.


## 2026-10-21 — Actual remote lifecycle native integration

Scope: new --remote-lifecycle mode in the owned Linux fixture and its native driver.
No product behavior/schema/dependency, host installation, real VM or credential
change. The example registers the four production remote lifecycle commands and
loads the embedded real React frontend. Existing owned TLS/Xvfb/sandbox/process
cleanup remains in use; no IPC or local control-plane command substitutes for the
remote workflow.

The UI connects with verified generated HTTPS/CA and bearer authentication, loads
the workspace, selects start, reviews/cancels/reviews, confirms, receives a deliberately
lost accepted reply and explicitly retries. It then reads known operation status and
acknowledges success before repeating for graceful shutdown. Each retry has identical
API path/key/body, and the two distinct actions use distinct keys. Only four action
HTTP POSTs occur (two per key); known operation GETs are observed independently.
Exactly one provider start and one graceful shutdown occur, with bounded shutdown
options and no force stop. Two persisted succeeded jobs show known successful tasks,
matching instance/scope/node/VMID and requested running/stopped states. The instance
association remains unchanged. Power never claims guest/SSH readiness.

Verification: embedded native example builds and actual --remote-lifecycle passes;
logs `.lab/native-deps/remote-lifecycle-ui-build.log` and
`.lab/native-deps/remote-lifecycle-ui-check.log`. The shared --snapshot-restore proxy
scenario also passes because the accepted-response loss branch now handles one
lost reply per lifecycle action; log
`.lab/native-deps/remote-lifecycle-shared-proxy-check.log`. JS/Python syntax, example formatting and diff review
are scoped to the fixture. No unrelated browser/Rust/server suites repeated.

Limits: isolated Linux/generated-provider proof, not a real VM, Android runtime,
installed startup or saved lifecycle recovery after native state loss. Current APK
snapshots remain unchanged and predate the feature. Next slice: saved lifecycle
GET recovery with no restored consent, then include the completed flow in Android
artifacts. Previously requested emulator/real-target approvals remain pending.


## 2026-10-21 — Server saved lifecycle history and detail

Scope: instance-scoped server/store GET history/detail for recorded start/shutdown.
No provider, native/UI, schema, dependency or VM behavior change. History resolves
registered/archived instance scope, filters lifecycle kinds before newest-100 limit,
sorts by created_at/id descending and validates each job. Detail resolves matching
instance/kind before parsing intent; corrupt unrelated data returns the same 404.
Returns original saved target/action, public operation and dispatch-started marker;
never restores confirmation, request key or task identifier. Provider configuration
and all write flags may be absent.

Durable lifecycle job validation is reused. Saved approval JSON is now capped at
4096 bytes in SQL before loading, matching the API contract and affecting worker/
job-reader callers too. Success requires matching saved action/instance/scope/task/
power/target and typed result serialization strips unexpected fields. Matching
corruption/oversize yields fixed 503 without journal content; history fails closed.
Queued/running/uncertain evidence is read unchanged and never claimed or enqueued.

Verification: six focused HTTP-router/SQLite tests pass, including reopened saved
start/shutdown on an archived fixture, no-provider/write-off reads, unchanged jobs,
wrong instance/kind/ID and wrapped-auth refusal, pre-limit filtering/order/bounds,
corruption/oversize isolation, result sanitization and wrong-power refusal. Nine
shared lifecycle storage tests pass for the bounded job-reader change. Logs:
`.lab/lifecycle-recovery-api-check.log`, `.lab/lifecycle-recovery-storage-check.log`.
Rust formatting/diff review passes; no unrelated suites repeated. Initial test
corruption used invalid JSON and hit SQLite's existing CHECK, so the fixture was
corrected to valid-but-damaged JSON without disabling constraints. Initial result
decoding compilation needed Deserialize on the typed evidence; fixed before passes.

Limits: router/store reopen, not actual binary/app restart, native recovery, Android
or real-provider evidence. Existing server archive now predates these GET endpoints;
APK snapshots also predate the remote lifecycle feature. Next slice: stateless native
fixed history/detail GET readers and validation; then recovery UI and actual restart
integration before refreshing delivery artifacts. All earlier emulator/real-target
approval requirements remain pending.


## 2026-10-21 — Stateless native saved lifecycle readers

Scope: fixed history/detail HTTPS commands through the retained remote profile.
Adds lifecycle_recovery module and two local-main grants (74 production commands).
Neither reader mutates retained lifecycle state or constructs approval/consent.
Input IDs are canonical before network access; generation checks discard stale
responses. History validates selected instance, latest-100 bound, unique IDs,
sorted metadata, typed action/status, failure allowlist and basic outcome coherence.
Detail validates saved target shape/definition hash/instance/known operation/action,
dispatch-task consistency and matching task/power result. Historical provider scope
is retained rather than replaced with current configuration. Unknown work remains
historical evidence; no live-state, readiness or SSH claim is introduced.

Existing local/remote lifecycle approval validation now delegates outcome matching
to a pure target/action evidence function; native recovery reuses that function
without fabricating confirmation/digest fields. Failure allowlisting is likewise
shared. Existing submission callers retain their original approval checks.

Verification: six focused native generated-HTTPS tests pass for exact GET paths,
no request-key/consent exposure, history bounds/duplicates/order/status, unrelated
result refusal, uncertainty/extra-field sanitization, auth/marker/capacity/size
refusal, and disconnected response suppression. Three affected local lifecycle
and nine affected remote lifecycle tests pass for the shared-validator extraction.
Logs `.lab/native-deps/lifecycle-recovery-native-check.log` and
`.lab/native-deps/lifecycle-recovery-native-affected.log`. Native library compilation,
command-grant drift, changed Rust formatting and diff review pass. No unrelated
server/browser/Android suites repeated; no VM, provider or credential access.

Limits: Linux native protocol only; recovery UI and actual app/server restart are
not proven yet. Server archive/APKs unchanged and predate the unfinished recovery
feature. Next slice: explicit lifecycle history/manual-ID/refresh UI with historical
wording, no consent/SSH action and stale-response guards. Then actual restart
integration before delivery refresh. Earlier emulator/real-target approvals remain
pending and are not inferred from continuation.


## 2026-10-21 — Saved lifecycle recovery UI

Scope: RemoteLifecycleRecovery panel on registered and archived remote rows, plus
focused browser fixture. Explicit history/list entry/manual-ID/refresh commands only;
no write, consent, retry-action, SSH action or persistence. Historical target/action/
status are displayed separately from current runtime truth. Recorded success requires
matching instance/action/kind/known task/target/scope/power evidence; uncertain work
never becomes successful. Errors remain distinct from empty history.

The UI validates target/IDs and result consistency, discards detail after edited ID,
and invalidates history/detail on cleanup/reconnect. No operation ID or confirmation
is automatically restored. Unique form labels support multiple environment panels.

Verification: frontend build passes; --lifecycle-recovery real React/Firefox with
simulated IPC passes registered/archived controls, explicit reads, history-empty/error
separation, saved target context, no consent/retry/SSH, uncertainty, wrong-instance/
false-success refusal, invalid ID blocking, historical shutdown evidence, edited-ID
stale reply and reconnect reset. Logs `.lab/lifecycle-recovery-ui-build.log`,
`.lab/lifecycle-recovery-browser-check.log`. Shared parent --remote-lifecycle and
--recreation-recovery scenarios also pass for the changed environment list; logs
`.lab/lifecycle-recovery-parent-action-check.log` and
`.lab/lifecycle-recovery-parent-history-check.log`.
JS syntax/diff review is scoped here; no unrelated Rust/server/Android suites.

Limits: simulated IPC, not new actual native-window/app-server restart or Android
runtime evidence. Existing server/APK archives unchanged and predate this recovery
path. Next slice: actual isolated Linux app/server restart recovery with provider
removed, all write flags false, GET-only reads and unchanged operation/identity/provider
counters. Then refresh the server/Android delivery artifacts. Earlier emulator and
real-target approvals remain pending.


## 2026-10-21 — Actual native lifecycle app/server restart recovery

Scope: --lifecycle-recovery owned integration mode and second-app driver. No product
behavior/schema/dependency, real VM, credential or host service change. The first
actual native app uses the already-proven remote lifecycle journey: one provider
start/shutdown each, lost accepted API replies and identical-key/body retries.
The fixture snapshots both full operations and provider counters, then cleanly
restarts the actual server on the same DB/new port with all five writes false and
provider/catalog absent. A different native app process begins without token, ID,
review or consent; it explicitly connects and loads recorded lifecycle history.

Both historical action details, manual copied-ID lookup and refresh pass through
production native HTTPS commands and the actual recovery panel. Recovery proxy
traffic is GET only. Both full operation records, saved instance and definition,
and provider read/power/task/write/running/task-reference counters stay unchanged.
No confirmation or SSH action appears in the history panel. Recorded start/running
is labelled historical while the current power state remains unobserved by recovery.

Verification: current server and embedded native example builds pass; combined
actual --lifecycle-recovery passes. Logs `.lab/native-deps/lifecycle-recovery-server-build.log`,
`.lab/native-deps/lifecycle-recovery-native-build.log`,
`.lab/native-deps/lifecycle-recovery-ui-check.log`. Shared --recreation-recovery is
also passes because the restart launcher now supports two lifecycle jobs as well as
single-job recovery; log `.lab/native-deps/lifecycle-recovery-shared-restart-check.log`. JS/Python syntax, example formatting and diff review are scoped
here; no unrelated suites repeated. Owned processes/temp state are cleaned up and
TLS/WebKit sandboxing remain enabled.

Limits: clean isolated Linux/generated-provider restart, not crash/real infrastructure,
Android runtime, installed startup or whole-roadmap completion. Server/APK archives
remain unchanged snapshots predating the recovery feature. Next slice: refresh the
Linux server and Android delivery artifacts with this integrated feature and exact
package checks. Exact Android execution/real target writes still require the earlier
pending concrete approvals, which continuation does not grant.


## 2026-10-21 — Deliver remote lifecycle and saved recovery artifacts

Scope: refresh optimized Linux server archive and both Android debug APKs after the
complete remote lifecycle/history/restart feature. No new application behavior,
dependency, VM, credential, host service or app installation change.

`make package-server` succeeds. The exact archive passes staging/reinstall/state
preservation, unit syntax/token guard, actual packaged authentication/admission/
refill, restart/persistence, stopped-directory backup/restore/history and invalid-token
refusal. The package fixture additionally verifies that authenticated missing-scope
requests reach both new lifecycle recovery handlers (their specific 404 contract)
without a provider. This binds the endpoints to this artifact; actual native remote
start/shutdown/restart evidence remains the separately recorded generated-provider
Linux debug-process journey, not a new real infrastructure or optimized-worker claim.

Server archive: 5375491 bytes; SHA-256
`72d3c5fef4cf58fab9c7fd865719b4d65cc19b290ce95306b8036eaa7a3aaedb`.

`make package-android` succeeds for ARM64/x86_64. Packager checks both signatures,
shared signer, manifest/ABI/SDK, backup/sharing policy and embedded native hashes
against the current target builds. Generated capabilities match source: local main
only, all 74 command permission identifiers present in both native libraries.
arm64-v8a: 482803600 bytes; SHA-256
`12d0f26c4a4c5279440dc238c85ac33c4870be613912b696e48efb54ec937e99`.
x86_64: 491117869 bytes; SHA-256
`e8baa50b834500791d409a2e456a89bef163e020ccaaec4a7e1315a10c54a33a`.

Artifacts/public instructions/checksums are refreshed in `.lab/packages/` and its
android subdirectory. Logs `.lab/lifecycle-delivery-server-build.log`,
`.lab/lifecycle-delivery-server-check.log`,
`.lab/android-toolchain/lifecycle-delivery-android-build.log`. Exact checksum,
permission drift, Python syntax and diff checks pass. No unrelated application
suites repeated. Metadata remains releaseCandidate=false and, for APKs,
runtimeValidatedForTheseApks=false. Earlier APK runtime evidence does not transfer.

Limits: no exact-current Android execution, real target/provider/guest proof, systemd
sandbox/deployment TLS or release signing. Next source slice: explicit scoped live
status/detail read for an existing remote VM, without inferring guest/SSH readiness
from power. Current emulator creation/start and real-target write approvals remain
pending; automatic continuation supplies neither.


## 2026-10-21 — Native scoped remote instance power read

Scope: explicit native read for existing registered remote target, reusing the
lifecycle Target shape through parent-module visibility rather than another model.
Adds one main-only command (75 total) and fixed InstanceStatus read path with a
4096-byte response cap. Server source/API is unchanged. The returned instance ID,
provider scope, node and VMID must match selection; power/QMP states are bounded
to the existing server vocabulary. Extra fields such as guestReady are stripped.
No prepared intent, consent, key, lifecycle state or observation job is installed.

Verification: five focused native generated-HTTPS tests pass for exact GET/no request
key, running/stopped/paused/unknown semantics without readiness inference, target/
state mismatch, invalid inputs before transport, 401/429/404/503/missing marker/
response-size refusal without body/token echo, and stale disconnected response.
Log `.lab/native-deps/remote-instance-status-check.log`. Native library compilation,
75-command main-only grant drift, changed Rust formatting and diff review pass.
No unrelated application suites repeated; no VM/provider/credential change.

Limits: Linux native protocol only, no new UI/native-window/Android/real VM evidence.
Power read does not attest ownership marker, guest initialization/IP, SSH access or
host trust. Existing server archive is unchanged; Android snapshots predate this
in-progress feature. Next slice: explicit read/refresh UI on registered remote rows,
show observation time and possible staleness, clear prior result on errors/reconnect,
and never infer SSH readiness. Then actual native integration before APK refresh.
All earlier real-target/emulator approvals remain pending.


## 2026-10-21 — Explicit remote power observation UI

Scope: RemoteInstanceStatus on registered remote rows only; no backend/native/
dependency changes. Loading workspace makes no status request. Explicit read/refresh
passes the selected instance/scope/node/VMID and validates the returned target,
power and normalized QMP before display. Shows local read-completion ISO time and
possible staleness; no ownership/guest/SSH readiness inference or automatic action.
The old unconditional live-state-not-checked row text is replaced by registered
plus the panel's actual observed/unobserved qualification.

A new read clears old evidence. Failed/wrong-target replies leave power unobserved.
Connection/target cleanup and mutation-review lock transitions invalidate pending
and displayed results; replies from before an action review cannot reappear after
cancellation. Archived rows have history but no live-read control.

Verification: frontend build and --remote-power real React/Firefox simulated-IPC
scenario pass for explicit reads, binding, running/paused/unknown distinction,
time/staleness, error clearing, wrong-target rejection, mutation-review race,
reconnect suppression and no write/SSH. Logs `.lab/remote-power-ui-build.log`,
`.lab/remote-power-browser-check.log`. Shared --remote-connection and --remote-lifecycle
also pass for environment-row/qualification/lock changes; logs
`.lab/remote-power-parent-connection-check.log` and
`.lab/remote-power-parent-lifecycle-check.log`. JS syntax/diff review
is scoped here; no unrelated Rust/server/Android suites repeated.

Limits: browser/simulated IPC, not new actual native-window/provider or Android proof.
Existing delivery artifacts unchanged and predate this in-progress power-read feature.
Next slice: actual Linux native HTTPS/status/provider read integration before APK
refresh, checking explicit post-action refresh and unchanged provider mutation count.
All real-target/emulator approvals remain pending.


## 2026-10-21 — Actual native explicit power observation

Scope: extend the owned remote lifecycle native driver with three explicit status
reads and the production status command registration; no product/server/dependency
change. The actual WebView/native HTTPS/server/provider path reads stopped before
any action, running after acknowledged start, and stopped after acknowledged graceful
shutdown. Relay asserts exact instance/scope/node/VMID and sequence. The UI shows
read completion time, staleness and no-readiness/ownership semantics; action review
removes old power evidence and disables reads. Reads do not add mutations: exactly
one provider start/one shutdown and four same-key-paired API POSTs remain.

Verification: embedded native example build and actual --remote-lifecycle pass;
logs `.lab/native-deps/remote-power-native-build.log` and
`.lab/native-deps/remote-power-native-ui-check.log`. --lifecycle-recovery also passes
for the shared driver, proving newly mounted power controls do not contact the
provider during GET-only historical recovery with writes/provider off; log
`.lab/native-deps/remote-power-restart-check.log`. JS/Python
syntax, example formatting and diff review are scoped here. No unrelated suites.

Limits: isolated Linux/generated-provider proof, not real VM/guest/SSH identity,
Android runtime or deployed service. Existing archives unchanged: server product
code is unaffected; APKs predate this completed client power-read feature. Next
slice: rebuild/package Android snapshots only, with exact native/permission/manifest
checks; no reason to rebuild the unchanged server. All pending emulator/real-target
approval requirements remain unchanged.


## 2026-10-21 — Android delivery includes explicit remote power reads

Scope: rebuild/package both Android debug targets for the completed scoped power
read/refresh feature. No new product behavior, VM/emulator, installation, credential,
server build or dependency change. `make package-android` succeeds with the owned
SDK/JDK/NDK/signing prefix. Both APKs pass signature/shared signer, package/version/
ABI/SDK, backup/sharing manifest policy and exact embedded native-library checks.
Generated capability equals reviewed source (local main only), and all 75 command
permission identifiers occur in both native libraries.

arm64-v8a: 482823920 bytes; SHA-256
`296893d8fc7574778c6e7c039330e0367390c214d0c9707e079c254abe2e871d`.
x86_64: 491267029 bytes; SHA-256
`0c3f0b83f920c0e21e3749dfae5c0b5b6037bd2f066536a6154839faf81c2c07`.

Artifacts/metadata/checksums/instructions are refreshed in `.lab/packages/android/`.
Log `.lab/android-toolchain/power-delivery-build.log`. Exact checksums, command
permission drift and scoped diff review pass; no unrelated application tests rerun.
The unchanged Linux server archive still hashes to
`72d3c5fef4cf58fab9c7fd865719b4d65cc19b290ce95306b8036eaa7a3aaedb`.

Limits: compilation/content only for these APKs. Metadata keeps releaseCandidate=false
and runtimeValidatedForTheseApks=false; earlier Android runtime evidence does not
transfer. The latest feature's actual Linux native evidence remains separately
recorded. Exact Android runtime, real provider/guest/deployment and release gates
remain open. Next source slice: retained definition/detail inspection for an existing
remote instance, binding its exact YAML/hash/identity to the selected row without
claiming live resources or SSH readiness. Pending emulator/real-target approvals
are unchanged.


## 2026-10-21 — Native retained remote instance detail

Scope: one fixed stored-instance GET command, no server/schema/dependency change.
Reuses lifecycle Target and the existing Environment read path. Selected instance/
provider scope/node/VMID/name are validated before transport and against detail.
Registered and archived associations are supported. Positive revision, nonempty
YAML up to 64 KiB UTF-8 bytes and exact SHA-256/hash equality are required. Only
typed public detail fields return; original YAML bytes are preserved and damaged
content is not echoed in errors. No provider read, plan, consent or mutation is
created by this detail command. Generic saved-instance read errors now describe
that read rather than lifecycle prerequisites.

Verification: five focused native generated-HTTPS tests pass for active/archived
exact path/YAML/hash/revision, metadata/target/status/content mismatch, byte bounds
including multibyte text, input rejection before network, 401/429/404/503/missing
marker/size refusal without private content echo, extra-field stripping and stale
disconnect. Log `.lab/native-deps/remote-instance-detail-check.log`. Native library
compilation, 76-command main-only grant drift, changed Rust formatting and diff
review pass; no unrelated suites repeated. No VM/credential/real-provider access.

Limits: native verifies stored byte/metadata consistency, not YAML schema validity
independently, current resource equality, current archived-VM absence, ownership
or guest/SSH readiness. UI and actual native-window/Android integration are still
open. Existing delivered APKs remain unchanged snapshots preceding this in-progress
feature; server archive is unaffected. Next slice: explicit saved-detail/YAML UI,
escaped literal display, revision/hash and desired-versus-observed distinction;
then native integration before Android refresh. Pending target approvals remain.


## 2026-10-21 — Retained definition inspection UI

Scope: RemoteInstanceDetail for registered and archived remote rows, explicit read/
refresh/hide. Displays definition revision/hash, saved association state/target and
literal YAML. React text children prevent HTML-looking YAML from becoming markup;
code wraps and scrolls within a bounded height. Desired declaration is explicitly
separate from observed current resources/existence/guest/SSH state. No editing,
consent, submission or SSH flow is attached to the panel.

UI guards selection/metadata/status/revision/byte bounds. Exact YAML SHA-256 checking
remains native. New reads clear previous evidence; errors, target/association rebind
or connection cleanup discard stale results. Hide clears local content only. Loading
a workspace does not fetch detail automatically. No new dependency or server change.

Verification: frontend build and --remote-detail real React/Firefox simulated-IPC
scenario pass for active/archived reads, literal HTML-looking YAML, revision/hash,
desired/observed qualification, failed refresh, target/revision/hash-shape refusal,
workspace rebind/reconnect suppression, hide and no mutation. Logs:
`.lab/remote-detail-ui-build.log`, `.lab/remote-detail-browser-check.log`.
Adjacent --remote-power and --recreation-recovery also pass for shared row changes;
logs `.lab/remote-detail-parent-power-check.log` and
`.lab/remote-detail-parent-history-check.log`.
JS syntax/diff review is scoped; no unrelated native/server/Android suites repeated.

Limits: mock IPC does not prove this UI's actual native command/hash/server path.
APK snapshots unchanged and predate this in-progress feature; server code/archive
unaffected. Next slice: actual native/server detail inspection, exact retained YAML/
hash/revision and no provider access, including archived/no-provider contexts where
appropriate. Then Android delivery refresh. Pending external target approvals remain.


## 2026-10-21 — Actual native retained-definition inspection

Scope: production detail command in the owned native example and shared acceptance
helper invoked by lifecycle/recreation second-app recovery. Adds a public instance-ID
DOM attribute to scope selection when archived/new associations reuse a VMID. No
server/dependency/VM/credential change; frontend rebuilt for that attribute.

Each second actual app runs against the restarted real server with provider/catalog
absent and five write flags false. The helper explicitly reads and refreshes the
selected registered association (lifecycle) or archived source (recreation), checks
literal YAML bytes/hash/revision/name/status against the original stored record,
verifies desired-versus-observed wording and absence of consent/SSH, then hides it.
Production native SHA-256 validation runs on the actual server response, not IPC
mocks. GET-only recovery, unchanged full operation/instance/definition and provider
read/task/power/write counters still pass. Drivers are confined to the owned example.

Verification: frontend and embedded native example builds pass; both augmented
actual --lifecycle-recovery and --recreation-recovery pass. Logs:
`.lab/native-deps/instance-detail-frontend-build.log`,
`.lab/native-deps/instance-detail-native-build.log`,
`.lab/native-deps/instance-detail-active-ui-check.log`,
`.lab/native-deps/instance-detail-archived-ui-check.log`. JS/Python syntax, example
formatting and diff review pass; no unrelated suites repeated. All owned processes
and temporary state cleaned up; TLS/SSH policies and WebKit sandbox unchanged.

Limits: isolated Linux/restarted-service/generated infrastructure, not Android or
real VM/guest resources/ownership. Existing delivered APKs unchanged and predate this
completed client feature; server archive unaffected. Next slice: rebuild/package
both Android targets with exact artifact checks, without rebuilding the unchanged
server. Earlier emulator/real-target approvals remain pending.


## 2026-10-21 — Android delivery includes retained definition inspection

Scope: rebuild/package Android ARM64/x86_64 after the completed definition/detail
feature. No server build, new behavior/dependency, VM/emulator operation, credential
access or app installation. `make package-android` succeeds with the owned toolchain.
Signatures/shared signer, manifests/ABI/SDK, backup/sharing policy and exact embedded
native-library hashes pass. Generated local-main capability matches source; all 76
command permission identifiers are present in both target libraries.

arm64-v8a: 483360840 bytes; SHA-256
`db832f81e5e9bec225485d572c66fe0d548f0323b8617ee62419c0cf9ac4c7ed`.
x86_64: 491610029 bytes; SHA-256
`ce49dbdea95d0d2394f0f17fc5f2008efcc110a090b4334dfa1d45edf0214d83`.

Artifacts/public metadata/checksums/instructions are refreshed in `.lab/packages/android/`.
Log `.lab/android-toolchain/detail-delivery-build.log`. Checksums, command-grant drift
and diff review pass; no unrelated tests rerun. The unchanged server archive still
has SHA-256 `72d3c5fef4cf58fab9c7fd865719b4d65cc19b290ce95306b8036eaa7a3aaedb`.
Metadata remains releaseCandidate=false and runtimeValidatedForTheseApks=false;
compilation/content is not fresh Android runtime proof. Earlier Linux native/server
inspection evidence is separately recorded.

Next source slice: allow explicit SSH preparation from an existing registered remote
row using a manually supplied reachable address/account and the existing strict
host-trust flow. Do not infer guest address/identity/readiness from power or saved
YAML, and do not auto-connect or unlock an identity. Real target and exact-current
Android acceptance still await the earlier pending concrete approvals.


## 2026-10-21 — Manual SSH entry for existing remote associations

Scope: existing registered-row Prepare SSH control and selected-association context
in RemoteConnection, reusing SshHostTrust. No native/server/dependency change.
Endpoint/account begin empty; port defaults to editable 22. The UI explicitly says
address/readiness are not discovered from power/YAML and the association does not
verify the operator-supplied endpoint. Selection does not inspect, trust, unlock,
authenticate or open a shell. Existing endpoint-scoped strict host trust is required.

Manual reselection creates a fresh trust component/context. Existing cleanup cancels
pending challenges and discards late replies; prior account/auth state is not reused.
A successful workspace refresh clearing/archiving/rebinding the selected row resets
manual context. Explicit disconnect/reconnect resets it. A failed workspace read
alone does not close a direct SSH context. Mutation-review locks disable selection.
Creation/recreation handoffs retain their existing prefilled ready-guest path.

Verification: frontend build and --existing-ssh real React/Firefox simulated-IPC
flow pass for registered-only entry, empty endpoint/account, explicit port, no auto
handshake/login, invalid URL, independent unknown-key confirmation/fresh inspection,
changed-key refusal, reselection/stale cancellation, archival cleanup and reconnect.
No identity unlock, authentication or session is invoked by this preparation test.
Logs `.lab/existing-ssh-ui-build.log`, `.lab/existing-ssh-browser-check.log`.
--remote-connection and --recreation handoff scenarios also pass for shared target
state/rendering; logs `.lab/existing-ssh-create-handoff-check.log` and
`.lab/existing-ssh-recreation-handoff-check.log`. JS syntax/diff review is scoped here; no unrelated native/server/
Android suites rerun. ADR 0078 records trust/context semantics.

Limits: simulated IPC only; no actual native host handshake, guest or Android proof
for this entry. Existing artifacts unchanged and predate the feature. Next slice:
actual native manual host-trust UI against an owned generated SSH peer, with no
personal keys/agent, no VM, no automatic login and explicit fingerprint consent.
Then appropriate full terminal/Android delivery work; earlier concrete approvals
remain pending.


## 2026-10-21 — Actual existing-instance manual native host trust

Scope: --manual-ssh owned native fixture, production SSH host command wiring and
manual preparation driver. No product/server/dependency change. The example includes
the existing generated SSH fixture with PTY/OS shell disabled, an independently
supplied fingerprint and fresh temporary trust directory. No agent or identity/
authentication/session handlers are registered. Production capability queries replace
older example stubs; Linux agent support remains false. Other modes use the owned
XDG application path for their unused host manager.

Actual React remote-row selection begins with blank endpoint/account, uses the
explicit loopback host/port and production inspect/confirm/cancel commands. Unknown
key requires checkbox confirmation; a fresh inspection enables account selection;
owned key rotation then produces changed-key refusal with no trust/login control.
Peer auth/channel/event counters are zero, all probes close and the saved public
trust bytes equal their pre-rotation bytes. Generated peer/temp trust are shut down/
removed. Provider read/task/write counts remain zero; no real VM, personal key,
agent socket or credential configuration is accessed.

Verification: native example builds and actual --manual-ssh passes; logs
`.lab/native-deps/manual-ssh-native-build.log`, `.lab/native-deps/manual-ssh-ui-check.log`.
Shared --recreation-recovery also passes for the changed native launcher/capability
wiring; log `.lab/native-deps/manual-ssh-shared-recovery-check.log`. JS/Python syntax, example formatting and diff review are scoped here; no
unrelated suites. Build setup initially exposed a misplaced module doc comment and
Tauri macro collisions with old capability stubs; both were resolved by restoring
the doc position and using production capability functions before the passing run.

Limits: Linux native host preparation only, no login/terminal/real VM/Android proof.
Current APKs unchanged and predate the existing-instance entry. Next slice: owned
native SSH authentication/terminal integration through that entry, with strict trust,
generated temporary identity/agent only and no personal files. Real-target/emulator
approvals remain pending; no automatic continuation grants them.


## 2026-10-21 — Existing-instance native application-key terminal

Scope: extend the owned --manual-ssh driver through existing production identity/
session commands. No product dependency, API or VM change. Fresh encrypted app
identity and trust live in an owned temporary directory; only its generated public
key is authorized for the generated peer's fixture account. An explicit nonexistent
owned agent socket prevents personal agent discovery. Account/passphrase/consent/
identity selection remain explicit in React, followed by actual native PTY/OS shell.

Verification: targeted native example build and --manual-ssh pass. xterm input
executes a command and displays its stdout; stty rows match resized renderer;
terminal disconnect closes connections. Peer asserts one channel and PTY/shell/
resize events. Subsequent key rotation is refused and saved trust bytes remain
unchanged. Provider reads/tasks/writes remain zero. Logs:
`.lab/native-deps/manual-ssh-terminal-build.log`,
`.lab/native-deps/manual-ssh-terminal-check.log`.
The initial run clicked the workspace Disconnect because the driver selected the
first label match; its selector now targets the terminal and the rerun passes.
Earlier verified dirty-worktree slices were retained. No unrelated suites rerun.

Limits: Linux owned endpoint only, not actual guest/Android acceptance. Android
artifact refresh now passes as recorded below.
Next: exact-current Android execution after existing concrete emulator approval;
otherwise advance declared provisioning source work. Full roadmap remains open.


## 2026-10-21 — Android delivery includes existing-instance manual SSH

Scope: rebuild ARM64/x86_64 debug APKs to include the current registered-row manual
SSH entry; no device installation, emulator start or real VM mutation. Existing
native terminal integration above passes before delivery. `make package-android`
passes frontend typecheck/build, both native target builds and Gradle packaging;
packager verifies signatures, manifest policies, ABI and exact embedded native
library hashes. Log `.lab/android-toolchain/manual-ssh-package.log`.

Current files under `.lab/packages/android/`:
- `vmanyware-0.1.0-android-arm64-v8a-debug.apk` SHA-256 `c9aa77f8b6fe5c1750f8cdfbcf2f486c22cceb09534dee5fdf46b3c47ef9dbb1`.
- `vmanyware-0.1.0-android-x86_64-debug.apk` SHA-256 `67f68cd5ab36e12345a32f965456e84c5729eb517f6ec399c363c69590123eeb`.

`artifacts.json`, SHA256SUMS and INSTALL.md are refreshed. These are debug test
artifacts: releaseCandidate=false and runtimeValidatedForTheseApks=false. Earlier
Android execution cannot validate these new files. The packaged Linux server is
unchanged. No unrelated server suites rerun for this client-only delivery.

Next task: execute this exact APK on the previously proposed owned Android target
after its concrete VM-operation approval. While that external gate remains open,
implement the declared package provisioning/post-provision snapshot workflow.


## 2026-10-21 — Declared package provider transport

Scope: fixed Debian package request/dispatch/PID observation using the existing
clone/readiness target guard. No API admission or worker behavior changes yet;
nonempty packages and snapshotAfterProvisioning remain explicitly rejected.
No dependency, VM, personal credential or host package mutation was introduced.

The request validates bounded unique package declarations, refuses option/shell/
removal selectors, and keeps every declaration in a separate positional argument.
A fixed timeout/noninteractive apt update/install script forbids removals and
checks literal installed state/exact requested versions with dpkg-query. Only the
generated guest would execute it. Raw package output is suppressed. Observations
return typed pending/completed/failed and verify marker/digest/power/no-pending
before and after. Dispatch never retries; caller durability remains mandatory.
ADR 0079 specifies the required journal/deadline/PID and final-snapshot integration.

Next task: implement the durable package stage and wire it into the provisioning
worker, with one dispatch, retained PID/deadline, uncertain-response/restart behavior
and explicit failure. Then integrate optional snapshot and open admission only
for the fully supported workflow. Android exact-artifact and real-guest gates stay
open; no full roadmap completion is claimed.

Verification for this package slice: `cargo test --locked -p vmanyware-server
proxmox::tests::packages --lib` passes all five tests; log
`.lab/packages-adapter-tests.log`. Tests check validated declarations, fixed bounded
POST body and read-only shell syntax, scoped PID/status, clean completion/failure,
malformed/leaking replies, identity mismatch and one POST after lost reply. Initial
command assertion had a wrong argv index; corrected to inspect the script argument.
The quote escaping was reviewed before the passing run. Changed files pass rustfmt
and diff whitespace review. The library compilation covers affected callers; no
unrelated suites or artifact rebuilds run for this inactive provider-stage addition.
No actual guest installation has been demonstrated yet.


## 2026-10-21 — Durable declared-package worker

Scope: package stage claim/dispatch/PID/read/finish in the sequential production
worker and SQLite journal. No API admission, snapshot, client, dependency or real VM
change. Older journals omit the new optional packages field and still deserialize.
Current public admission still rejects nonempty packages and final snapshot requests.

The first claim retains startedAt/deadline=startedAt+960. First submission requires
creation enabled, current matching catalog/approval and same provider scope. Its
stage marker is committed before the single fixed POST, and PID is retained before
status reads. Known-PID restart recovery reads only, without catalog/write flags.
Marker-without-PID never resubmits or contacts the provider; lost replies remain
uncertain through the fixed deadline. Timeout stays uncertain and stops automatic
polling. Nonzero/signal exits fail explicitly; only confirmed success stores the
scoped installed/packageCount proof. The operation completes only after the last
planned stage; pending final snapshots cannot be skipped. Saved readiness is never
replaced by a mere process-accepted response. Reader validation rejects malformed
PID/deadline/proof, and transactional updates prevent PID rebinding/forged completion.

Verification: all six package worker tests pass in
`.lab/packages-worker-tests.log`; two affected durable readiness tests pass in
`.lab/packages-affected-readiness-tests.log`; the affected seed-regeneration restart
test passes in `.lab/packages-affected-seed-tests.log`. The earlier five package
transport tests also passed during this slice's initial library compilation
(`.lab/packages-worker-compile.log`). No unrelated suite or artifact rebuild ran.
New tests initially referred to Failure as a wrapper rather than the existing enum;
that compile error was corrected before the passing runs. Scoped formatting and
diff review pass. Earlier dirty-worktree changes are retained.

Limitations: owned HTTPS/SQLite evidence only; public API/native workflow activation,
optional snapshot, actual guest packages and current Android execution remain open.
Next task: implement the optional post-provision snapshot using a retained one-time
submission/task journal and independent snapshot gate, then integrate API/client
admission/result validation for the complete supported package/snapshot workflow.
Full roadmap, real-target and platform gates remain open.


## 2026-10-21 — Optional final provisioning snapshot worker

Scope: final provisioning snapshot stage/storage/executor and independent gate
propagation through the sequential production queue. No public admission, client,
dependency, schema or real infrastructure change. YAML declares the stage already;
it now waits for all earlier stages and valid guest/package evidence. Names and
metadata bind the exact operation/definition/plan. Both creation/snapshot gates and
current catalog/approval are required only before first dispatch. Existing scoped
preflight verifies storage/name/tree/marker/digest/pending state.

The snapshot marker and digest are committed before one fixed disk-only POST; only
a matching qmsnapshot task can be linked. The optional private seed snapshotDigest
field retains the initial digest without changing the resource-only SQL column.
One task read occurs per round. Known task restart reconciliation is GET-only with
write flags off and catalog absent. Unknown/lost task replies never redispatch.
Success requires completed task plus exact disk-only snapshot metadata and valid
retained VM identity, and only then completes provisioning. Reader corruption and
forged completion are rejected. Existing readiness result is retained for consumers.
ADR 0080 records consistency/backup and external acceptance limits.

Verification: six new scoped tests pass in `.lab/provisioning-snapshot-tests.log`.
Additional digest/evidence corruption assertions pass in the changed completion
test only (`.lab/provisioning-snapshot-integrity.log`). Six affected package-worker
tests pass (`.lab/provisioning-snapshot-affected-packages.log`); existing confirmed
create API/worker/restart journey passes (`.lab/provisioning-snapshot-affected-create.log`);
standalone snapshot queue scheduling/gate test passes
(`.lab/provisioning-snapshot-affected-scheduling.log`). The latter checks are warranted
by shared reader/seed journal and worker gate propagation. Scoped formatting and
diff review pass. No unrelated suite or artifact rebuild ran. Earlier dirty slices
remain untouched except the required shared journal/worker wiring. The first run
exposed the SQL resource-only digest constraint; the implementation was corrected
to use the private journal before the passing runs. Existing fixture helpers now
accept explicit YAML and keep their default behavior; snapshot test operation IDs
are normalized in owned SQLite before claims/dispatch for exact metadata assertions.

Limits: source/owned-provider evidence only. Current public admission still rejects
nonempty packages and snapshotAfterProvisioning=true. Current APK/server artifacts
predate these inactive additions. Real guest installation/snapshot and exact-current
Android execution remain open. Next task: integrate API preflight/admission and
native create/recreate/recovery validation/UI for the full declared package/snapshot
workflow, then prove its authenticated end-to-end generated-provider path. Keep the
full roadmap and concrete external approval gates open.


## 2026-10-21 — Declared workflow API admission and real initial tag

Scope: creation/recreation admission/plan capability, shared package validation and
fresh clone resource tag, with complete API/worker/generated-provider journeys.
Supported declarations are admitted; snapshotAfterProvisioning additionally needs
the independent snapshot flag or returns 403 without reservation. Invalid package
selectors return 422, and image-default user attestation stays required. Enqueue
still does not contact Proxmox. No client/schema/dependency/real VM change.

The integration exposed a real source mismatch: creation stored the description
marker but strict lifecycle/snapshot reads require the instance tag. The existing
stopped-clone/digest-bound resource PUT now sets/verifies vmanyware-<instance-id>.
This replaces inherited tags only on that freshly created clone, adds no separate
write and does not repair/replay existing uncertain jobs. Generated fixtures now
record the field actually submitted rather than fabricating lifecycle identity.
ADR 0081 records behavior and real ACL/old uncertain-job limitations.

Verification: two bearer-protected API/worker HTTPS tests exercise twelve declared
workflows (create/recreate × package-only/snapshot-only/both × disks 8/40GiB), all
passing with complete stages/proofs, mutation counts, restart, same-key retry and
retained-source recovery. Log `.lab/declared-provisioning-flow.log`. Six create and
seven recreation API checks pass (`.lab/declared-provisioning-admission.log`),
including explicit plan capability/403-without-reservation. Seven affected resource
checks pass, including absent/foreign tags (`.lab/declared-provisioning-resources.log`).
Two existing no-feature API/worker journeys pass
(`.lab/declared-provisioning-existing-flow.log`). The first admission run had an old
422 expectation for snapshot-disabled; it now correctly expects 403 before passing.
Checks were broadened only for the shared resource-write contract/API handlers.
Earlier dirty work is retained; no unrelated suites or artifact rebuilds ran.

Limits: generated-provider PID/exit/snapshot evidence, not actual guest apt or real
storage/restore proof. Native create/recreate plan validation and the create form
still restrict these features. Current packaged server/APKs predate this source.
Next task: native/UI integration of declared packages and final snapshot, including
review/result/recovery validation and a complete native generated-provider journey.
Keep actual guest/Android/platform acceptance and concrete VM-operation approvals open.

The five focused package transport/validation checks also pass after admission
reused their validator (`.lab/declared-provisioning-package-validation.log`).
Generated provider JS syntax, scoped Rust formatting and diff whitespace review pass.


## 2026-10-21 — User delivery/checkpoint request

The user authorized committing/pushing accumulated roadmap work and updating the
README and public project status in ../devutils-org.github.io. This explicit request
overrides the usual no-sibling-edit restriction only for that website repository.
The user also authorized continued autonomous source/artifact work; the home Proxmox
host is unavailable for now. No real VM/production/credential operation is inferred.

README/public copy now distinguish implemented provider/native paths, API-only
package/final-snapshot support, debug artifacts and actual infrastructure/platform
acceptance. Next implementation task remains native package/final-snapshot support.
Commit/push results will be reported after the actual commands complete.

Checkpoint verification: staged source passes diff whitespace review; ignored lab,
SDK, artifacts and generated permission files are excluded. Private-key material
scan found only the explicit invalid key rejection fixture (literal "fixture"),
not a private key. Website synchronization incorporates upstream TypeScript/French
changes instead of replacing them. Its current build/type/targeted-render results
and actual push outcomes are recorded in the session delivery report.


## 2026-10-21 — Published source and website checkpoint

Accumulated source/docs were committed as 8cc1594 and pushed successfully to main.
GitHub redirected the former moha-bekh remote to devutils-org/VMAnyware; origin now
uses that canonical HTTPS URL. Existing gh HTTPS authentication handled the push;
no credential file or SSH verification policy was changed.

The website was fast-forwarded to its newer TypeScript/bilingual baseline, and
concurrent upstream changes were incorporated before pushing its VMAnyware-only
status commit 50abc4a. English/French landing pages, tool summaries and site README
now distinguish source/API/native evidence from actual guest/platform/release gates.
Website typecheck, build/prerender and 12 targeted site/render tests pass. Logs:
/tmp/vmanyware-site-status-types.log, /tmp/vmanyware-site-status-build.log and
/tmp/vmanyware-site-status-tests.log. The type/layout issues from transferring the
old JS page were corrected against current Page types and language-parity tests.
Generated output and node_modules were not committed. No unrelated application
suite reran for these documentation/public-copy edits. Website deployment result is
separate from successful Git push and is reported from its workflow status.

Next implementation: client-native plan validation and explicit create/recreate
review/form support for declared packages/final snapshots, followed by owned native
integration and refreshed APKs. The full roadmap remains active; real home Proxmox
acceptance awaits the user's future server. No real VM operation occurred here.


## 2026-10-21 — Native declared-plan validation

Scope: native Plan acceptance shared by remote create/recreate/recovery; no UI,
server production behavior, runtime launch, dependency or infrastructure change.
Packages now follow the same bounded/unique grammar as server transport; final
snapshot intent is accepted and retained. Shared fixture corpus locks both sides
without adding server dependencies to the client. Exact comparison includes
package order/names/versions and snapshot intent; runtime capability alone may
change independently. Unknown-response retry preserves the retained private plan,
hashed approval and identical request key/body. Existing scope/readiness checks
are unchanged. ADR 0082 documents source and acceptance boundaries.

Verification: 13 native create/prepare/recovery tests pass (three new), log
`.lab/native-deps/declared-native-plan-tests.log`. Twenty affected native recreation/
recovery checks pass (`.lab/native-deps/declared-native-affected-recreation.log`).
One server contract check passes against the same 17-case corpus
(`.lab/native-deps/declared-native-server-contract.log`). Rust formatting and diff
review are scoped to the changed files. No unrelated suites, app launch or artifact
rebuild were needed. Earlier checkpoint remains committed/pushed; this is its next
reviewable source slice.

Limits: the current React create form still submits empty packages/no final
snapshot; richer review/results and full native declared workflow need integration.
Current APK/server artifacts and real target/platform gates remain separate.
Next task: explicit package inputs and final-snapshot option in RemoteCreate, with
exact plan summaries in creation/recreation, stale-consent invalidation and scoped
browser acceptance; then native generated-provider integration and refreshed APKs.


## 2026-10-21 — Declared package/final-snapshot inputs and reviews

Scope: RemoteCreate inputs/exact returned-feature check/review, retained recreation
feature summary and the affected UI drivers. No new native/server/dependency/VM
change in this slice; prior native-plan changes remain separately verified.
Package input begins empty and final snapshot unchecked. One declaration per line
preserves order/versions, trims surrounding whitespace and ignores blanks. Reviews
show the exact returned plan. A changed package sequence/snapshot intent is rejected
before consent. Existing fieldset locks prevent editing while prepared/submitted;
Edit definition clears review/consent and a fresh preview uses a new request key.
Unknown submission still allows only the same retained confirmed request. A stable
create-confirm ID separates consent from the new option. Recreation explicitly
shows retained packages/new snapshot without implying old disk/snapshot recovery.
ADR 0083 records boundaries and consistency/dependency wording.

Verification: frontend typecheck/build passes (`.lab/declared-ui-build.log`). New
--declared-create React/Firefox simulated-IPC flow passes
(`.lab/declared-ui-browser.log`), including changed server declaration rejection,
exact versions/intent, defaults, edit/key/consent invalidation, lock and same-key
retry plus existing status/recovery/SSH-trust assertions. The launcher checks that
the declared scenario was active. Existing --remote-connection and --recreation
pass (`.lab/declared-ui-default-create.log`, `.lab/declared-ui-recreation.log`), the
latter also checking retained feature summaries. The initial test's disabled
property check ignored inherited fieldset state; it was corrected to :disabled
before passing. JS syntax and diff review pass for changed drivers. No unrelated
native/server suite reran; Android driver's updated consent selector is syntax-only.

Limitations: browser IPC is simulated, not a joined native declared workflow or
actual guest package/snapshot execution. Current packaged artifacts predate this UI.
Next task: exercise actual native create/recreate with declared package/final
snapshot through the authenticated control plane and owned generated provider,
including full stage result and no duplicate mutations; then refresh deliverables.
Full real-target/platform roadmap remains open; no real VM was modified.


## 2026-10-21 — Joined native declared creation and recreation

Scope: --declared-create/--declared-recreation owned native acceptance, example-only
production create-command wiring, generated provider optional declared mode and
SQLite/submission evidence. No product dependency or real infrastructure mutation.
The real React/WebKit form/review calls production retained native commands over
verified HTTPS/bearer to the actual server/worker. Each workflow explicitly reviews
build-essential plus git=1:2.0-1 and final disk-only snapshot, requires consent,
loses its first accepted reply and retries only the identical path/key/body.

Generated provider asserts seed/start/Cloud-Init observation before package dispatch,
exact bounded package command/declarations, package completion observation before
snapshot and exact operation/definition/plan snapshot metadata. Only eight expected
non-GET provider requests occur (including the read-only Cloud-Init exec), with no
duplicate clone/config/seed/start/package/snapshot. SQLite all-stage success,
scoped installed/count proof, final disk-only snapshot, matching definition/ready
instance and unchanged retired source are required. Recreation links source lineage;
creation does not. SSH handoff is explicit and still demands independent host trust.
No guest login or actual apt process is part of these controlled provider tests.

Verification: native example and current debug server compile, logs
`.lab/native-deps/declared-workflow-native-build.log` and
`.lab/native-deps/declared-workflow-server-build.log`. --declared-create passes
(`.lab/native-deps/declared-create-native-check.log`); --declared-recreation passes
(`.lab/native-deps/declared-recreation-native-check.log`). Shared provider/example/
runner changes warrant the existing --recreation-recovery check; it passes through
app/server restart and provider/catalog/write-off GET-only recovery with unchanged
counters (`.lab/native-deps/declared-workflow-default-recovery-check.log`). Scoped
JS/Python syntax, Rust example formatting and diff review pass. No unrelated suites
rerun. Earlier native/UI source slices remain dirty and retain their recorded checks.

Limits: generated HTTPS/guest results, not actual Proxmox, Debian apt execution,
snapshot guest-state restoration or current Android runtime. Current packaged
server/APKs predate this source. Next task: rebuild the Linux archive and both
Android debug APKs from the integrated source, verify exact packaged content and
server behavior, then pursue Android/real-target/platform acceptance within concrete
approval/environment boundaries. Full roadmap remains active.


## 2026-10-21 — Integrated declared-workflow delivery artifacts

Scope: build/verify the Linux release archive and both Android debug APKs for the
now-integrated native package/final-snapshot workflow. No host service installation,
emulator/device operation, VM or credential change. Android install guidance now
covers exact review, independent snapshot permission, all-stage completion,
uncertain-operation recovery and fresh-instance versus old-snapshot semantics.
The companion INSTALL.md copy matches the updated guide exactly.

Verification: make package-server passes optimized compilation/allowlisted archive
creation (`.lab/declared-delivery-server-build.log`). The exact archive's staged
process passes checksums/staging/reinstall preservation, unit syntax, authenticated
startup, auth refusal, restart/persistence, admission/refill, stopped backup/restore,
history and scoped lifecycle route checks
(`.lab/declared-delivery-server-check.log`). No host unit was started. Existing
schema-specific migration checks were not repeated: this diff has no schema change;
this packaged-process check and recorded source/native stage proofs cover delivery.
The declared native workflows remain independently proven against the debug server,
not relabelled as a new full workflow test of this release archive.

make package-android passes frontend typecheck/build, ARM64/x86_64 native builds,
Gradle, signatures, compiled-manifest policies, ABI and exact current embedded
native-library hashes (`.lab/android-toolchain/declared-workflow-package.log`).
SHA256SUMS, artifacts.json and installation instructions are refreshed. These are
releaseCandidate=false and runtimeValidatedForTheseApks=false artifacts; historical
Android runtime does not validate these hashes. No unrelated application tests ran.

Current deliverables:
- `.lab/packages/vmanyware-server-0.1.0-linux-x86_64.tar.gz` SHA-256 `543c26c7b63971d0aff6bd42bac14690dabd45df202cc85a5c4a6e15074b89a2`.
- `.lab/packages/android/vmanyware-0.1.0-android-arm64-v8a-debug.apk` SHA-256 `81a353932fbfc9f63dad3d17cfc93cb098c9a0a27f11912ef053c4fcdedf3de4`.
- `.lab/packages/android/vmanyware-0.1.0-android-x86_64-debug.apk` SHA-256 `82a878f751110aecc1749d70897664961c5a8390c91ecc493d69acd2e4593dc2`.

Limits: current Android execution, actual home Proxmox/guest packages/restored state,
real deployment and platform/OS-storage acceptance remain open. The roadmap is not
complete. Next task: expose safe, actionable provisioning failure/step evidence in
native create/recreate/recovery, so package/snapshot failures are distinguishable
while preserving retained operation identity and no-replay behavior. Exact-APK
execution remains available only within its concrete target approval boundary.


## 2026-10-21 — Safe provisioning failure reasons in native/UI recovery

Scope: typed optional provisioning failure on the native Operation shared by
creation/recreation/recovery, and fixed local messages in their four React views.
No server/schema/dependency/VM mutation. Native permits only known relevant codes,
strips raw extra fields, rejects unrelated/unknown/object failures and contradictory
healthy failure/non-success readiness. Older missing-code failures remain readable.
The renderer never prints a raw unknown code or provider/guest message. Packages
failed/unconfirmed/timed out and final snapshot disabled/precondition/state reasons
are distinct. Timeout remains uncertain; snapshot task completion does not establish
metadata proof. No extra mutation, restored consent, blind retry or failure SSH path
is introduced. ADR 0084 records compatibility and private-journal boundaries.

Verification: 15 native create/prepare/recovery tests pass (two new) in
`.lab/native-deps/provisioning-failure-native-tests.log`, including generated-HTTPS
GET forwarding only the safe code while retaining operation ID. Twenty affected
native recreation/recovery tests pass
(`.lab/native-deps/provisioning-failure-affected-recreation.log`). Frontend build/
typecheck passes (`.lab/provisioning-failure-ui-build.log`). Three scoped React/
Firefox simulated-IPC journeys pass: declared creation plus saved creation recovery
(`.lab/provisioning-failure-create-ui.log`), recreation
(`.lab/provisioning-failure-recreation-ui.log`) and saved recreation
(`.lab/provisioning-failure-recovery-ui.log`). Assertions cover safe package/snapshot
reasons, unknown-code fallback, redaction, no false readiness/SSH and unchanged
request counts/IDs. Scoped syntax/format/diff review pass. No unrelated server suite,
actual native launch or APK rebuild reran for this slice.

Limits: separate native protocol/browser evidence, not new joined failure injection
or exact-APK runtime. Current APKs include the complete declared workflow but predate
these richer messages; the packaged server is unchanged. Earlier dirty slices keep
their recorded verification. Next task: read-only safe per-stage provisioning
progress and history evidence, preserving target/plan/operation binding and hiding
private approval, task/PID and seed data; then refresh client artifacts in a batch.
Full real home Proxmox/platform/OS-storage/release roadmap remains open.


## 2026-10-21 — Safe recorded provisioning progress (ADR 0085)

Scope/acceptance: authenticated GET-only ordered provisioning stage evidence,
retained operation/target/plan/source binding, no provider access or write flags,
no private journal exposure or mutation. Implemented the bounded public projection
and one-transaction Store read at `/api/v1/operations/{id}/provisioning`. Existing
strict job decoding plus sequential prefix/status checks reject corrupt data with
a fixed 503; malformed, absent and non-provisioning IDs have a fixed 404.
`submissionStarted` is historical dispatch evidence, including read-only probes;
it does not establish completion, current power/readiness or SSH trust.

Verification: five focused HTTP/SQLite tests pass in
`.lab/provisioning-progress-api-tests.log`, covering authentication/method/identity,
all eight ordered stages, failure/prefix, restart uncertainty, corruption refusal,
redaction and unchanged operation records. Two targeted generated-HTTPS workflow
tests pass across 12 create/recreate × package/snapshot × disk combinations in
`.lab/provisioning-progress-declared-flow-tests.log`; completed progress is read
through a provider/catalog/write-free router with correct lineage and hashes.
Scoped rustfmt and diff review pass. No unrelated suite or artifact rebuild ran.

Website status now reflects ADRs 0082–0084 in English/French, including joined Linux
native declared workflows, safe failure messages and refreshed debug artifacts.
Its build/typecheck and 12 focused site/render checks pass in
`.lab/declared-status-site-{build,tests}.log`. Website commit `4e0e5a5` is pushed.

Limits: progress is not yet wired into native commands/UI. Current packaged server
predates this endpoint, and current APKs predate richer failure messages; their
recorded packaging evidence does not cover these latest changes. No real VM,
Android runtime, host deployment or OS-backed key storage was validated here.
Next task: one native/client recorded-progress slice, preserving operation/target/
plan/source binding and historical wording, followed by a batched delivery refresh.


## 2026-10-21 — Native/client recorded provisioning stages (ADR 0086)

Scope: fixed read-only native progress reader and explicit stage display across
creation, recreation and both recovery views. Acceptance: operation/target/plan/
source binding, ordered typed stages, private fields discarded, no execution or
restored consent, historical wording and late-response suppression.

Implemented one command with explicit production handler/manifest/local-main ACL
(total 77). Existing strict GET-only recovery reconstructs operation/environment;
the progress GET must match identity, hashes/scope/target, status and failure, plus
successful readiness plan. Known mandatory/optional stage order, consecutive
positions and sequential status prefix are checked. A racing worker transition
requires an explicit read refresh; nothing redispatches. Four views use the shared
component, match known plan/source/target and clear/discard obsolete results. Raw
read errors get fixed text; progress does not grant SSH or replace operation status.

Verification so far: three new native projection/HTTPS tests pass
(`.lab/native-deps/provisioning-progress-native-tests.log`) and three affected
GET-only recovery tests pass (`provisioning-progress-affected-recovery.log` in that
directory). Frontend typecheck/build passes (`.lab/provisioning-progress-ui-build.log`).
Three scoped real React/Firefox simulated-IPC journeys pass
(`.lab/provisioning-progress-{create,recreation,recovery}-ui.log`), covering stage
ordering, plan mismatch, error redaction, stale response, source/target matching,
no automatic reads and no progress-based SSH. Scoped format/syntax/diff and command
ACL review pass. Current server and native example compile; after correcting the
example build to use `--features tauri/custom-protocol`, joined Linux native declared
creation also passes with explicit package/final-snapshot stage reads
(`.lab/native-deps/provisioning-progress-create-native.log`). The initial two
launches omitted that feature and timed out before any create/provider operation;
they are not counted as acceptance. Recreation/delivery checks follow below.


ADR 0086 completion: joined Linux native declared recreation also passes, including
explicit succeeded package/snapshot stage display
(`.lab/native-deps/provisioning-progress-recreation-native.log`). Both native
workflows retain exact same-key lost-reply retry, expected provider calls and
independent SSH trust handoff; progress introduces no provider requests.

Batched delivery scope/acceptance: compile/package changed server API and client
native/ACL/UI once; verify exact contents and one extracted release-server journey.
`make package-server` passes (`.lab/provisioning-progress-server-package.log`),
outer/inner checksums pass, and the exact extracted release binary passes the
joined native declared creation/package/snapshot/progress flow
(`.lab/native-deps/provisioning-progress-package-native.log`). The owned harness
now accepts an explicit `--server-binary`; default remains the current debug binary.
Linux archive SHA-256: `ab04bd9f92f42b3d1b0b0e0927cbc62abd4116878d705df0ca73417e0bc8ab96`.
No installation/systemd/real provider operation occurred; unrelated package suites
were not rerun for this API/client slice.

With the existing owned toolchain, `make package-android` passes frontend, both
native targets, Gradle/signature/manifest and exact embedded native-content checks
(`.lab/android-toolchain/provisioning-progress-package.log`). New ARM64 APK SHA-256
`9882383f1abed8d0c73a7256b01ffe9508e2416d7fe5289219ab605317da33aa`
(483945936 bytes); x86_64 SHA-256
`1964bdb4833064313e40b443ec1bf693305a1fa920f13f1f8a81fd3d9d612109`
(492274477 bytes). They include richer failure messages, new progress IPC/ACL and
all four progress views. `releaseCandidate=false` and
`runtimeValidatedForTheseApks=false` remain accurate. No AVD/device was started or
installed. Android instructions are copied into the output package.

Limitations: simulated Proxmox responses do not prove actual guest apt, target ACL,
storage, SSH or restored state. Current APK execution, real deployment,
Keychain/Keystore support, Windows/iOS parity and the added local-provider request
remain open. Next autonomous source task: inspect and implement the first concrete
platform credential-store use case without reading personal credentials, keeping
passphrase-encrypted-file support labelled accurately and tests in an owned store.

Publication checkpoint: source commit `6a3b622` is pushed to VMAnyware main.
Website EN/FR copy now includes recorded stages and exact extracted-server evidence;
its source-only copy update passes build/render generation and diff review
(`.lab/provisioning-progress-site-build.log`). No unrelated website suite reran for
these text changes. Website commit `85d9e1e` is pushed; Pages run `39650078223`
completed successfully (`.lab/provisioning-progress-site-deploy.log`). Both worktrees
were clean after publication. Test artifacts remain local/ignored, not a public
release or a claim of device/real-host execution.


## 2026-10-21 — Optional Linux system SSH unlock (ADR 0087)

Scope/acceptance: explicit opt-in saving/reading/forgetting of the existing app-key
passphrase in Linux Secret Service, exact directory/public-fingerprint scope,
no automatic vault access or silent fallback, unchanged encrypted SSH key, and
owned isolated acceptance including actual SSH authentication. Implemented five
fixed native commands, bringing the manifest/handler/local-main capability to 82.
Support detection only reports platform implementation. UI offers separate unchecked
save/forget consent, clears input before IPC and requires fresh connection consent
when switching to the system key store. Existing host/owner/expiry/single-use/
cancellation/authentication/session gates stay in the shared SSH implementation.

Use pinned Linux-only dbus-secret-service 4.1.0 with encrypted DH session and zero
prompt timeout. Only the default collection and exact four lookup attributes are
accessed. Correct passphrase verification precedes save; unique-item/secret readback
precedes success. Duplicate, locked, missing or corrupt entries fail closed with
fixed text. Forget deletes only this saved unlock secret; manual encrypted-file
unlock remains available explicitly. No other platform or token persistence is
implemented. ADR 0087 records the dependency, trust and recovery choices.

Verification: 18 affected native SSH checks pass and two opt-in vault cases stay
ignored by default (`.lab/native-deps/identity-vault-affected-ssh-tests.log`). This
broader SSH selection was warranted by refactored shared identity loading and the
new crypto dependency feature graph; unrelated HTTP/server suites did not rerun.
A focused namespace check also passes after adding successful-save readback
(`identity-vault-scope-test.log` in that directory). Two actual ignored cases pass
through the owned runner (`identity-vault-owned-check.log`): save/reopen/forget,
wrong secret/duplicate/locked refusal, another directory/key cannot retrieve the
secret, exact private ciphertext remains unchanged, host verification precedes
vault use, wrong window is refused and an owned SSH peer authenticates once.
Each case uses its own private D-Bus/GNOME Keyring, temporary XDG state and generated
passwords. Signed gcr 3.41.2-2 / gnome-keyring 1:51.1-1 archives were verified with
the packaged Arch public keyring and extracted to the owned prefix without hooks;
`.lab/native-deps/vault-packages.json` retains provenance. No host installation,
personal bus/keyring, credential or VM was touched.

Frontend build/typecheck passes (`.lab/identity-vault-ui-build.log`). Two scoped
React/Firefox simulated-IPC journeys pass: explicit vault save/read/forget,
unsupported/unavailable error redaction, no implicit access, input/consent clearing,
system SSH selection without a renderer secret and no automatic session
(`.lab/identity-vault-ui-check.log`); affected manual app-key/terminal/stale-response
flow also passes (`.lab/identity-vault-affected-ssh-ui.log`). Command ACL, scoped
Rust format, Python/JS syntax and diff review pass. Strict client Clippy reports
pre-existing `snapshot_restore.rs:229` too_many_arguments, confirmed unchanged in
HEAD (`.lab/native-deps/identity-vault-clippy.log`). It passes with only that known
lint allowed (`identity-vault-clippy-scoped.log`); no unrelated source was edited.

Limits: separate backend and browser evidence, not joined WebView/new IPC or an
exact-current desktop/mobile artifact. OS vault lookup metadata is not isolation
from a compromised unlocked account. Private keys are still encrypted application
files, not non-exportable hardware keys. Current APKs predate new commands; no
artifact rebuild or device/AVD/real-provider operation ran for this Linux slice.
Next task: extend the owned actual Linux WebView/IPC/manual-SSH fixture to start a
private test vault, remember through the new production command, select the system
source, authenticate/open the real owned terminal, forget and verify read refusal.
Then batch delivery/platform validation rather than rebuilding per tiny edit.

Publication checkpoint: `f5755aa` is pushed to VMAnyware main. Website copy now
separates the optional Linux backend/browser evidence from pending joined WebView
and other-OS stores, and notes that existing artifacts predate vault commands.
Build/render generation and diff review pass (`.lab/identity-vault-site-build.log`);
no unrelated website suite reran for text-only changes. Website commit `d89765a`
is pushed; Pages run `39652211141` succeeds (`.lab/identity-vault-site-deploy.log`).
The worktrees were clean after publication. No release tag or artifact rebuild was
made. Next joined native fixture must build with `tauri/custom-protocol`, use only
an owned private bus/XDG/keyring and retain exact host trust and terminal gates.


## 2026-10-21 — Joined Linux system-vault UI/IPC/SSH acceptance

Scope/acceptance: prove real React save/unlock/forget, five production vault commands,
scoped private Secret Service, system identity selection, actual SSH/PTY shell and
forgotten-secret refusal without a personal bus, keyring, agent or provider call.
Added opt-in `--system-identity` to the existing owned Linux fixture. The vault
launcher now supports this native binary inside the already private bus, validates
its root marker/exact XDG paths, waits for name ownership without activating a
second service and stops/reaps its daemon. Both native and outer runners require
a distinct system-vault completion marker, so an old normal-SSH binary cannot pass.
The example manages the same generated IdentityStore for IPC and SSH, registers the
existing five commands and verifies the exact missing-secret error at completion.

Verification: the current example builds with the verified prefix and
`--features tauri/custom-protocol` (`.lab/native-deps/system-vault-native-build.log`).
The actual Linux WebKit/Tauri/verified-HTTPS/API/owned SSH journey passes
(`.lab/native-deps/system-vault-native-check.log`): unchecked save consent,
passphrase/consent cleared, explicit vault read, separately reviewed host trust,
fresh source consent, public-key selection, real shell stdout, matching PTY resize,
terminal disconnect, explicit forget/read refusal and changed-host rejection with
saved trust unchanged. The peer still proves exactly one session channel and PTY/
shell/resize requests. The control-plane/provider side records zero provider calls;
no VM or existing credential is touched. Two affected native/backend vault cases
also pass through the adapted private launcher
(`.lab/native-deps/system-vault-affected-runner.log`).

Python/JS syntax, IPC-manifest consistency (82 production commands) and diff review
pass. No production source, dependency, schema, browser bundle, server binary or
artifact packaging changed in this slice. Earlier source/browser checks retain
their recorded evidence; they were not rerun. Normal manual-SSH shared-driver
regression and publication are recorded below when complete.

Limits: this is owned Linux runtime acceptance, not normal installed desktop service,
current APK/device, real guest or other OS store acceptance. The private key remains
an encrypted app file and only its unlock passphrase is in Secret Service. Current
delivery artifacts still predate vault commands. Next autonomous slice: implement
Android Keystore-backed protection for the same app-owned unlock use case through
the existing Android/JNI bridge, without creating/starting an AVD or reading personal
credentials. Keep no-silent-fallback, scope binding, explicit save/use/forget,
host-trust-first and backup exclusions; require independent runtime acceptance.


The affected normal manual-key SSH journey also passes through the shared driver
(`.lab/native-deps/system-vault-affected-manual-ssh.log`), retaining manual passphrase,
PTY/stdout/resize/disconnect and changed-host refusal without enabling a test vault.
No unrelated server, browser or native suite reran for these fixture-only changes.

Publication checkpoint: source/evidence commit `48e1062` is pushed. Website EN/FR
now reports the joined private-keyring acceptance and retains normal deployment,
Android/other stores and artifact limits. Text-only build/render generation and
diff review pass (`.lab/system-vault-site-build.log`); website commit `11dcb6c` is
pushed and Pages run `39654060275` succeeds (`.lab/system-vault-site-deploy.log`).
No unrelated site suite reran. Worktrees were clean after publication.

Android starting points inspected: `src-tauri/src/android_tls.rs` already exports
an `EnvUnowned`/`jni_mangle` initializer; `MainActivity.kt` loads the native library
and initializes the platform verifier before Tauri starts. Add a narrowly scoped
keystore bridge alongside that path without altering TLS verification or
regenerating the reviewed Android project. Use the existing owned build toolchain;
source/compilation never substitutes for explicit device/AVD runtime acceptance.


## 2026-10-22 — Android Keystore identity unlock, session checkpoint (ADR 0088)

Scope/acceptance: Android implementation for the same explicit save/use/forget
identity commands, key/directory scope binding, fixed failures/no silent fallback,
source/native/Kotlin compilation and package checks. Implemented a narrow JNI
helper retained via application context and initialized alongside unchanged TLS
verification. Startup does not open a vault. Rust computes a purpose/directory/
fingerprint SHA-256 scope, verifies the existing encrypted key/passphrase before
save, confirms readback and retains shared busy/host/owner/session gates. Kotlin
accepts only scoped aliases, wraps with AndroidKeyStore AES-256-GCM, binds scope as
AAD and stores only bounded versioned ciphertext in private SharedPreferences.
Existing backup/device-transfer exclusions cover that data. Missing/invalidated
keys behind ciphertext cannot be silently regenerated. Explicit forget removes
only the scoped key/ciphertext, preserving the SSH key. Fixed native exceptions
never return Java/provider errors or plaintext to the renderer. UI labels now
support Linux/Android and preserve separate unchecked consent. No new dependency,
IPC command (82 total), network permission or automatic migration is added.

Verification: `make package-android` passes with the existing owned SDK/JDK/NDK
(`.lab/android-toolchain/keystore-package.log`): frontend/typecheck, ARM64/x86_64
native compilation, Kotlin/Gradle, both signed debug APKs and signature/version/
ABI/manifest/exact native-library checks. Initial compile errors (inner doc position
and typed JNI byte-array signatures) were corrected; final compile/package handles
exited successfully. An additional package check requires IdentityVault in DEX
(`keystore-content-check.log` in that directory). Compiled Kotlin JNI descriptors
match save/read/forget, and both new vault and unchanged TLS initializer symbols
are exported (`keystore-jni-descriptors.log`, `keystore-jni-exports.log`).

The focused Linux-host Android-scope test passes
(`.lab/native-deps/android-keystore-scope-test.log`): stable lowercase 64-hex scope,
changed directory and changed public identity produce independent scopes. It does
not emulate Android cryptography. The affected React/Firefox simulated-IPC journey
passes (`.lab/android-keystore-ui-check.log`), including explicit vault operations,
input/consent clearing, scoped SSH selection, no automatic connection and private
error suppression. Scoped rustfmt, Python syntax, IPC-manifest consistency and diff
review pass. No unrelated server/Linux native suite or Clippy reran for Android
conditional branches/UI copy changes.

Current local test artifacts (`.lab/packages/android/`):
- ARM64 SHA-256 `5a6554b8e32c54d39321fb99c1d2b274be73ca0c6a75397c067793d7277d2166`, 485011736 bytes.
- x86_64 SHA-256 `0aa58facc6a8d659e76f64cdcc738c44ac2072ddac55c3429c9643d3a072ab47`, 492984213 bytes.
- `releaseCandidate=false`, `runtimeValidatedForTheseApks=false`; updated INSTALL
  instructions match repository documentation. No device/AVD was started/installed,
  personal credential/key store touched, real VM contacted or host service changed.

Limits: no actual Keystore save/decrypt, process/activity restart, ciphertext tamper,
wrapping-key invalidation/loss, OEM transfer behavior, physical security-level or
Android system-source SSH/PTY runtime evidence yet. No biometric/StrongBox claim:
the wrapping key has no user-authentication requirement, and a compromised app can
use it. The SSH key remains encrypted application storage. Forgetting saved unlock
does not revoke an already selected key/session. Source compilation cannot close
platform/phase acceptance. Existing Linux acceptance remains independently recorded.

Next task after resuming: use an approved owned Android target and current exact APK
for the acceptance sequence above; keep package provenance and no silent fallback.
If no runtime target is authorized, prepare its opt-in fixture without launching it,
then continue other source/platform work. User asked to stop at this checkpoint;
do not start another slice in this session. Website remains at the accepted Linux
checkpoint `11dcb6c`; Android source/package evidence is now in this repo and should
be synchronized on the next publication pass, without claiming device validation.


## 2026-10-23 — Prepared owned Android Keystore acceptance (ADR 0089)

Scope/acceptance: compile an isolated test APK and prepare a reviewed, opt-in
save/read/process-restart/forget plus negative Keystore scenario; enforce target/
artifact/reset gates without launching a VM or reading personal credentials.
Added four AndroidJUnit4 instrumented cases using only generated random scopes,
explicit owned-fixture argument and emulator hardware/package guards. Tests exercise
roundtrip/reopened helper/forget, authenticated ciphertext tamper/wrong scope,
wrapping-key loss with no silent regeneration and explicit forget/resave, malformed
scope/version/size and byte-array clearing. They add no production corruption/key
hook, do not enumerate aliases and clean only their own generated scopes.

The native Android harness gains --keystore with a mandatory local APK whose digest
must match the installed standalone APK before data reset. Actual IPC scenario
requires explicit save/forget confirmation, correct passphrase, unchanged public
identity, system unlock before/after process restart, forgotten-secret read refusal
and preserved manual identity. The separate planner verifies packages/signatures,
shared signer, main x86_64 ABI and test-only instrumentation target, hashes APKs and
host controllers, and records exact target/operations. Execution rechecks the plan
before ADB and checks AVD name/QEMU/boot/ABI before any installation. It requires all
four instrumented cases and matching native report; it removes only the test APK it
installed and stops the owned app. It does not create/start an emulator.

Verification: the targeted test APK assembles successfully
(`.lab/android-toolchain/keystore-test-apk-build.log`). Initial direct Gradle attempt
reached the unchanged native Rust hook needing a live Tauri CLI options server;
correct targeted build excludes :app:rustBuildX86_64Debug and reuses the existing
ADR-0088 verified library. Main Rust/Kotlin crypto source is unchanged. AndroidTest
uses pinned JUnit 1.3.0/runner 1.7.0; the unused generated Espresso dependency is
removed. A manifest now explicitly marks the artifact testOnly=true/allowBackup=false;
its absence was caught/refused by the first plan attempt before any ADB operation.

The read-only final plan passes SDK signature/manifest/ABI/signer/hash checks
(`.lab/android-toolchain/keystore-plan-check.log`). Main APK SHA-256 remains
`0aa58facc6a8d659e76f64cdcc738c44ac2072ddac55c3429c9643d3a072ab47`.
Test APK is 400436 bytes, SHA-256
`9f5447e94d2123d2686d05afa1d26a7d0fc196b6273cab96b2d1be1ec1ba264a`.
Plan SHA-256 is `47d7cc257434b038f7be1b62b8f8f5f4635565ef20acb63e9bd976c41b4798bf`.
The three host-only refusal cases pass (`.lab/android-keystore/host-guards.log`):
missing artifact causes zero fake-ADB calls, wrong AVD and changed installed APK
cause only guarded reads and no reset/install/force-stop. Python/JS syntax, diff
review and unchanged IPC manifest consistency (82 commands) pass. No unrelated
server/browser/Rust suite or production APK rebuild reran for this test-only slice.

Read-only SDK inspection confirms an API-35 Google APIs x86_64 image and Pixel-6
definition are already available; configured owned AVD listing is empty. This is
not VM creation/start authorization. Proposed setup and exact app/test operations
are reviewable in the plan and experiments/android-native/README.md. Runtime
approval remains pending; no ADB call against an emulator, installation, credential
change, real VM or provider access occurred. Four cases are compiled, not passed.
The plan report retains runtimeValidated=false and full APK/platform acceptance is
not changed. Next task: execute on the explicitly approved disposable runtime, then
record actual failures/success and continue Android system-source SSH/rotation.


## 2026-10-23 — Restore validation context and strict native Clippy

Scope/acceptance: resolve the known too_many_arguments failure in restore validation
without suppressing it or weakening operation/target/snapshot proof checks. Replaced
six positional validation fields with one borrowed RestoreContext, used by live
review and saved-operation recovery. It carries only recorded validation data,
not preparation state, consent or a serialized execution request. Expected operation
ID remains explicit. The original predicate and failure/state logic are unchanged.
No provider, persistence, protocol, API/IPC or frontend behavior is added.

Verification: 14 affected snapshot-restore native tests pass
(`.lab/native-deps/restore-context-tests.log`), including exact target/snapshot/
stopped evidence, known-task uncertainty, same-key retry, authentication and stale
reply/no-replay guards. Nine saved restore recovery tests pass
(`.lab/native-deps/restore-context-recovery-tests.log`), covering cross-target proof,
fixed GET-only reads, no restored intent, corruption/dispatch refusal and private
error fields. Strict `cargo clippy --locked -p vmanyware-client --lib -- -D warnings`
now passes with the owned native prefix (`restore-context-clippy.log` in that
directory), without any allow/disable for the old lint. Scoped Rust format and diff
review pass. No new tests mirroring the refactor were added; existing security/
protocol cases cover the changed callers. No unrelated server/frontend suite ran.

All commands are terminal. No VM, app/device or credential mutation occurred.
No package was rebuilt for this internal predicate-input refactor: reviewed Android
plan/APKs remain the earlier exact Keystore candidate, not runtime evidence for the
new restore source. No percentage or platform acceptance is inferred.

Next: keep the single pending Android target/operation approval intact; do not infer
it from an automatic continuation or repeat the approval request. Meanwhile implement
the absent macOS Keychain unlock-secret source through the existing explicit APIs,
using target-specific compilation where possible and clearly retaining real macOS
runtime/Keychain acceptance. No personal login keychain may be inspected or changed
by the agent. Windows/iOS, real infrastructure and deployment/release gates remain.


## 2026-10-23 — macOS Keychain unlock-secret source (ADR 0090)

Scope/acceptance: implement explicit save/read/forget for the existing encrypted SSH
identity, exact scope binding, no secret export/fallback, and targeted compilation
with the available tools. Added a macOS-only backend using one default SecKeychain
handle per action, fixed service and purpose/canonical-directory/public-fingerprint
SHA-256 account. Only correct manual verification precedes save; only item-not-found
allows add; readback confirms save. Forget requires consent and exact item-not-found
after delete before success. No unrestricted keychain search, enumeration, creation,
unlock, default/search-list modification or process-global interaction policy change.
Native OS prompts can follow explicit user actions; runtime/prompt/signing/locked-store
behavior is unproven. Framework buffers are freed; full memory erasure/device-only/
biometric/hardware protection is not claimed. The SSH private key stays encrypted
in its application file. Five existing API/caller cfgs now include macOS, with 82
unchanged production commands. A pinned, macOS-only security-framework dependency
reuses version 3.7.0 already in Cargo.lock, default features disabled; no new package
versions or unrelated TLS features are requested.

Verification: one focused Linux-host scope test passes
(`.lab/native-deps/macos-keychain-scope.log`) using fresh encrypted fixture files,
without a Keychain. The actual IdentityStore/macOS backend module passes Rust
metadata typechecking for aarch64-apple-darwin with production-locked versions and
checksums (`macos-keychain-typecheck.log` in that directory). The target std was
installed through rustup; no Apple SDK, framework link or Tauri app is supplied by
this probe. Initial probe-only missing serde derive dependency was fixed; the final
check passes. scripts/check-macos-identity-source.py generates a small ignored
isolated check crate, not a production workspace abstraction or mock backend.

Affected frontend build/typecheck (`.lab/macos-keychain-ui-build.log`) and real React/
Firefox simulated-IPC scenario (`.lab/macos-keychain-ui-check.log`) pass. Strict Linux
client-library Clippy passes (`.lab/native-deps/macos-keychain-clippy.log`), along with
scoped Rust/Python syntax/format, diff and unchanged command manifest consistency.
No unrelated server suite, native GUI, Android artifact build, VM/device or credential
operation ran. Existing reviewed Android Keystore artifacts/approval remain unchanged.

Read-only CI inspection found no macOS client job. Added path-filtered macos-source
workflow: pinned existing checkout/setup-node actions, read-only permissions, actual
OS/architecture/Xcode/toolchain recording, bundled frontend, full native client build
with tauri/custom-protocol, then only the no-Keychain scope test. macos-15 is the
current standard Apple Silicon label; the repository is public. This is ordinary
hosted CI compilation/test work, not a user VM/AVD or credential operation. Debug
symbols are disabled and Cargo jobs limited to two for the standard runner. Its
actual build/link result is pending until pushed/observed. No macOS GUI or Keychain
acceptance is inferred even from a successful build.

Existing Local checks CI run 40026821590 for 3d423c3 failed server formatting and
passed frontend, before server Clippy/tests; logs are retained in
`.lab/native-deps/baseline-ci-format.log`. The first reported formatting diff is in
server/src/lib.rs. This predates the Mac slice. Keep the overall CI failure visible;
resolve it as the next bounded diff rather than skipping or weakening the check.
No real private keychain has been inspected/changed. macOS runtime, iOS/Windows,
Android approved runtime, real Proxmox and deployment/release gates remain open.


## 2026-10-23 — Unblock the existing server CI formatting gate

Scope/acceptance: fix only the three formatting differences reported by the failed
Local checks CI, preserve token-level behavior and pass its exact formatting check.
Rustfmt wraps the new provisioning-progress route and sorts existing module
statements in server/src/lib.rs, proxmox.rs and store.rs (seven added/four removed
lines). No predicate, route, field, SQL, dependency or mutation behavior changes.
`cargo fmt --check -p vmanyware-server` and diff review pass
(`.lab/native-deps/server-ci-format-check.log`). No application suite was rerun
locally for this format-only diff. The existing CI still performs its required
server Clippy/tests after push; observe the result rather than assume it passes.

macOS client source CI run 40028754564 is confirmed in progress for backend commit
26eb8a2. This server-only formatting commit does not match its path filter and
should not rebuild/cancel that independent platform gate. The actual result is
pending. Android approval remains unanswered; no AVD/device operation is authorized.


## 2026-10-23 — Session-close verification checkpoint

Observed macOS source CI run 40028754564 completed successfully for 26eb8a2:
actual bundled native client build/link and scope-only test. README, ADR 0090 and
roadmap status now distinguish this from unexecuted Keychain/GUI runtime checks.
Server CI 40028869036 exposed five strict Clippy errors after formatting was fixed;
only test item order, generic argument borrows and boolean assertion syntax changed.
`cargo clippy --locked -p vmanyware-server --all-targets -- -D warnings` passes
(log: `.lab/native-deps/server-ci-clippy-close.log`). Scoped formatting and diff
review pass. No unrelated application tests, VM or credential operations ran.
The new push triggers existing Local checks; observe its actual result next session.


## 2026-10-23 — Correct historical migration fixtures

Scope: fix the demonstrated CI unit-test setup without changing production SQL or
weakening foreign-key assertions. Each fixture now uses a separate database with
the actual committed migrations through its historical version; previously the
same schema-15 database was repeatedly relabeled with older version numbers.
The focused snapshot_schema_keeps_foreign_key_enforcement_after_fresh_and_migrated_open
test passes across fresh and versions 10–14, checking final version 15, enforcement,
foreign_key_check and rejected orphan provisioning intent. Log:
`.lab/native-deps/snapshot-schema-fixtures.log`. No VM, device, infrastructure or
credential operations occurred. No unrelated application test suite was rerun.
Scoped server library/test Clippy with warnings denied and cargo fmt check pass
(`.lab/native-deps/snapshot-schema-clippy.log`); diff review passes. Next CI result
is pending push/observation, not assumed green.


## 2026-10-23 — Windows native compilation gate

Scope/acceptance: establish actual Windows compilation evidence for the existing
client before expanding its unsupported SSH/store boundary. Added a path-filtered
Windows client source workflow using standard windows-2025 x64, existing pinned
checkout/setup-node actions, read-only permissions, two Cargo jobs, debug symbols
disabled and a 30-minute limit. It records Windows/architecture/Rust, builds locked
frontend dependencies and the actual Tauri executable with bundled assets. No
app launch, credential operation, custom VM, installer or parity claim. GitHub's
official runner reference confirms windows-2025 is a standard public x64 label.
Workflow diff reviewed; execution is pending push/observation. No local application
suite repeated for this CI-only change. Next: observe Windows build and correct
actual target-specific failures. Local checks 40030173686 for 1247440 is in progress;
continue observing its handle, not restarting. Android approval is unchanged.


## 2026-10-23 — Hosted server/frontend CI recovered

Observed Local checks run 40030230911 completed successfully for ee14eae:
frontend dependency install/typecheck/build, server format, strict all-target
Clippy and the server/local TLS/process test step all passed. This includes the
corrected historical-schema test. Watch session 98462 exited successfully; log:
`.lab/native-deps/local-checks-ee14eae-watch.log`. Windows compilation remains
active in run 40030231001, job 120676388603, actual bundled-client build step.
No local suites were repeated during observation. No VM/device or credential
operation occurred. Next: observe that exact native build before claiming success.


## 2026-10-24 — Windows resource build failure

Windows run 40030231001 is terminal/failed, not live: the actual Tauri build script
requires icons/icon.ico, absent from the repository. Logs retained in
`.lab/native-deps/windows-ee14eae-failed.log`; watch session 49657 exited 1.
Generated the ICO from the existing committed icons/source.svg with the locked
local Tauri CLI into ignored .lab/windows-icons, copying only icon.ico into source.
No new design, dependency, configuration, command permission or runtime behavior.
ICO header/directory bounds and file format check pass. No Linux/mobile suite was
rerun for this Windows resource. Next: observe the affected Windows compilation
after push; later Rust/link errors are not ruled out. Hosted frontend/server CI
40030230911 remains successful for the unchanged application source.


## 2026-10-24 — Diff-selected local CI jobs

Scope: apply the user's diff-only verification rule to Local checks, which was
rebuilding server and frontend for every STATE/doc/native-icon push. A dependency-
free selector now reads the complete NUL-delimited Git diff with renames disabled
(so both removed and added paths count). Server/migrations select server; frontend
files select frontend; src-tauri is handled by the separate native workflows;
documentation selects neither. Shared/unknown inputs select both. Missing/invalid
SHAs/history, initial pushes and manual dispatch run both, without suppressing any
existing formatter, Clippy or test command. Read-only checkout supplies full history.

Verification: eight path/mixed/shared/unknown scenarios, four manual/invalid/initial/
missing-history fallbacks and matching stdout/GITHUB_OUTPUT pass. Three actual
historical diffs produce server-only, docs-only and native-icon-only as intended.
Workflow YAML parses; diff review passes. No unrelated application suites ran
locally. The workflow/selector change itself deliberately selects both in CI.
Windows run 40030829324 is confirmed live in its actual-client build step after
the icon correction; no result is inferred. Next: observe that exact Windows run
and the first selector-enabled Local checks run. No VM/device/credential operation.


## 2026-10-24 — User-requested session pause

Windows run 40030829324 is now confirmed completed/successful for 00439c4, proving
the actual bundled Windows client compiles/links with the supplied icon resource.
Local checks 40031022048 remains live; selector and frontend succeeded, server
was still running at the last detailed observation. Continue that exact run; do
not restart on observation timeout. Local watch session 67310 only observes the
now-terminal Windows run and may already have exited; no build depends on it.

iOS build investigation only: official Tauri CLI 2.12.3 build.rs source was read
into ignored `.lab/native-deps/tauri-ios-build-2.12.3.rs`. It confirms unsigned
simulator build flags and Cargo argument forwarding. Linux CLI cannot expose ios
commands; hosted macOS is needed. No iOS source/workflow/project edits occurred.
No VM/device, signing or credential operation occurred. Scope remains incomplete.
Only this documentation checkpoint changed; diff review is the required check.
