
connect:
	nc -U zap.sock

dev:
	cargo run --bin=zap-server

release:
	cargo run --bin=zap-server --release

test:
	cargo test

fmt:
	cargo fmt

clippy:
	cargo clippy
