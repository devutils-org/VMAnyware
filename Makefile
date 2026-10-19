.PHONY: doctor install-client dev web server check check-client check-rust check-server check-ssh check-terminal-webkit check-terminal-native check-instances-browser check-lifecycle-native check-linux-server check-linux-client

.DEFAULT_GOAL := doctor

doctor:
	node scripts/doctor.mjs

install-client:
	npm --prefix apps/client install

dev:
	npm --prefix apps/client run tauri dev

web:
	npm --prefix apps/client run dev

server:
	cargo run -p vmanyware-server

check: check-client check-rust

check-client:
	npm --prefix apps/client run build
	npm --prefix apps/client test

# Opt-in Linux Firefox check with a disposable profile and simulated native IPC.
check-instances-browser: check-client
	node experiments/instances-browser/check.mjs

# Opt-in Linux real WebView/IPC/control-plane flow, confined to generated HTTPS fixtures.
check-lifecycle-native: check-client
	test -n "$(NATIVE_TEST_PREFIX)"
	PKG_CONFIG_PATH="$(NATIVE_TEST_PREFIX)/sysroot/usr/lib/pkgconfig" LD_LIBRARY_PATH="$(NATIVE_TEST_PREFIX)/sysroot/usr/lib" LIBRARY_PATH="$(NATIVE_TEST_PREFIX)/sysroot/usr/lib" cargo build --locked -p vmanyware-client --example lifecycle_smoke --features tauri/custom-protocol
	cargo build --locked -p vmanyware-server
	python3 experiments/lifecycle-native/check.py --prefix "$(NATIVE_TEST_PREFIX)"

# Opt-in macOS renderer smoke; simulated IPC, never a user agent or real SSH host.
check-terminal-webkit: check-client
	node experiments/terminal-webkit/check.mjs

# Real bundled Tauri/IPC/agent/shell fixture. macOS GUI session required.
check-terminal-native: check-client
	cargo build --locked --release -p vmanyware-client --example terminal_smoke --features tauri/custom-protocol
	node experiments/terminal-native/check.mjs

check-rust:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace

# Opt-in local Docker Linux arm64 check of committed source, with no credentials.
check-linux-server:
	node scripts/check-linux.mjs server

check-linux-client:
	node scripts/check-linux.mjs client

check-server:
	cargo fmt -p vmanyware-server -- --check
	cargo clippy -p vmanyware-server --all-targets -- -D warnings
	cargo test -p vmanyware-server

# Isolated localhost experiment; intentionally outside the application workspace.
check-ssh:
	cargo fmt --manifest-path experiments/ssh/Cargo.toml -- --check
	cargo clippy --locked --manifest-path experiments/ssh/Cargo.toml --all-targets -- -D warnings
	cargo test --locked --manifest-path experiments/ssh/Cargo.toml
