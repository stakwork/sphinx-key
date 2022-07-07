uniffi-bindgen --version

should match the uniffi version in Cargo.toml

### build the C ffi

uniffi-bindgen scaffolding src/crypter.udl

### kotlin

rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android arm-linux-androideabi 

uniffi-bindgen generate src/crypter.udl --language kotlin

cross build --target i686-linux-android --release
cross build --target aarch64-linux-android --release
cross build --target arm-linux-androideabi --release
cross build --target armv7-linux-androideabi --release
cross build --target x86_64-linux-android --release

### swift

uniffi-bindgen generate src/crypter.udl --language swift

