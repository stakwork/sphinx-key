uniffi-bindgen --version
should match the uniffi version in Cargo.toml

uniffi-bindgen generate src/crypter.udl --language kotlin

uniffi-bindgen generate src/crypter.udl --language swift

### manually build the C ffi

uniffi-bindgen scaffolding src/crypter.udl
