# Disposable Linux native lifecycle smoke

This opt-in harness runs the built React UI in a real Tauri/WebKitGTK window,
uses the production Rust control-plane commands over actual IPC, connects to an
actual Axum subprocess, and dispatches only to a generated localhost HTTPS peer.
It never accesses Proxmox, an existing credential or a personal browser/profile.
The UI driver does not replace invoke or return simulated native responses.

Prerequisites are Linux, Python 3, OpenSSL, dbus-run-session, bubblewrap with user
namespace/read-only overlay support, Mesa software rendering, and a verified
package prefix containing `sysroot/usr` with Xvfb and WebKitGTK/runtime helpers.
The current temporary Arch prefix is described in [native prefix evidence](../../docs/native-linux-prefix.md).
No package installation occurs when running the harness.

Run the complete build/check target with the verified prefix:

```sh
make check-lifecycle-native NATIVE_TEST_PREFIX=/path/to/verified-prefix
```

The example embeds built assets and the repository-owned scenario. It registers
production lifecycle/read commands and two fixture-only runtime/report commands;
it is never linked into normal application startup. Its configuration uses a
separate fixture identifier and ephemeral WebView storage. The normal app has no
fixture flag, arbitrary eval endpoint or automation reporting command.

The runner generates a temporary CA/key, HTTPS peer, database, X11 auth cookie,
XDG directories and isolated D-Bus session. It hides existing home directories in
a mount namespace, overlays verified `/usr` dependencies read-only, mounts only
owned temporary paths writable and uses software Mesa rendering. WebKit sandbox,
TLS/hostname verification and X11 cookie authentication stay enabled. Xvfb has no
TCP listener. All child environments are explicit. Child groups, temporary data,
HTTPS listener and display are cleaned up on completion/failure.

Fixture setup saves a definition and registers one association. The scenario then
uses actual UI interactions to check the native bridge, read lifecycle conditions,
review/cancel/confirm start, observe successful task plus running power, acknowledge,
recheck and confirm graceful shutdown, and observe stopped power. The runner
requires exactly one fixed start and one graceful-shutdown provider POST, unchanged
association, no provider errors and token/UPID-free server logs.

This demonstrates only the isolated Linux software-rendered harness. It does not
prove normal packaged application launch, GPU rendering, macOS/mobile behavior,
real Proxmox ACL/mutations, guest readiness or SSH. Firefox resume/stale-result
coverage and native command HTTP tests remain separate evidence.


On 2026-10-18, the complete target passed using the owned prefix
`/tmp/vmanyware-native-deps-61ktnqj7`: frontend build/23 Node tests, example/server
build, actual native UI/IPC start and graceful shutdown, verified TLS and fixture
assertions. D-Bus accessibility/portal warnings remain on this minimal display;
they did not fail the asserted interactions. Initial Xvfb/NVIDIA EGL startup
crashed; explicit software Mesa selected the intended disposable backend. An
initial OpenSSL fixture certificate had CA constraints inappropriate for server
authentication; a generated server-auth end-entity certificate fixed verification
without any trust bypass.
