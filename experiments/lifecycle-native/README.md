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


## Deletion and restart recovery

After building the embedded example with the verified native prefix, run:

```sh
python3 experiments/lifecycle-native/check.py --prefix .lab/native-deps --deletion-recovery
```

This mode uses the actual deletion UI/production native commands and authenticated
server worker. It checks running-target refusal, cancel/re-review, independent
consent, a lost accepted API response and identical path/key/body retry. Exactly
one fixed provider DELETE is allowed, without job purge or bulk unreferenced-disk
removal. The journal must contain a succeeded destroy task with VM/volume absence
and retirement evidence. The deleted association and unchanged definition remain.

The runner then restarts both app and server. Provider configuration is removed
and all five write flags are false. The new app starts without restored credentials,
operation ID or consent; the driver explicitly connects using generated fixture
configuration. History, copied-ID lookup and refresh use only GETs, leave provider
counters unchanged and return the unchanged operation. TLS, WebKit sandbox and
X11 cookie authentication remain enabled. All owned processes are cleaned up.
Evidence: `.lab/native-deps/deletion-recovery-ui-check.log`.

This is generated-infrastructure Linux proof. Real Proxmox deletion/ACL/storage
behavior, packaged startup, fresh mobile runs and live provider crash recovery
remain separate acceptance gates. The affected restore/recovery mode also passes
with the shared launcher changes.

## Retained-definition recreation

Build the current frontend/server and embedded example with the verified prefix,
then run:

```sh
python3 experiments/lifecycle-native/check.py --prefix .lab/native-deps --recreation
```

This mode reuses the generated HTTPS provider from the Android creation fixture
through an owned Node adapter. Node is an additional prerequisite for this mode;
no emulator or real Proxmox is required. The runner saves a minimal `mobile-lab`
definition (2 CPUs/2048 MiB/8 GiB, no packages), registers an association and retires
that association in the isolated SQLite fixture. It does not perform deletion.
The current catalog includes an explicit synthetic image-default-user attestation.
A public demonstration SSH key is used; no private key or personal identity is read.

The actual native React/WebKit UI selects the retired source, reviews target VMID
202, cancels/reviews again with consent cleared, and explicitly confirms fresh
provisioning. The verified HTTPS proxy loses only the first accepted response.
Both API submissions must retain the identical path/key/body. Production native
commands, authenticated Axum server process and its production worker then run
clone, resource/Cloud-Init configuration, seed regeneration, start and synthetic
Cloud-Init/network readiness. The driver reads status and prepares the existing
SSH host-trust form for `debian@192.0.2.42`; it does not establish SSH or accept a key.

Independent runner assertions require exactly six provider mutations (one clone,
two configuration writes, one seed regeneration, one start and one guest-status
exec), one succeeded provisioning operation, all stages succeeded and exactly one
new distinct registered instance. Source/YAML/hash, current definition and durable
source lineage must remain intact. Provider assertions check settings/public key,
resource values, stage order and no duplicate clone/seed/start. Reports/owned
processes/listeners are closed, then the server shuts down cleanly. Logs:
`.lab/native-deps/recreation-ui-check.log`. The shared deletion/restart mode is
selected to verify the launcher changes.

This proves only isolated Linux native integration with generated infrastructure.
It does not prove normal installed startup, real template/ACL/guest/SSH behavior,
fresh mobile recreation, live provider crash recovery or a combined real delete-
recreate journey. Separate storage/worker tests cover database reopen; native
saved-source history/restart recovery remains open.

## Recreation app/server restart recovery

```sh
python3 experiments/lifecycle-native/check.py --prefix .lab/native-deps --recreation-recovery
```

This combined mode first runs the complete actual native recreation journey,
including cancelled review, fresh consent, lost accepted HTTP reply and identical
API retry with six expected provider mutations. It records the succeeded operation,
retired source and new target, then cleanly restarts both server and application.
All five write flags are false; provider variables and the runtime template catalog
are removed. New native profile/consent state starts with no token or saved ID.

The second UI process explicitly connects using only generated fixture TLS/auth
configuration, loads source history, selects the saved operation, enters the copied
ID and refreshes the saved result. The panel must describe historical evidence,
without restoring a recreation form/checkbox or exposing direct SSH from an old
result. Recovery requests through the authenticated HTTPS proxy must be GET only.
The generated provider stays alive; its complete request/error snapshot must be
unchanged after recovery, as must the complete operation and source/target records.
The final provider report still requires exactly six mutation calls and closes the
owned listener/process. No actual VM or emulator is created in this fixture.

The embedded example/server builds and combined mode pass. Shared deletion/restart
is selected because the launcher/restart branch changed. Evidence:
`.lab/native-deps/recreation-recovery-ui-check.log`. This is actual isolated Linux
WebView/IPC/process/HTTPS evidence, not a live-provider crash, deployed/packaged
startup, updated Android APK acceptance or real guest/SSH proof.

## Remote lifecycle through the retained HTTPS profile

```sh
python3 experiments/lifecycle-native/check.py --prefix .lab/native-deps --remote-lifecycle
```

Build the existing embedded lifecycle_smoke example first using the verified native
prefix and `--features tauri/custom-protocol`. This mode launches actual React/
WebKit/native IPC, authenticated Axum and its production worker against the owned
generated HTTPS provider. The remote workspace reviews/cancels/reviews and confirms
start, then graceful shutdown. The HTTPS proxy loses the first accepted API reply
for each action; native/UI must explicitly retry the same key and approval. Exact
request pairs match, distinct actions have distinct keys, known IDs are read by GET,
and only one provider start plus one provider shutdown is observed. Both durable
operations must show matching target/scope, successful task and expected power state;
instance identity stays unchanged. Power success never claims SSH/guest readiness.

No IPC simulation, actual VM, installed host service or personal credential is used.
Generated certificates/tokens/state, sandboxed Xvfb and processes are owned and
cleaned up. Log: `.lab/native-deps/remote-lifecycle-ui-check.log`. Android, real guest,
normal installed startup and saved lifecycle recovery after losing native state
remain separate acceptance gates.

## Lifecycle app/server restart recovery

```sh
python3 experiments/lifecycle-native/check.py --prefix .lab/native-deps --lifecycle-recovery
```

The first owned native process completes remote start/shutdown with the accepted-
reply-loss/same-key retry scenario. The runner snapshots both full saved operations,
instance/definition and provider counters, stops the actual server, then opens the
same DB in a new server process/port with all five write flags false and provider/
catalog configuration absent. A second actual native app begins without token,
operation ID or restored intent, connects explicitly and reads history, both saved
actions, copied ID and refresh through production HTTPS commands and React controls.
The proxy requires GET-only recovery traffic; both full operations, retained
instance/definition and provider read/task/power/write counters remain unchanged.
Historical start power is explicitly distinct from current state. No consent or
SSH action is restored. All owned processes and temporary state are cleaned up.
This proves clean isolated Linux restart recovery, not a crash, real VM or Android
runtime. Log: `.lab/native-deps/lifecycle-recovery-ui-check.log`.

The remote lifecycle driver now also exercises the explicit power panel. It reads
stopped before any action, running after acknowledged start, and stopped after
acknowledged shutdown. Each native read traverses verified HTTPS to the registered
instance status route. The relay verifies target/scope and the exact observed
sequence; the provider still receives only the original one start/one shutdown.
The UI must show a read timestamp/staleness/no-readiness warning and invalidate its
old power result during action review. --lifecycle-recovery additionally verifies
that mounting this panel in the new app does not introduce automatic provider reads.
Logs: `.lab/native-deps/remote-power-native-ui-check.log` and
`.lab/native-deps/remote-power-restart-check.log` (see STATE.md for completed runs).

## Retained definition in real native recovery contexts

Both --lifecycle-recovery and --recreation-recovery now inspect saved definition
panels in the second app, after the actual server restarts without provider/catalog
and with all writes false. The shared instance-detail.js helper targets the exact
association ID (archived and new associations may reuse the same VMID), explicitly
reads/refreshes and compares literal YAML text, SHA-256, revision/name and saved
status to the fixture's original stored record, then hides it. No edit/consent/SSH
control follows. Existing full journal/identity/provider-counter invariants still
apply, establishing that detail inspection adds no provider reads or mutation.
The example alone includes this acceptance driver; normal application builds do not.
Logs: `.lab/native-deps/instance-detail-active-ui-check.log` and
`.lab/native-deps/instance-detail-archived-ui-check.log`. These are isolated Linux
native/server checks, not Android or real VM validation.

## Existing-instance manual native SSH host trust

```sh
python3 experiments/lifecycle-native/check.py --prefix .lab/native-deps --manual-ssh
```

The owned example embeds the real remote React entry and production native host,
application-identity and session commands. An in-process loopback SSH peer, fresh
passphrase-encrypted application identity and temporary trust directory are created
for this run. Only its generated public key is accepted, for account `fixture`.
The agent manager has an explicit nonexistent owned socket path, preventing personal
agent discovery; Linux agent support remains false.

The UI selects the registered row, begins with blank address/account, enters the
explicit endpoint, confirms the independently supplied fingerprint and reinspects.
It explicitly unlocks the connection identity, selects its public key and opens the
real PTY/OS shell. xterm input executes a command whose stdout must render, then
`stty size` must match the resized renderer. Terminal disconnect closes the peer
connection. A subsequent owned host-key rotation is refused without replacing the
saved trust bytes. Peer counters require exactly one session channel and recorded
PTY, shell and resize requests. Temporary peer, shell, identity and trust are removed.
Normal application builds include no driver, rotation command or owned peer.

The control-plane side is real authenticated HTTPS, with zero Proxmox provider
reads/tasks/writes. This is Linux native evidence, not real VM endpoint ownership
or Android execution. Log: `.lab/native-deps/manual-ssh-terminal-check.log`.


## Declared packages and final snapshot through actual native IPC

```sh
python3 experiments/lifecycle-native/check.py --prefix .lab/native-deps --declared-create
python3 experiments/lifecycle-native/check.py --prefix .lab/native-deps --declared-recreation
```

Both modes use the real React/WebKit client, retained production native commands,
verified HTTPS/bearer control plane, production sequential worker and an owned
in-memory HTTPS provider. The creation mode uses the explicit fields and unchecked
confirmation; the recreation mode reviews the retained declaration. Both request
build-essential and pinned git, followed by a disk-only final snapshot.

The provider requires Cloud-Init observation before package dispatch, package
completion observation before the final snapshot, exact positional declarations
and operation/definition/plan-bound snapshot metadata. A first accepted API reply
is deliberately lost; the two confirmed API requests must retain identical path,
key and body. The provider must record exactly eight expected non-GET requests,
including one package dispatch and one final snapshot. SQLite evidence requires
all stages succeeded, matching scoped package result, final disk-only snapshot and
unchanged definition/retired source. Creation has no recreation lineage; recreation
links the exact retired source and a distinct new instance. SSH handoff remains
explicit with unchanged independent host trust; these modes do not log into a guest.

The generated provider returns controlled guest PID/exit/status data; no apt command
or VM actually executes. These are Linux native integration results, not real
Proxmox/guest installation or current Android acceptance. Shared provider behavior
remains unchanged when declared=false; --recreation-recovery also passes after this
wiring, including GET-only recovery without provider/catalog/write flags.
Logs: `.lab/native-deps/declared-create-native-check.log`,
`.lab/native-deps/declared-recreation-native-check.log` and
`.lab/native-deps/declared-workflow-default-recovery-check.log`.

Declared create/recreation also explicitly read recorded stage progress through
the production native command and authenticated API after completion. They require
succeeded package/final-snapshot stages and saved-evidence wording before the
independent SSH handoff. The extra calls are GET-only and must not change the
expected provider mutations. The default uses current debug server/example binaries. To exercise an extracted
owned release archive, pass `--server-binary /path/to/extracted/bin/vmanyware-server`.
This does not install a host service or execute an Android APK.

## Owned Linux system key store and actual native SSH

```sh
# Build the example with the verified prefix and tauri/custom-protocol first.
python3 experiments/lifecycle-native/check.py --prefix .lab/native-deps --system-identity
```

This opts into the existing-instance manual SSH journey with a real Secret Service.
The owned bus launcher starts the verified-prefix GNOME Keyring daemon in a private
D-Bus session and temporary XDG directories. It waits for bus-name ownership without
activating another service. Native mode requires the fixture root marker and exact
owned XDG paths, stops/reaps the daemon on completion and never connects to a
personal desktop bus, keyring or SSH agent.

The real React driver uses unchecked save consent, saves the generated identity's
passphrase through production IPC, clears the input and explicitly unlocks through
the system key store. It independently reviews the SSH host key, switches to the
system identity source, submits no passphrase in SSH-selection IPC, grants fresh
connection consent, selects the key and opens the real owned PTY/shell. The existing
stdout/resize/disconnect/changed-host assertions still apply. Explicit forget must
clear its consent and a subsequent UI read must fail; a native final check requires
the exact missing-secret error before deleting the generated identity. A distinct
native completion marker prevents an old normal-SSH binary from satisfying this mode.
This is an opt-in owned fixture, not personal credential or real VM authorization.
