.PHONY: doctor install-client dev web server check check-client check-rust check-server

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

check-rust:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace

check-server:
	cargo fmt -p pocketlab-server -- --check
	cargo clippy -p pocketlab-server --all-targets -- -D warnings
	cargo test -p pocketlab-server
