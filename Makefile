build:
	cargo build --release --workspace

build-notify:
	cargo build --release -p tbay-notify --target x86_64-pc-windows-msvc

fmt:
	cargo fmt --all

test:
	cargo test --release

lint:
	cargo clippy --all
