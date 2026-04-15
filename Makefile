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

web:
	rm -rf ./target/dx/tbay/release/*
	dx build --release
