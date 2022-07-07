echo "=> creating kotlin bindings"
uniffi-bindgen generate src/crypter.udl --language kotlin

echo "=> creating C FFI scaffolding"
uniffi-bindgen scaffolding src/crypter.udl

echo "=> building i686-linux-android"
cross build --target i686-linux-android --release
echo "=> building aarch64-linux-android"
cross build --target aarch64-linux-android --release
echo "=> building armv7-linux-androideabi"
cross build --target armv7-linux-androideabi --release
