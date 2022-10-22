all:
	PKG_CONFIG_SYSROOT_DIR="/usr/lib/i386-linux-gnu/" RUSTFLAGS="-C target-feature=-crt-static" cargo build --target i686-unknown-linux-musl
clean:
	cargo clean
