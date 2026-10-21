# Native command window boundary

This opt-in Linux check runs two actual WebKit windows against the production
application ACL. `main` must receive the `runtime_platform` response; `untrusted`
must receive a permission refusal. A deadline, unexpected success or unrelated
IPC failure fails the check. It uses an owned temporary X display/app profile,
generated Xauthority and the verified native dependency prefix. WebKit sandboxing
stays enabled. No VM, provider, SSH identity or personal credential is accessed.

From the repository with the owned dependency prefix prepared:

```sh
python3 scripts/check-command-permissions.py
PKG_CONFIG_PATH="$PWD/.lab/native-deps/sysroot/usr/lib/pkgconfig" \
LIBRARY_PATH="$PWD/.lab/native-deps/sysroot/usr/lib" \
LD_LIBRARY_PATH="$PWD/.lab/native-deps/sysroot/usr/lib" \
cargo build --locked -p vmanyware-client --example command_acl_smoke --features tauri/custom-protocol
python3 experiments/command-acl-native/check.py --prefix .lab/native-deps
```

The embedded frontend must already exist. This fixture proves Linux native IPC
window admission, not Android runtime, every command's behavior or security of
compromised code in the authorized main window. ADR 0072 records the policy.
