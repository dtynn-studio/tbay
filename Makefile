build:
	cargo build --release --workspace

fmt:
	cargo fmt --all

test:
	cargo test --release

lint:
	cargo clippy --all
