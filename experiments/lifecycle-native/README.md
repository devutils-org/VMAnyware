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

## Remote snapshot-list scenario

The same isolated example/runner also supports a read-only remote snapshot-list
journey. With the verified prefix available, build only the affected native
example and server, then run:

```sh
export PKG_CONFIG_PATH="$PWD/.lab/native-deps/sysroot/usr/lib/pkgconfig"
export LIBRARY_PATH="$PWD/.lab/native-deps/sysroot/usr/lib"
export LD_LIBRARY_PATH="$PWD/.lab/native-deps/sysroot/usr/lib"
cargo build --locked -p vmanyware-client --example lifecycle_smoke --features tauri/custom-protocol
cargo build --locked -p vmanyware-server --bin vmanyware-server
python3 experiments/lifecycle-native/check.py --prefix .lab/native-deps --snapshots
```

Use `tauri/custom-protocol` to embed the already-built frontend; a plain development
build waits for Vite and cannot run this self-contained scenario. Rebuild the
frontend only if its source changed. The prefix must include Xvfb/xauth in addition
to the WebKitGTK libraries.

This mode generates a token/certificate, starts an actual authenticated Axum
server and a verified localhost HTTPS reverse proxy, then uses real React controls
and production remote native commands. The server reads a generated HTTPS
provider: first only the synthetic current entry, then incomplete snapshot
metadata, then a permission failure. Assertions cover target display, empty/error
separation, unknown metadata, escaped descriptions, backup wording and disconnect.
The runner requires exactly three provider reads and zero provider writes;
lifecycle execution stays disabled. Fixture setup registers one disposable local
association before the UI starts. Neither invoke nor server responses are mocked
in the WebView, and no real Proxmox or personal credential is used.

## Confirmed remote snapshot creation

With the current frontend assets and native example/server built as above, run:

```sh
python3 experiments/lifecycle-native/check.py --prefix .lab/native-deps --snapshot-create
```

This separate mode enables snapshot execution only in the owned server and
forwards read-only preflight plus the confirmed request through its generated
verified HTTPS proxy. The real WebView driver reviews/cancels, reviews again with
fresh unchecked consent, confirms, refreshes the retained operation through GET,
independently lists the created snapshot, acknowledges and disconnects. Native
production commands retain and validate the actual review/request/operation.

The generated provider accepts exactly one disk-only POST with fixed target,
name and Unicode/special-character description. Runner checks require five
snapshot reads, at least one independent task poll, unchanged instance association
and exactly one succeeded journal operation with matching task-backed complete
disk-only metadata. No real Proxmox, guest, existing credential or personal browser
is used. Recorded evidence: `.lab/native-deps/snapshot-create-ui-check.log`.

The two snapshot modes are mutually exclusive. Default lifecycle behavior stays
separate. This demonstrates isolated Linux native integration, not packaged or
mobile startup, real-provider snapshots, guest consistency or restoration.

## Recovery after actual app and server restart

Run the distinct combined mode with the same built assets/example/server:

```sh
python3 experiments/lifecycle-native/check.py --prefix .lab/native-deps --snapshot-recovery
```

It first runs the confirmed creation scenario once. After that application exits,
the runner records the succeeded operation, stops the server cleanly, reopens the
same SQLite database in a new server process with snapshots disabled and every
Proxmox configuration variable removed, then starts a second native application
process. A fresh connection supplies the generated HTTPS/token/CA explicitly;
the form must not restore the old token, operation ID or creation intent.

The second driver finds the saved operation through instance history, reads the
copied ID and refreshes it, with historical/current-state wording and no consent
control. The proxy must see only GETs in this phase. Provider read counts and the
single POST count must stay unchanged, and the entire saved operation must match
the pre-restart record. The separate provider remains alive only to detect any
unexpected access; the restarted server has no provider credentials/configuration.

Evidence: `.lab/native-deps/snapshot-recovery-ui-check.log`. This verifies a
succeeded saved operation in the isolated Linux fixture, not a crash during an
in-flight provider task, normal installed startup, mobile or real infrastructure.
Existing journal/native unit tests cover uncertain/running records separately.

## Destructive restore with a lost submission response

With the existing built frontend, build the current server and embedded-asset
example as above, then run:

```sh
python3 experiments/lifecycle-native/check.py --prefix .lab/native-deps --snapshot-restore
```

This separate mode uses actual React/WebKit, production retained native restore
commands, verified authenticated HTTPS, the actual API/worker and an owned HTTPS
provider. A generated pre-existing disk-only snapshot/configuration belongs to the
registered disposable fixture instance. The first read-only review observes a
running VM and must refuse consent. Later fixture reads observe it stopped;
no provider power write is implemented. An eligible review is cancelled and
prepared again, with two independent unchecked confirmations each time.

The HTTPS proxy deliberately loses the first submission response only after the
real API has returned 202 and stored the approval. The native UI must retain its
key/consent and retry the exact same path, key and body. The provider accepts only
one fixed rollback POST with start=0. The runner verifies two identical API
submissions, one succeeded restore journal record, a retained successful task,
exact instance/scope/node/VMID/name, a separately computed canonical selected-config
fingerprint and independently observed stopped power. It also requires 12 snapshot
list GETs, 10 saved-config reads, 10 power reads and at least one task poll across
review/revalidation/completion. The live digest changes only after rollback while
the selected-config fingerprint stays stable. Final listing/status reads and
terminal acknowledgement/disconnect must work; instance association stays equal.

Evidence: `.lab/native-deps/snapshot-restore-ui-check.log`. Initial driver-only
failure used an input setter for the CA textarea; fixed its element dispatch and
rebuilt only the example before rerunning this scenario. The earlier failed log
is retained as `.lab/native-deps/snapshot-restore-ui-check.initial.log`.
TLS, X11 cookies and the WebKit sandbox remain enabled. Owned processes/listeners/
temporary data are cleaned up; minimal-session portal/accessibility warnings remain.
This is synthetic provider completion, not actual storage rollback, guest-content
readback, normal installed/mobile launch or restart recovery of a restore journal.
Snapshot mode flags are mutually exclusive.

## Restore recovery after actual app/server restart

```sh
python3 experiments/lifecycle-native/check.py --prefix .lab/native-deps --restore-recovery
```

This mutually exclusive combined mode first runs the real native restore scenario,
including running-target refusal, cancelled review, independent consent and exact
same-key retry after a deliberately lost API response. It still requires one
provider rollback POST and task/selected-config/stopped-target evidence.

After the first application exits, the runner saves the succeeded operation,
cleanly stops the actual server and starts a new server process on the same
SQLite database. All four write flags are explicitly false and every Proxmox
configuration variable is removed. A second native application process starts
with a fresh WebView/profile/consent state and supplied generated HTTPS/token/CA.
The recovery driver explicitly reads history, inspects the saved row, reads the
copied operation ID and refreshes it without preparing a new restore.

The proxy must see only GETs during recovery. Provider read counts and the single
rollback POST count must remain unchanged, and the complete saved operation must
equal its pre-restart record. Both app processes must exit successfully and the
server must shut down cleanly. Evidence:
`.lab/native-deps/restore-recovery-ui-check.log`. Current embedded example and
server builds plus the combined scenario pass. No product dependency/schema or
source change was required; no unrelated suites were repeated.

This verifies a succeeded restore record across clean app/server restarts in the
isolated Linux fixture. It does not prove crash recovery during a live provider
rollback, current VM/snapshot state, real disk/guest contents, mobile/installed
startup or any real infrastructure mutation. Separate journal/native tests cover
queued/running/uncertain records. Generated listeners/processes and temporary
state are cleaned up; TLS, sandbox and display-cookie checks remain enabled.
