
prepare-android:
	rustup toolchain install stable
	rustup target add x86_64-linux-android
	rustup target add x86_64-unknown-linux-gnu
	rustup target add aarch64-linux-android
	rustup target add armv7-linux-androideabi
	rustup target add i686-linux-android
	make install-uniffi-bindgen

install-uniffi-bindgen:
	cargo install uniffi_bindgen --version 0.19.2