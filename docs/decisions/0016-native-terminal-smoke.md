# ADR 0016 — Exercise real Tauri IPC in an isolated Cargo example

Date: 2026-10-17
Status: Local native fixture implementation; evidence is recorded in STATE.md.

Native SSH-manager/real-shell tests and simulated-IPC WebKit rendering leave the
actual Tauri command boundary and bundled asset CSP untested together. Add an
opt-in macOS Cargo example importing the same native command/manager source and
compiled frontend. Use real Tauri IPC and custom-protocol assets with the existing
CSP. Do not add test commands, injected scripts or a runtime test flag to the
normal application.

Reuse the existing disposable-agent/local-SSH fixture and real PTY source rather
than create another credential/process implementation. The internal agent-manager
constructor accepts an optional explicit socket: the product supplies None and
retains late environment discovery; fixtures supply their owned socket. No socket
selection parameter is exposed through the WebView. No new dependency is needed.

Give the example an incognito WebView, separate application identifier and private
temporary host trust. Supply only generated public metadata to an initialization
script that operates the actual UI. Compare first-use fingerprints against that
independently generated value before confirmation. Keep the report/deadline and
owned-resource cleanup local to the example; an external runner checks exit plus
an explicit passing report.

This establishes a local desktop integration gate, not remote guest/OpenSSH peer,
physical keyboard/IME, normal app startup/distribution or mobile compatibility.
See experiments/terminal-native/README.md for boundaries and reproducible command.
