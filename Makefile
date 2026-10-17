.PHONY: doctor install-client dev web server check check-client check-rust check-server check-ssh check-terminal-webkit

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
	cargo run -p pocketlab-server

check: check-client check-rust

check-client:
	npm --prefix apps/client run build
	npm --prefix apps/client test

# Opt-in macOS renderer smoke; simulated IPC, never a user agent or real SSH host.
check-terminal-webkit: check-client
	node experiments/terminal-webkit/check.mjs

check-rust:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace

check-server:
	cargo fmt -p pocketlab-server -- --check
	cargo clippy -p pocketlab-server --all-targets -- -D warnings
	cargo test -p pocketlab-server

# Isolated localhost experiment; intentionally outside the application workspace.
check-ssh:
	cargo fmt --manifest-path experiments/ssh/Cargo.toml -- --check
	cargo clippy --locked --manifest-path experiments/ssh/Cargo.toml --all-targets -- -D warnings
	cargo test --locked --manifest-path experiments/ssh/Cargo.toml
