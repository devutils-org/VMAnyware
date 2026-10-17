# ADR 0012 — Bound the native macOS PTY transport before rendering

Date: 2026-10-17
Status: Implemented; local fixture evidence is recorded in STATE.md.

Reuse the native host-trust and selected-agent authentication path for one PTY
per window. Keep connection handles, socket ownership and channel processing in
Rust. Expose only opaque window-bound IDs, bounded byte input/output, dimensions
and close. No user command string or new dependency is needed.

Use pull-based native reads with one 250 ms reader and bounded input/output
queues rather than push events without acknowledgment. A renderer can request
the next chunk after consuming the previous one, providing backpressure across
the WebView boundary. Keep bytes intact for incremental terminal decoding. A
single channel actor processes input while output is pending; never silently
drop user input on saturation.

Require positive PTY and shell acknowledgments before reporting start. Limit
starts, windows, channel buffers, frame sizes and blocking writes. Retain an
owned socket shutdown guard throughout the actor so explicit close, replacement,
window destruction, startup failure and task abort release that connection.
Document that cancellation cannot revoke a remote request already transmitted.

This establishes transport policy with local protocol fixtures only. A graphical
terminal, actual native UI walkthrough, real shell/guest integration and mobile
platform validation remain separate gates. See docs/ssh-sessions.md for the IPC
contract and exact evidence boundaries.
