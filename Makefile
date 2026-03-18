build:
	cargo build --release --workspace

build-win:
	cargo build --release --workspace --target x86_64-pc-windows-msvc

fmt:
	cargo fmt --all

test:
	cargo test --release

lint:
	cargo clippy --all
