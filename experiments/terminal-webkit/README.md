# Isolated macOS WebKit terminal smoke

Run `make check-terminal-webkit` on macOS with the repository Node dependencies
installed and Apple CommandLineTools (`swiftc`/Cocoa/WebKit) available. This is
opt-in because it opens an owned fixture window behind other windows and uses
platform GUI facilities; it is not part of the default cross-platform check.

The Node harness compiles a small host in a private temporary directory, serves
the client in Vite development mode on an ephemeral loopback port and loads it
in a WKWebView with a nonpersistent data store. It injects **simulated** Tauri IPC
before any application JavaScript. No Rust SSH command, existing agent, personal
browser profile, key file or real SSH/Proxmox endpoint is accessed. The fake
endpoint/fingerprints exist only in this test source and never authorize trust
in the actual application. No fixture is included in the application build.

The UI exercise checks no automatic agent listing, explicit identity choice,
terminal rendering of UTF-8 split across reads, resize propagation, synthetic Unicode textarea input, byte 3 for
Ctrl-C, disconnect and cleanup of a native ID returned after the UI cancelled a
pending start. Vite development mode exercises React StrictMode's effect replay;
there must be no duplicate start or premature close. Results report through the
owned WebKit host. A thirty-second page deadline and child deadlines fail closed;
the local server, temporary binary/directory and fixture window close on exit.

This establishes renderer/UI behavior on the local macOS WebKit engine with fake
IPC. It does not establish real Tauri IPC integration, packaged CSP behavior,
hardware keyboard mappings, a native-to-real-shell round trip, a real guest or
mobile compatibility. Native Rust protocol tests and the isolated actual-shell
fixture provide their own separate evidence. See docs/ssh-terminal.md.
