.PHONY: run bindgen-bin bindgen

run:
	cargo run --bin cli

bindgen-bin:
	cargo run --features=uniffi --bin uniffi-bindgen

bindgen:
	cargo build --release --features=uniffi
	cargo run --features=uniffi --bin uniffi-bindgen generate --library target/release/libseycore.dylib --language swift --out-dir out
