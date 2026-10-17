# ADR 0013 — Use xterm.js for the bounded macOS terminal prototype

Date: 2026-10-17
Status: Implemented; compiled and isolated WebKit/fixture-tested locally.

The native PTY transport now provides an actual bounded byte-stream use case.
Add exactly pinned xterm.js 6.0.0 and addon-fit 0.11.0 to render it and propagate
container dimensions. Bundle them locally under the existing IPC-only CSP. No
WebSocket/attach, clipboard, web-links or GPU addon is needed. This is a desktop
prototype choice, not evidence for iOS/Android compatibility.

Use the parsed-write callback to request the next native output event only after
the previous block is consumed. Preserve binary chunks for incremental terminal
decoding. Serialize input with a fixed byte budget and coalesce resize bursts;
close visibly when delivery cannot proceed. Never silently discard typed input.

Keep session creation on the explicit user action. Render an existing opaque ID;
StrictMode setup replay must not initiate a second login or close the surviving
session. Close late start results and suppress old read/error callbacks. Record
Node controller tests, actual WebKit rendering with simulated IPC, Rust protocol
fixtures and real-shell evidence separately; no one fixture proves the full path.

Ignore remote title/link actions and consume clipboard OSC 52. Do not insert
terminal content as HTML or load external scripts/fonts. Follow the published
xterm flow-control and embedding-security guidance linked in docs/ssh-terminal.md.
A manual packaged native walkthrough and real guest test remain open gates.
