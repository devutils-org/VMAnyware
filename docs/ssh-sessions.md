# Native macOS PTY transport

This Rust/native-command transport now backs the [graphical prototype](ssh-terminal.md).
It reuses
[explicit agent selection](ssh-agent.md); it has no private-key, command-execution,
agent-forwarding or generic signing parameter. A new selection authorizes one
connection attempt for the endpoint/login/key held in Rust.

`start_ssh_session` consumes that selection, repeats persisted host verification,
authenticates with only the chosen identity, then requests a session channel,
`xterm-256color` PTY and shell. Both requests require positive acknowledgments.
The overall start deadline is ten seconds. Dimensions must be 1–500 rows/columns.
Unknown/changed hosts fail before the agent is contacted.

## Ownership and bounds

Each window owns at most one session; a new start replaces its previous session.
There are at most eight window slots and eight simultaneous starts. Random opaque
IDs and the invoking native window bind write/read/resize/close operations.
An old close request cannot remove a newer session. Window destruction cancels
its pending selection and aborts its owned session actor. Session startup checks
its generation between handshake/channel/PTY/shell stages. Requests already sent
to a peer cannot be revoked.

`write_ssh_session` accepts 1–32768 raw bytes; its success means queued, not remote
execution or consumption. Ctrl-C is byte 3, not a special remote execution API.
Input and output queues each contain at most eight messages. Input saturation
returns a fixed retryable error; bytes are never silently discarded. An actor
keeps at most one additional pending output event and stops reading while output
is blocked, retaining input handling. Each native output event carries at most
32768 bytes. Unexpected oversized/unsupported extended data closes the session.

`read_ssh_session` has one outstanding reader per session. It returns a typed
binary stdout/stderr, exit status, EOF, closed or fixed failure event, or no event
after 250 ms. It does not decode UTF-8, combine byte chunks into strings or log
terminal content. An exit status/EOF does not replace final connection closure.
Queued output drains before the closed result. A failure event is best effort if
the output queue is already full; transport closure remains observable.

A three-second write/resize deadline closes a blocked transport. Client channel
queues contain at most sixteen messages, with a 256 KiB SSH window and 32 KiB
maximum packets. Keepalives run every thirty seconds (three misses maximum),
and the connection has a five-minute inactivity timeout. These limits bound the
prototype; they are not a claim of production terminal throughput.

The actor retains a cloned TCP shutdown guard. Closing/removing its entry aborts
that actor and shuts down its own socket even if russh has detached internal
work. No global process, personal agent or unrelated SSH connection is killed.
Already pending native reads may complete during close; the frontend ignores
results from a superseded session generation.

## Evidence boundary

Default macOS Rust tests start cleared-environment disposable agents and local
SSH fixtures with generated keys and temporary trust stores. They exercise exact
agent authentication, binary data including NUL/255/Ctrl-C, maximum input chunks,
PTY/shell acknowledgments and refusals, observed resize, remote exit status/EOF,
wrong-window refusal, close/reconnect and window-owned transport cleanup. Separate
policy tests exercise input saturation, single-reader enforcement and bounds.

The fixture echoes protocol data; it does not execute a remote OS shell. The
isolated `make check-ssh` real-shell fixture still establishes local PTY resize
and actual foreground Ctrl-C behavior. Neither alone proves an integrated graphical native terminal
or real guest interoperability; WebKit renderer tests have their own boundary. No real infrastructure/credential was used.

The native managers now also have a real-shell integration test, using the same
disposable PTY source as the isolated experiment. It verifies persisted first-use
confirmation, exact agent authentication, actual /bin/sh output and stty resize,
foreground sleep interruption/status 130 within two seconds, reconnect, and
closure while another sleep is active. Owned shell/foreground PIDs and SSH
connections must disappear after both explicit close and window destruction.
This does not involve the renderer or Tauri's actual IPC/window process.
portable-pty is used only as a macOS test dependency, not a product dependency.
