# ADR 0019 — Bound the initial native terminal to macOS

Date: 2026-10-18
Status: Accepted as an implementation boundary; not a Phase 0 completion claim.

The initial MVP terminal targets macOS only. iOS and Android terminal support is
explicitly deferred until each platform has application build and runtime
proof. Linux and Windows native terminals also remain unsupported. The Linux
control plane and browser build are separate from native terminal support.

Evidence is the real macOS Tauri/IPC/generated-key/disposable-agent/local-shell
smoke in ADR 0016, plus separate strict OpenSSH checks against lab guest 202.
These are complementary observations, not an integrated native-to-guest proof.
Normal application startup, physical keyboard behavior, packaging and that
integrated path still need evidence before release.

The current native authentication path uses a macOS Unix agent socket. Do not
assume that socket access, agent discovery or private-key handling transfers to
mobile application sandboxes. No mobile authentication backend is selected.
No key import, embedded credential or insecure host-key fallback is authorized
by this decision.

Revisit mobile support with an isolated prototype for each OS: record SDK/device
versions; compile SSH crypto; confirm host trust and exact identity selection;
exercise PTY, renderer, keyboard/special keys, resize, background/suspend,
reconnect and socket/process cleanup. Define key access and storage for that OS
before exposing authentication in its UI. A frontend build or upstream framework
support is insufficient evidence.

This explicit limitation permits work on platform-independent definitions,
operation persistence and the loopback service while mobile feasibility remains
open. Snapshot/restore, least-privilege mutation ACLs and remaining desktop
acceptance gates are unaffected. Phase 0 stays open.
