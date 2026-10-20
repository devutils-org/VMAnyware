# 0052 — Owned Android creation-to-SSH integration gate

Status: current x86_64 debug APK passes the local generated-provider journey.

## Decision

Extend the opt-in Android native harness with an explicit creation mode. Reuse the
actual packaged Tauri application, authenticated HTTPS connection, production
control-plane executable and sequential worker. Add only a loopback HTTPS peer
that implements the bounded Proxmox requests exercised by this workflow. It has
in-memory state, exact mutation/command assertions and no external endpoint.

The service receives a clean environment containing only generated fixture
credentials, private CA, disposable database and attested fixture catalog. Creation
mode starts with an empty database rather than seeded operation results. Its HTTPS
proxy permits only GET and the three fixed POST endpoints for plan, definition
storage and confirmed creation. The ordinary read-only mode remains GET-only.

Drive the real native UI through application-key unlock/public-key insertion,
preview, unchecked confirmation and explicit submission. Check exactly one create
request and the expected clone/configuration/seed/start/readiness requests, with
the same Android public key and requested resources. Require guestReady before
selecting the explicit handoff and checking its exact address/account review.

The generated provider declares a synthetic guest at 192.0.2.42. The host-alias
network route is still unavailable. Therefore the SSH portion explicitly edits
the endpoint to the owned ADB reverse tunnel and uses the same generated app key
against the existing real OS-shell SSH fixture. This is an explicit test transport
substitution, not proof that the synthetic guest exists or is reachable. Do not
weaken the production readiness/IP or SSH host-verification rules to make it pass.

The shared SSH driver scopes buttons to its host-trust section, so terminal
Disconnect cannot accidentally disconnect the control plane. Key-panel selectors
use details.app-identity because the application header shares its older class.
These are harness corrections, not production fixture flags or mocked native IPC.

## Evidence and limits

Actual Android execution covers native TLS/token handling, definition storage,
plan/confirmation, production worker execution, guest proof supplied by the local
provider, explicit handoff, strict SSH trust, wrong passphrase, authentication-only,
real shell input, rotation/stty, interrupt, reconnect, process death and changed-key
refusal. Provider mutation counts remain unchanged across SSH/application restart.

The opt-in report records only public fingerprints and method/path request lists.
Owned service/provider/SSH sockets, reverse mappings, TLS keys and database are
removed in cleanup. APK/report hashes and reviewed screenshots live in ignored
.lab records. No real Proxmox connection, VM, personal key or credential store is
used. Full software IME, direct/deployed networking, physical devices, iOS, real
provisioning and release acceptance remain open. This gate does not close those
roadmap requirements.
