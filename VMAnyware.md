# Roadmap — PocketLab

> **Mission:** Build a native, cross-platform application for reproducible Linux environments hosted on a self-managed Proxmox VE homelab.
>
> **Product promise:** Define an environment once, provision it on Proxmox, then manage and access it from desktop or mobile whenever needed.
>
> **Status:** Implementation in progress; Phase 0 feasibility remains open.
>
> **Audience:** Coding agent / autonomous implementation agent
>
> **Approach:** Vertical slices, small changes, documented decisions, working software at every milestone.

---

## 1. Product scope

### MVP outcome

A user can:

1. Connect the application to a self-hosted PocketLab control plane.
2. Connect that control plane to one Proxmox VE host or cluster endpoint.
3. Inspect managed environments and their runtime status.
4. Create a Debian-based environment from a prepared Proxmox template.
5. Configure CPU, memory, disk, and SSH public key.
6. Follow provisioning progress until the VM is ready.
7. Start and stop the VM.
8. Connect over SSH from desktop and mobile where networking permits.
9. Create and restore VM snapshots.
10. Delete an instance while retaining its environment definition, then recreate it.

### Product principles

- **Self-hosted first:** no mandatory vendor cloud or account service.
- **Definition ≠ instance:** deleting a VM must not delete its reusable definition unless explicitly requested.
- **Proxmox is the first provider, not the domain model:** keep provider-specific details behind an adapter.
- **Reproducibility over magic:** inputs, image/template versions, provisioning steps, and limitations must be explicit.
- **Secure by default:** least-privilege Proxmox credentials, verified TLS/SSH host keys, no exposed Proxmox admin API.
- **Asynchronous operations:** VM creation, snapshotting, and restoration are tracked jobs, not long blocking HTTP requests.
- **Cross-platform with honest parity:** share domain logic and UI where practical, but verify native/mobile capabilities independently.
- **No premature infrastructure:** no Kubernetes, multi-cloud, marketplace, multi-tenant SaaS, or plugin framework in the MVP.

---

## 2. Proposed stack

Treat this as the default, not as permission to add dependencies without evaluating them.

| Area | Initial choice | Notes |
|---|---|---|
| Desktop/mobile app | Tauri 2, React, TypeScript | Validate iOS and Android constraints in Phase 0. |
| Shared client logic | Rust crates where useful | Keep shared logic independent of platform UI. |
| Control plane API | Rust + Axum | Single self-hosted Linux service. |
| Persistence | SQLite + SQL migrations | Appropriate for a single control-plane instance and MVP. |
| Virtualization provider | Proxmox VE API | No shelling into Proxmox for normal lifecycle operations. |
| Guest initialization | Proxmox templates + Cloud-Init | Start with one documented Debian template. |
| Environment definition | Versioned YAML schema | Validate strictly; never execute arbitrary config implicitly. |
| SSH | Rust SSH library and/or platform-native integration | Prove interactive PTY support on desktop and mobile early. |
| Networking | Private LAN/VPN first | Do not expose the Proxmox management API to the public internet. |
| Testing | Rust tests, API integration tests, UI checks | Use a test Proxmox environment; do not run destructive tests against an unconfirmed production host. |

Do not assume one SSH implementation or terminal renderer works equally well on every platform. Phase 0 must test this.

---

## 3. High-level architecture

```text
┌──────────────────────────────────────────────────────────┐
│ Native clients: macOS / Linux / Windows / iOS / Android │
│ Tauri + React/TypeScript                                  │
│ Environment catalog · provisioning · snapshots · SSH     │
└──────────────────────────┬───────────────────────────────┘
                           │ HTTPS + client authentication
┌──────────────────────────▼───────────────────────────────┐
│ PocketLab control plane (self-hosted Linux service)       │
│                                                          │
│ Axum API                                                 │
│ Domain model and validation                              │
│ Operation orchestrator / job state                       │
│ SQLite persistence                                       │
│ Environment schema and template renderer                 │
│ Proxmox provider adapter                                  │
└──────────────────────────┬───────────────────────────────┘
                           │ Proxmox API over verified TLS
┌──────────────────────────▼───────────────────────────────┐
│ Proxmox VE                                               │
│ VM templates · clones · lifecycle · snapshots · storage  │
└──────────────────────────────────────────────────────────┘

SSH path:
  Client ── direct SSH over LAN/VPN ──> Guest VM
  Optional later: secure tunnel/broker if direct routing is unavailable.
```

### Architecture boundaries

- `domain`: provider-neutral environment definitions, instances, snapshots, operations and state transitions.
- `application` / `orchestrator`: use cases and job execution.
- `providers/proxmox`: translate domain operations to Proxmox API calls and normalize results.
- `persistence`: SQLite repositories and migrations.
- `api`: HTTP routes, request validation, authentication and response types.
- `client`: API client, app UI, SSH experience and platform integrations.
- Do not leak Proxmox response types throughout the domain model.
- Do not introduce a generic provider abstraction until the first Proxmox implementation demonstrates the necessary interface. Keep the seam narrow and practical.

Suggested initial repository:

```text
pocketlab/
├── apps/
│   └── client/                  # Tauri 2 + React + TypeScript
├── crates/
│   ├── domain/                  # Provider-neutral models and state rules
│   ├── config-schema/           # YAML parsing and validation
│   ├── api-client/              # Typed HTTP client
│   └── ssh-core/                # SSH primitives, if shared implementation works
├── server/
│   ├── api/
│   ├── application/
│   ├── orchestrator/
│   ├── providers/
│   │   └── proxmox/
│   ├── persistence/
│   └── main.rs
├── templates/
│   └── debian-dev.yaml
├── examples/
│   └── kernel-lab.yaml
├── migrations/
├── docs/
│   ├── architecture.md
│   ├── decisions/
│   ├── development.md
│   └── proxmox-setup.md
├── ROADMAP.md
└── README.md
```

This is a target layout, not a requirement to create empty crates immediately. Start with the smallest coherent workspace and extract modules when there is a real boundary.

---

## 4. Environment definition

The definition must be versioned and separate from a live VM instance.

Example starting point (adjust names to the final schema; do not assume this is already implemented):

```yaml
apiVersion: pocketlab.dev/v1
kind: Environment

metadata:
  name: kernel-lab
  description: Linux kernel development environment

spec:
  image: debian-13
  resources:
    cpu: 4
    memoryMiB: 8192
    diskGiB: 40

  provisioning:
    cloudInit: true
    sshAuthorizedKeys:
      - ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA pocketlab-example
    packages:
      - build-essential
      - clang
      - gdb
      - qemu-system-x86
      - git

  lifecycle:
    snapshotAfterProvisioning: true
```

Requirements:

Implemented schema and defaults are documented in `docs/definitions.md`.
Public keys are inline contents; server-side key-file path resolution is not supported.

- Define and document the exact schema and defaults.
- Reject unknown or invalid fields unless forward compatibility explicitly requires another policy.
- Never store private SSH keys in environment YAML.
- Do not assume `image` names map automatically to Proxmox templates. Maintain an explicit mapping from PocketLab image identifiers to known template IDs and versions.
- Keep secrets out of logs, error messages, state snapshots and generated diagnostics.
- Make configuration export deterministic.
- Record the definition version/hash used to create each instance.
- State clearly that recreating a VM from a definition is not the same as restoring its previous disk contents.

### Reproducibility levels

1. **Configuration reproducibility:** same VM resources, template identifier, network choices and provisioning declaration.
2. **Software reproducibility:** pinned image/template versions and package versions where possible.
3. **Work-state recovery:** restore a snapshot or retain a persistent disk; do not claim this follows automatically from the definition.

The MVP must achieve level 1 and document level 2 limitations. Snapshot support provides a first path to level 3.

---

## 5. Domain model and state

Start with these conceptual entities:

- `EnvironmentDefinition`: reusable, versioned desired configuration.
- `EnvironmentInstance`: a concrete VM managed by PocketLab.
- `ProviderReference`: provider name, Proxmox node, VMID and other required identifiers.
- `Operation`: asynchronous action with ID, type, status, progress/message where available, timestamps and safe error details.
- `Snapshot`: snapshot ID/name, VM reference, creation time and description where supported.
- `TemplateReference`: PocketLab image identifier mapped to a known Proxmox template and version.

Keep the state model explicit:

- **Desired state:** what the user requested.
- **Observed state:** what Proxmox currently reports.
- **Operation state:** progress/result of a create/start/stop/snapshot/restore/delete task.

Do not equate “start API request accepted” with “guest ready for SSH”. Readiness must be a separate step and should have a timeout and clear failure state.

Operations should be safe to retry where possible. Handle duplicate requests, timeouts after a provider-side success, stale state and concurrent conflicting actions.

---

## 6. API surface for MVP

Use versioned routes (for example `/api/v1`). Exact response schemas must be documented and tested.

| Method | Route | Purpose |
|---|---|---|
| `GET` | `/api/v1/health` | Service health, without leaking secrets |
| `GET` | `/api/v1/environments` | List definitions and linked instances |
| `POST` | `/api/v1/environments` | Validate definition and provision instance |
| `GET` | `/api/v1/environments/{id}` | Read definition and runtime state |
| `PUT` | `/api/v1/environments/{id}` | Update definition only when safe and explicitly supported |
| `POST` | `/api/v1/environments/{id}/start` | Start instance |
| `POST` | `/api/v1/environments/{id}/stop` | Stop instance |
| `POST` | `/api/v1/environments/{id}/recreate` | Recreate instance from retained definition |
| `DELETE` | `/api/v1/environments/{id}/instance` | Delete instance, retain definition |
| `DELETE` | `/api/v1/environments/{id}` | Delete definition and instance only with explicit confirmation |
| `GET` | `/api/v1/environments/{id}/snapshots` | List snapshots |
| `POST` | `/api/v1/environments/{id}/snapshots` | Create snapshot |
| `POST` | `/api/v1/environments/{id}/snapshots/{snapshotId}/restore` | Restore snapshot |
| `GET` | `/api/v1/operations/{operationId}` | Read asynchronous operation state |

API rules:

- Long-running operations return an operation identifier, typically with `202 Accepted`.
- Validate request bodies and resource bounds before calling Proxmox.
- Use structured errors with stable error codes and safe user-facing messages.
- Add pagination only when needed; do not overbuild it for the initial single-user case.
- Do not expose arbitrary Proxmox API passthrough routes.
- Do not log authorization headers, tokens, private keys, Cloud-Init secrets or full sensitive payloads.

---

## 7. Roadmap and acceptance criteria

### Phase 0 — Feasibility spikes and decisions

**Goal:** Resolve high-risk platform and infrastructure questions before building the full product.

Tasks:

- [x] Create the repository and minimal Rust workspace.
- [x] Create a minimal Tauri 2 app and verify development/build setup. (macOS arm64: dev/release builds pass; user confirmed native bridge returns `macos`.)
- [x] Document the supported OS versions and toolchain. (Current macOS development baseline and unverified targets in docs/platforms.md; no release/minimum-OS guarantee.)
- [ ] Connect a Rust prototype to a disposable/test Proxmox VE environment using a dedicated least-privilege API token.
- [ ] Verify TLS certificate handling and API-token permissions.
- [ ] Read VM inventory and inspect a known template.
- [ ] Prototype clone → Cloud-Init configuration → start → poll task → verify readiness.
- [ ] Prototype create/list/restore snapshot on a disposable VM.
- [ ] Test SSH authentication, host-key verification, interactive PTY and terminal resizing on desktop. (Native macOS manager/agent/real-shell fixtures and isolated WebKit renderer checks pass; integrated Tauri UI/IPC and remote guest validation pending.)
- [ ] Test whether the chosen terminal and SSH approach works on both iOS and Android.
- [ ] Record mobile limitations and any platform-specific implementation required.
- [ ] Write short ADRs for the decisions that materially affect architecture.

**Acceptance criteria:**

- A disposable VM can be cloned and started through the API.
- Proxmox TLS is verified; no insecure certificate bypass is the default.
- Required Proxmox permissions are documented.
- SSH/terminal feasibility has been tested on desktop and both mobile OS families, or the MVP limitation is explicitly recorded.
- A short technical risk report identifies the next implementation path.

**Exit rule:** Do not proceed on the assumption that one shared terminal implementation works everywhere. If mobile PTY support is uncertain, resolve it with a working prototype or make a documented, deliberate MVP trade-off.

### Phase 1 — Repository foundation and domain model

**Goal:** Establish the smallest maintainable structure.

Tasks:

- [ ] Add formatting, linting and test commands for Rust and TypeScript.
- [x] Add a README with setup and development commands.
- [ ] Implement core domain types and validation.
- [ ] Define lifecycle and operation states.
- [x] Add YAML schema parsing and deterministic serialization.
- [x] Add SQLite schema and migrations.
- [x] Add structured tracing and safe error handling. (Allowlisted JSON request/lifecycle events and fixed errors; subprocess redaction/shutdown tests pass.)
- [x] Add a development configuration example that contains no secrets.

**Acceptance criteria:**

- Domain tests cover valid/invalid definitions and state transitions.
- Migrations run from a clean database.
- Invalid configuration fails before any provider call.
- No credentials are committed or printed in logs.
- A new contributor or agent can build and test the project using the README.

### Phase 2 — Control plane and Proxmox adapter

**Goal:** Manage an existing VM through a documented API.

Tasks:

- [ ] Implement the Axum service and versioned API routes.
- [x] Implement Proxmox client configuration and verified TLS. (Trusted/untrusted TLS and hostname checks covered by local fixtures; live host verification pending.)
- [ ] Support listing nodes/templates/managed VMs as required by the domain.
- [ ] Implement VM start and stop.
- [ ] Poll and track Proxmox tasks without blocking the HTTP request.
- [ ] Persist operations and their terminal states.
- [ ] Add request validation, timeouts and safe error mapping.
- [ ] Add control-plane authentication before exposing it beyond localhost/private network.
- [ ] Document the dedicated Proxmox user, token and minimum required permissions.

**Acceptance criteria:**

- An API client can list and start/stop a known test VM.
- Long-running work is represented by a persistent operation ID.
- A service restart does not silently lose the record of in-progress operations.
- Provider errors are translated into safe, actionable API errors.
- The service does not require a Proxmox administrator credential.

### Phase 3 — Provisioning and reusable definitions

**Goal:** Create a VM from a reusable definition.

Tasks:

- [x] Document how to prepare/import a supported Debian template in Proxmox. (Proposed Debian 13 amd64 procedure reviewed against official sources; real-host support validation pending.)
- [x] Implement an explicit PocketLab image-to-Proxmox-template mapping. (Strict server catalog and offline plan; live template verification pending.)
- [ ] Implement clone and resource configuration.
- [ ] Apply Cloud-Init settings and SSH public keys.
- [ ] Implement provisioning/readiness polling with timeout and failure handling.
- [ ] Persist definition, definition hash, provider reference and operation state.
- [ ] Implement environment list/detail views in the API.
- [ ] Implement deletion of an instance while retaining its definition.
- [ ] Implement recreation from the retained definition.
- [ ] Ensure partial failures are reported and recoverable; do not silently leave unmanaged VMs.

**Acceptance criteria:**

- A valid definition creates a VM from the documented template.
- CPU, memory, disk and SSH public key match the requested values where supported.
- Provisioning does not report “ready” until the configured readiness checks pass.
- A deleted instance can be recreated without rewriting its definition.
- A failed clone/provisioning operation leaves a visible error and has a documented cleanup/retry path.
- The same definition produces deterministic configuration inputs, while acknowledging that generated VM IDs and runtime addresses differ.

### Phase 4 — Snapshots and lifecycle safety

**Goal:** Support safe recovery and basic VM lifecycle management.

Tasks:

- [ ] List snapshots for an environment instance.
- [ ] Create a snapshot with a validated, unique name and optional description.
- [ ] Restore a selected snapshot with an explicit destructive-action confirmation.
- [ ] Track snapshot and restore tasks as operations.
- [ ] Handle unsupported storage/snapshot conditions with clear errors.
- [ ] Document snapshot consistency and storage requirements.
- [ ] Distinguish snapshots from independent backups in the UI and documentation.
- [ ] Add guardrails for deletion and conflicting operations.

**Acceptance criteria:**

- Snapshot creation and restoration work on the supported test storage configuration.
- A restore operation cannot be mistaken for a successful result before Proxmox completes it.
- Unsupported snapshot operations fail safely and explain why.
- The UI makes the selected VM and snapshot clear before a destructive restore.

### Phase 5 — Native client and cross-platform UX

**Goal:** Deliver the core user journey on desktop and mobile.

Tasks:

- [ ] Implement control-plane connection setup and secure credential handling.
- [ ] Implement environment catalog and detail screens.
- [ ] Implement create-environment form and configuration validation.
- [ ] Implement operation progress, errors and retry affordances.
- [ ] Implement start/stop/recreate/delete actions with confirmation where needed.
- [ ] Implement snapshot list/create/restore flows.
- [ ] Implement SSH connection management and known-host verification.
- [ ] Implement a usable terminal experience on desktop.
- [ ] Implement a mobile terminal layout, software keyboard handling, special keys and reconnection.
- [ ] Test the same core journey on macOS, Linux, Windows, iOS and Android.
- [ ] Document platform-specific limitations and required permissions.

**Acceptance criteria:**

- The user can complete the main workflow without opening the Proxmox web UI.
- All long-running operations show progress and actionable failure states.
- Mobile layouts remain usable at small screen sizes.
- Secrets are stored using appropriate platform facilities where supported.
- SSH host keys are verified and unknown/changed host keys are handled explicitly.
- Platform support claims match tested capabilities; no untested parity claims.

### Phase 6 — Security, packaging and release candidate

**Goal:** Make the MVP safe to run on a real homelab.

Tasks:

- [ ] Review the threat model and trust boundaries.
- [ ] Ensure control-plane authentication is required on non-loopback interfaces.
- [ ] Restrict CORS and network binding to documented safe defaults.
- [ ] Add rate/size limits appropriate to the API.
- [ ] Review secret redaction and error messages.
- [ ] Test expired/invalid tokens, TLS failures, network interruptions and Proxmox unavailability.
- [ ] Test app and service upgrades, migrations and restart recovery.
- [ ] Package the control plane as a Linux service with a documented configuration file.
- [ ] Package clients for the platforms that have passed acceptance testing.
- [ ] Write installation, backup/restore, troubleshooting and uninstall documentation.
- [ ] Tag a release candidate only when all required acceptance criteria pass.

**Acceptance criteria:**

- No default configuration exposes the Proxmox management API publicly.
- Proxmox credentials are least-privilege and can be revoked.
- The control plane can restart without corrupting stored definitions or losing operation history.
- Installation and recovery steps have been tested from a clean setup.
- Known limitations and security assumptions are documented.

---

## 8. MVP definition of done

The MVP is complete when all of the following work end-to-end:

- [ ] Connect client to the self-hosted control plane.
- [ ] Connect control plane to a supported Proxmox VE environment securely.
- [ ] Create an environment from the supported YAML definition.
- [ ] Clone a prepared Debian template and configure its resources.
- [ ] Configure SSH public-key access through Cloud-Init.
- [ ] Follow provisioning until the VM is ready.
- [ ] Start and stop the VM.
- [ ] Connect by SSH from supported desktop and mobile clients, subject to documented network requirements.
- [ ] Create and restore a snapshot.
- [ ] Delete the VM but retain the definition.
- [ ] Recreate the VM from the retained definition.
- [ ] See errors and recover from a failed or interrupted operation.
- [ ] Complete the workflow without using the Proxmox web UI.
- [ ] Install and operate the service using the documented instructions.

---

## 9. Explicitly out of scope for MVP

Do not implement these unless the user explicitly changes the scope:

- Additional hypervisors (libvirt, VMware, Hyper-V, cloud providers).
- Public hosted SaaS/control plane.
- Multi-tenant organizations, invitations, RBAC matrix or collaboration.
- Kubernetes deployment or distributed control plane.
- VM migration, HA orchestration or cluster scheduling.
- Full graphical console streaming (VNC/SPICE).
- Marketplace/community template repository.
- Snapshot replication or independent backup system.
- Automatic public relay/tunnel service.
- Arbitrary shell command execution on the Proxmox host.
- Automatic installation of a guest agent unless required and documented for the chosen template.
- Billing, analytics or telemetry by default.

---

## 10. Security requirements

These are implementation requirements, not optional polish:

1. Use a dedicated Proxmox account/API token with only the permissions required for the supported workflows.
2. Verify TLS for Proxmox API connections. Do not ship `danger_accept_invalid_certs` or equivalent insecure defaults.
3. Never expose arbitrary Proxmox API passthrough to clients.
4. Do not store private SSH keys in environment definitions or send them to the control plane for direct SSH connections.
5. Verify SSH host keys; make first-use and changed-key behavior explicit.
6. Store client credentials in platform-appropriate secure storage where possible.
7. Require authentication when the control plane listens on a network interface.
8. Redact credentials, tokens, private keys and sensitive Cloud-Init data from logs.
9. Require explicit confirmation for destructive actions such as VM deletion and snapshot restoration.
10. Never run generated YAML, scripts or user input on the Proxmox host as arbitrary shell commands.
11. Bound resource values and validate identifiers before provider calls.
12. Document that the control plane is a privileged service and must be reachable only through a trusted network or a securely configured access path.

---

## 11. Testing strategy

- **Unit tests:** schema validation, domain transitions, operation state, error mapping and serialization.
- **Provider tests:** mock HTTP responses for Proxmox tasks, API failures, expired credentials and timeouts.
- **Integration tests:** run against a disposable Proxmox VM/template and test storage.
- **End-to-end tests:** create → ready → SSH → snapshot → modify → restore → delete instance → recreate.
- **Platform smoke tests:** launch the app and verify connection setup on each supported OS.
- **Security tests:** invalid credentials, certificate mismatch, changed SSH host key, malformed definitions, oversized payloads and unauthorized operations.
- **Recovery tests:** control-plane restart during a running operation and reconciliation against observed Proxmox state.

Never make destructive integration tests depend on a real user's default Proxmox environment. Require an explicitly configured test target and explicit confirmation before running them.

---

## 12. Working instructions for the coding agent

### Execution protocol

1. Read this roadmap and the existing repository before making changes.
2. Inspect existing code and conventions; do not overwrite functioning work to match the suggested folder layout.
3. Start with **Phase 0**. Do not jump directly to a large implementation.
4. Implement one small vertical slice at a time.
5. Before each phase, list the intended changes and the acceptance criteria being targeted.
6. After each slice, run relevant formatting, linting and tests; report actual results.
7. Update this roadmap by checking off completed items only when evidence supports completion.
8. Record significant architecture choices in short ADRs under `docs/decisions/`.
9. Ask before destructive actions, production infrastructure changes, credential changes or operations that could affect existing VMs.
10. Do not fabricate test results, successful builds, platform support or API behavior. Clearly mark untested items.
11. Prefer official Proxmox and Tauri documentation when confirming API or platform behavior.
12. Keep dependencies minimal and justify each new runtime dependency.
13. Do not introduce abstractions, generic frameworks or extra infrastructure without a demonstrated requirement.
14. If a decision is blocked by missing credentials, a test host or hardware, implement a mock/testable seam and document what remains blocked. Never invent credentials or infrastructure.
15. At the end of each work session, report: files changed, behavior implemented, commands/tests run and their actual outcomes, known risks, and the single next recommended task.

### Definition of a completed task

A task is complete only when:

- the implementation exists;
- the relevant tests or a documented manual verification have run;
- errors and edge cases have been considered;
- docs/configuration examples have been updated when needed;
- no secret or unsafe default was introduced;
- the agent reports evidence rather than merely claiming completion.

### First task for the agent

**Do not implement the full roadmap immediately.** Start by:

1. Inspecting the repository and available development environment.
2. Proposing the smallest Phase 0 spike plan.
3. Identifying the exact unknowns for Proxmox API access, Cloud-Init and mobile SSH/PTY support.
4. Creating only the minimal project skeleton and tests needed for those spikes.
5. Reporting findings and recommending the next phase.

---

## 13. Reference documentation

Use these as starting points and verify current details during implementation:

- Proxmox VE API Viewer: <https://pve.proxmox.com/pve-docs/api-viewer/>
- Proxmox VE Administration Guide: <https://pve.proxmox.com/pve-docs/pve-admin-guide.html>
- Tauri documentation: <https://v2.tauri.app/>
- Axum documentation: <https://docs.rs/axum/latest/axum/>
- SQLite documentation: <https://www.sqlite.org/docs.html>
- Cloud-Init documentation: <https://cloudinit.readthedocs.io/>

---

## 14. Product north star

> **A reproducible systems lab, available from any device.**

The first version should do a few things reliably: define, provision, access, snapshot, restore and recreate. Build a trustworthy end-to-end workflow before expanding the provider matrix or adding infrastructure.
