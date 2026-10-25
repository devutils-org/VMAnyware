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
