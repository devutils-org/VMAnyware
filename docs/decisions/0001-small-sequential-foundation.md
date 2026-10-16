# ADR 0001 — Start with one client and sequential feasibility slices

Date: 2026-10-16
Status: Accepted for Phase 0; platform/library choices remain subject to spikes.

## Context

The roadmap targets a Rust/Axum control plane and a Tauri 2 React/TypeScript
client. Proxmox provisioning and interactive SSH on mobile carry unresolved
risks. The user prefers conserving their subscription allowance through small
iterations rather than concurrent agent work.

Sibling projects inspected:

- `../omniscient`: explicit working instructions, workspace conventions,
  decisions, measured state and scoped verification.
- `../atfq`: small Rust modules, fixture-driven tests and short dated decisions.
- `../distroproof`: not found at the supplied location; no conventions inferred.

## Decision

- Keep the original roadmap as `VMAnyware.md`; do not create a competing copy.
- Use English artifacts, a short `AGENTS.md`, a bounded `STATE.md` and small ADRs.
- Work with one agent and one slice at a time. Worktrees are optional isolation,
  not a prerequisite for the project.
- Prepare only the client shell, including a real frontend-to-Rust smoke check.
  Use npm because it is already available. Vite supplies TypeScript compilation;
  React supplies the UI; Tauri API/CLI supply the native bridge and runner.
  No UI kit, state library or native plugins are needed for this slice.
- Defer Axum, SQLite, Proxmox and SSH dependencies until their own prototypes.
  Do not copy the observability or content infrastructure of sibling projects.
- Defer CI/release automation until commands have passed locally and dependency
  locks exist. Do not publish unverified platform support claims.

## Consequences

The foundation is small and easy to resume. It cannot manage environments yet.
Actual desktop launch and mobile feasibility must be demonstrated before moving
to full implementation. The temporary floating Rust channel and package ranges
must become reproducible through a tested toolchain pin and committed locks.
