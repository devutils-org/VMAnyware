# 0094 — Application shell for the native client

Status: accepted (2026-10-25)

## Context

The client rendered every feature on one long page: connection, templates,
each environment's controls and recovery panels, creation, SSH and runtime
checks. Panels appeared and grew with each click, so real acceptance testing
required endless scrolling and re-entering the server details every launch.

## Decision

- A connect screen precedes the workspace. Only the public server address and
  public CA are remembered (browser storage, opt-in, forgettable); the access
  token is never stored. The local loopback service stays available as a
  developer mode from that screen.
- After connecting, a shell with a sidebar (desktop) or bottom tab bar (phone)
  offers Environments, New environment, Terminal, Activity and Settings. The
  workspace loads automatically.
- An environment opens a detail view with Overview (power + retained
  definition, read on open), Snapshots (listed on open) and History (all
  recovery reads). Mutations (start, shutdown, snapshot, restore, delete,
  recreate) open in a modal side drawer that cannot be closed while the
  existing component reports a lock, so review/consent/recovery semantics are
  unchanged.
- Creation and Terminal stay mounted while hidden so a live SSH session or an
  in-progress creation survives navigation.
- Visual language follows devutils-org.com tokens (near-black ground,
  cyan/violet accents, hairline borders, 18/12/6 px radii, monospace labels)
  applied to application controls rather than landing-page sections.

## Consequences

No IPC command, server route or safety check changed. Opening an overview or
snapshot tab now performs the same read the previous explicit button did; reads
never submit work. Screenshots were produced from the built bundle with a
preview-only fake bridge outside the repository; native GUI acceptance remains
a separate manual check.

## Amendment — streamlined action flow (same day)

Real use showed each action needed about seven interactions (review button,
one or two checkboxes, confirm, manual refresh, acknowledge, close). Now:

- Actions without inputs (start, shutdown, restore, delete) run their
  read-only review when the drawer opens; snapshot and recreation keep one
  Continue step for their inputs.
- The labelled primary button ("Start VM 9004", "Create snapshot ui-base") is
  the explicit confirmation for non-destructive actions. Destructive actions
  (restore, delete) keep one consent checkbox that sets both the confirmation
  and the data-loss acknowledgement the native command still requires.
- Submitted operations are re-read automatically (bounded polling, reads only)
  until succeeded/failed; uncertain readiness keeps being read as before.
- The drawer can always be closed: running or pending work stays mounted in
  the background with a top-bar indicator. Once an operation is definitively
  succeeded/failed, the retained native request state (the former
  acknowledge) is released automatically; a success closes its drawer after a
  short delay (except recreation, which offers SSH next), a failure stays
  readable. Uncertain or still-running work is never released. "Done" remains
  only as a fallback if the release call fails. Power and snapshot views re-read
  after an action settles.

Prerequisite checks, request-key retention, same-key retry and evidence
validation are unchanged.
