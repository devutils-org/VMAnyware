# 0072 — Explicit main-window application command permissions

Status: accepted; Linux native boundary evidence, other-platform acceptance open.

## Problem and decision

The previous `core:default` capability restricted Tauri core APIs to `main`, but
custom application commands remained outside an application ACL. Tauri permits
registered application commands across windows by default unless an application
manifest is declared. This meant the capability did not enforce the intended
boundary for remote-control and SSH commands.

Declare the 68 production commands in `app_commands.rs` and pass that manifest to
`tauri-build`. Grant their generated `allow-*` permissions only through the explicit
local `default` capability for `main`; select only that capability in the app
configuration. No remote origins, wildcard windows or additional WebView grants
are enabled. Generated permission files are build output and ignored by Git.

The handler still exposes the existing functions; arguments, operation confirmation
and target/trust policies remain independently enforced. This is a window boundary,
not protection against compromised trusted code in `main`. The main window retains
all permissions needed for its current product use cases. Do not add remote content
or new WebViews under an authorized window without reviewing inherited authority.

## Owned fixtures

Smoke examples need reporting/host-rotation commands absent from production.
`permissions/fixture.toml` declares their permission, but production grants it to
no window and registers neither command. Only owned terminal/lifecycle example
setup dynamically grants it to its `main` window. No production setup path grants
that permission. Declaring a permission does not grant it.

The two-window `command_acl_smoke` example uses the same generated context and
application ACL with the existing `runtime_platform` command. Actual Linux
WebKit/IPC admits it from `main` and rejects it from `untrusted` with a permission
error. The test runs in an owned Xvfb display with generated Xauthority, temporary
app data and enabled WebKit sandbox; no provider, key store, credentials or VM.
The existing native recreation/restart journey is the affected end-to-end check.
`scripts/check-command-permissions.py` checks drift between the production handler,
manifest and capability grants and rejects unexpected remote/extra grants.

Linux compilation and fixture evidence do not prove Android/iOS/Windows/macOS
runtime parity. Earlier APK snapshots predate this permission change. No fresh
Android runtime claim or change to the pending emulator approval is made.

## References

- [Tauri capabilities: application commands and AppManifest](https://v2.tauri.app/security/capabilities/)
- The locally resolved tauri-build application ACL generator and Tauri runtime
  authority were also inspected to confirm behavior for this dependency lock.
