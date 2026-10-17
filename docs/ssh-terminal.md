# macOS graphical SSH terminal prototype

After [host verification](ssh-host-inspection.md), enter the SSH login name,
allow agent access and list identities. Choose one public fingerprint, then
**Open terminal**. This consumes that selection; the alternative **Check selected
identity** still authenticates and closes without a terminal. A fresh connection
always requires a fresh explicit identity listing and verified host handshake.

The graphical prototype uses pinned xterm.js 6.0.0 and addon-fit 0.11.0, bundled
locally and loaded only when explicitly opening a terminal. Renderer loading
finishes before the native connection starts; a load error cancels the selection.
The parent retains socket ownership during lazy rendering and cancellation. Text input is UTF-8; xterm binary input stays raw. **Send Ctrl-C** emits
byte 3, while **Disconnect** closes the current Rust-owned session. Resizing the
terminal container propagates dimensions, clamped to the native limit of 500.
After closure, choose/list an identity again rather than replaying a consumed ID.

The [native PTY transport](ssh-sessions.md) stays in Rust. The renderer reads one
bounded event at a time, waits for xterm's parsed-write callback, then requests
the next block. It never independently decodes output chunks, so split UTF-8
sequences reach xterm intact. Scrollback is limited to 1000 lines. Frontend input
is serialized into at most 32 KiB messages with a 128 KiB pending-byte limit;
resize bursts coalesce. Native queue saturation retries the same input for at
most one second. Any overflow or persistent write failure visibly closes the
session rather than silently losing input.

Superseded/unmounted sessions ignore late reads and failures. A start result that
arrives after cancellation is immediately closed. StrictMode's discarded initial
effect starts no reader and cannot close the surviving session. Endpoint/login
edits, relisting and leaving the host-trust screen dispose the old terminal. The
native window lifecycle also closes its owned socket independently of JavaScript.

No remote HTML, title or link is applied to the application; OSC 52 clipboard
access is consumed without action. There is no link/clipboard addon, remote-font
loader, agent forwarding or external runtime script. Terminal data and keystrokes
stay within the bundled renderer/native SSH path and are not logged or sent to
the control plane. As with any terminal, content can influence terminal state;
this prototype is not a sandbox for arbitrary untrusted applications.

Unread output that remains blocked for three seconds fails and closes the
connection. The pinned SSH library can otherwise stop processing outgoing Ctrl-C
while its receive queue is full. A backgrounded/suspended renderer may therefore
lose its session. Active output consumers can interrupt a fast producer; a
fully paused consumer gets an explicit failure instead of an indefinite hang.
See [ADR 0014](decisions/0014-stalled-terminal-output.md).

## Verification and limits

`make check-client` compiles the renderer and runs seven Node tests of the actual
transport controller: output backpressure, binary/split UTF-8 preservation,
serialized bounded input, resize coalescing, full-queue retries, overflow, stale
results and exit/EOF/close. They use simulated native calls, not SSH.

`make check-terminal-webkit` optionally exercises the real xterm renderer and
React development UI in macOS WebKit with isolated simulated IPC. Its
[fixture boundary](../experiments/terminal-webkit/README.md) is explicit. Native
Rust tests exercise the product agent/PTY path against both generated protocol
fixtures and an actual local OS shell, including real resize/Ctrl-C and owned
process cleanup. `make check-ssh` retains its separate actual-shell exercise.

A manual native application walkthrough combining the renderer, Tauri IPC and an
approved disposable shell endpoint is still pending, as are packaged-runtime CSP,
actual keyboard mappings, real guest/agent interoperability and mobile lifecycle.
The main bundle is about 246 KiB and the lazy terminal about 335 KiB before gzip;
the original large-main-chunk warning is gone. `STATE.md` records actual build
and test evidence; none of these local fixtures closes the real-host Phase 0 gate.

Primary references: [xterm terminal API](https://xtermjs.org/docs/api/terminal/classes/terminal/),
[flow control](https://xtermjs.org/docs/guides/flowcontrol/),
[embedding security](https://xtermjs.org/docs/guides/security/).
