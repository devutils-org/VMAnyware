# ADR 0099 — Standalone read-only terminal client

Status: accepted, 2026-10-25.

## Context

Users working in a shell or tmux need a quick way to inspect environments and
follow operations without starting the graphical application. The existing
control plane already supplies the required authenticated read endpoints.
The native Rust client currently lives inside the Tauri crate, while confirmed
action sequencing also uses TypeScript. Reusing that crate would pull desktop
build requirements into a terminal-only binary.

## Decision

Add one workspace crate, `apps/tui`, producing `vmanyware tui`, using Ratatui
and Crossterm like sibling terminal applications. It contains only the concrete
HTTP read projections needed by the working UI; there is no provider adapter,
server dependency, generic request command or empty shared crate. Do not refactor
the proven native client as part of this first read-only slice.

The terminal client uses verified HTTPS, the server bearer-authentication
marker, bounded responses, no redirects/retries/proxies, and optional explicit
private CA trust. It validates object identities and definition hashes before
displaying associated evidence. Only GET endpoints are available. A background
reader handles bounded refresh batches; the terminal loop owns input and
rendering. Late selected-object replies are discarded. Quitting restores the
terminal and does not wait for network timeouts or affect server work.

Mouse follow-up: render visible controls and their hit rectangles together.
List hits use the actual rendered list offset and filtered indices; hidden
panes and modal-obscured controls have no active hit areas. Mouse actions reuse
the keyboard command guards, and wheel input acts on the pointed pane. Enable
Crossterm mouse capture with cleanup alongside bracketed paste on exit/panic.

Ratatui is pinned to 0.30.2. Its `unstable-rendered-line-info` feature supplies
wrapped paragraph height for scroll clamping; the locked version and rendering
tests protect this narrow API usage. Other HTTP/serialization/runtime/crypto
dependencies reuse the existing workspace versions. New lock entries come
from the terminal rendering/input stack, without updating existing packages.

## Consequences

This provides a usable monitoring interface without a graphical runtime or new
server contract. Small read-only projections overlap the native client and must
be kept aligned with the wire protocol. Extract a shared Tauri-independent API
client when a second slice actually needs shared action behavior; never copy
mutation/recovery state machines into a second frontend. System SSH handoff and
confirmed VM actions remain subsequent slices. No native/mobile roadmap gate
is closed by terminal-fixture acceptance.
