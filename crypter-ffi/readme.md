uniffi-bindgen --version

should match the uniffi version in Cargo.toml

### build the C ffi

uniffi-bindgen scaffolding src/crypter.udl

### kotlin

rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android arm-linux-androideabi 

./build-kotlin.sh

### swift

rustup target add aarch64-apple-ios x86_64-apple-ios 

armv7-apple-ios
armv7s-apple-ios
i386-apple-ios

./build-swift.sh
