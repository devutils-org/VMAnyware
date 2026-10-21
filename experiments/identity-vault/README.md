# Owned Linux Secret Service acceptance

This opt-in runner creates a new private D-Bus session and temporary XDG
config/data/cache/runtime directories per test. It starts an owned verified-prefix
`gnome-keyring-daemon` with a disposable test password via stdin, runs exactly one
ignored Rust case per session, stops its daemon and removes the temporary state.
It never connects to a personal session bus or reads an existing keyring.

Prerequisites: Linux, Python 3, dbus-run-session, gdbus and a verified package prefix
containing the daemon/runtime libraries. The current local prefix has signed Arch
gcr 3.41.2-2 and gnome-keyring 1:51.1-1. Signature/URL/hash evidence is in ignored
`.lab/native-deps/vault-packages.json`; no install hooks or host package changes ran.

Build the affected test executable with the prefix variables documented in
[the Linux prefix guide](../../docs/native-linux-prefix.md), then pass its exact
path from Cargo's `Running unittests` output:

```sh
cargo test --locked -p vmanyware-client ssh:: --lib
python3 experiments/identity-vault/check.py \
  --prefix /absolute/verified-prefix \
  --test-binary /absolute/path/to/target/debug/deps/vmanyware_client_lib-HASH
```

The default Cargo invocation skips the two vault cases and never accesses a vault.
The runner refuses zero-case success and isolates the roundtrip/locked-store case
from the SSH case. It proves real Secret Service save/read/forget and native SSH
public-key authentication with generated owned keys, not WebView IPC, a packaged
client, another desktop service, a hardware key or another OS. The unit-test guard
is an additional misuse check; the runner's explicit child environment/private bus
and temporary directories provide the isolation.
