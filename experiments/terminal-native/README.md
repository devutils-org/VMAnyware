# Real macOS Tauri terminal smoke

Run `make check-terminal-native` from a macOS GUI session with the repository
Rust/Node dependencies and the system ssh-agent supporting `-U`. This is opt-in:
it compiles an optimized Cargo example and opens its own disposable test window.
Default `make check` compiles/checks example source but never runs GUI automation.

Unlike the isolated WebKit smoke, this test does not mock native IPC. It uses:

- The compiled production frontend, including its lazy xterm module/CSS.
- Tauri's actual WebView, custom application asset protocol and configured CSP.
- The same SSH command/manager source as the product (imported by path).
- A real cleared-environment `/usr/bin/ssh-agent -D -U -a` with generated keys.
- A generated-key localhost SSH peer connected to an actual disposable /bin/sh PTY.

A private temporary host-trust directory and explicit fixture socket are supplied
through Rust construction. The example has a separate application identifier and
an incognito WebView; no personal app trust store, SSH config/key file, existing
agent, browser profile, Proxmox endpoint or VM is accessed. The runner clears the
child environment. Generated private keys stay in memory and the fixture agent.
Only generated public fingerprints/endpoint metadata reach the scenario script.

The example reads its fixed repository-owned scenario script (no runtime script
path parameter) and supplies it as an initialization script. It automates actual
form controls. It compares the
inspected host fingerprint with the independently generated fixture value,
explicitly confirms it, inspects again, lists and selects the generated identity,
opens the terminal and observes shell stdout rendered by xterm. It resizes the
container and requires actual stty row count to match the renderer, sends Ctrl-C
to foreground sleep and requires rendered status 130 in under two seconds, then
disconnects/reconnects. Rust independently checks two authenticated connections,
resize observations, closed SSH connections and reaped shell fixtures. The
example then rotates only the generated localhost host key: the UI must reject
it with no overwrite/identity action. Rust requires no additional authentication
and byte-identical temporary trust data before/after rotation.

The example-only report command and scenario are never registered/embedded in
the normal application. No runtime test flag, TLS/host-key bypass, generic signing
or fixture HTTP endpoint is added to the product. The normal constructor still
resolves the configured agent only after an explicit action/verified handshake.

Before exit the fixture closes sessions/listener/shells, terminates/reaps its own
agent and removes private temporary directories. A 45-second internal deadline
uses the same cleanup. The runner requires both exit zero and an explicit passing
report; closing the window alone cannot fabricate success. It has a 60-second
last-resort process deadline. The test uses no global kill or personal agent.

Evidence remains bounded: the peer is a local russh fixture, not a real guest or
OpenSSH server. Input events are synthetic, not physical keyboard/IME tests. The
example imports the real commands and frontend but does not exercise normal app
startup configuration, distribution signing/installers, the user's agent or mobile
platforms. Keep those gates open in STATE.md and the roadmap.

The workspace navigation assertion retains the terminal element across four
anchor destinations, checks destination focus, then requires fresh stdout from
the same real shell before continuing resize/Ctrl-C/disconnect checks. The
scenario scrolls the terminal back into view before inspecting fresh stdout:
xterm defers rendering while offscreen, so hidden DOM rows are not evidence of
a stalled shell. This assertion passed on macOS arm64 after correcting that
visibility precondition (see STATE.md). Failures include the scenario assertion
in stderr, in addition to the step and Rust cleanup checks. No simulated-IPC
browser result substitutes for this native run.
