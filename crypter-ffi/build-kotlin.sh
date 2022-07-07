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

echo "=> renaming files"

mv target/aarch64-linux-android/release/libcrypter.a target/aarch64-linux-android/release/aarch64-libcrypter.a
mv target/aarch64-linux-android/release/libcrypter.so target/aarch64-linux-android/release/aarch64-libcrypter.so

mv target/armv7-linux-androideabi/release/libcrypter.a target/armv7-linux-androideabi/release/armv7-libcrypter.a
mv target/armv7-linux-androideabi/release/libcrypter.so target/armv7-linux-androideabi/release/armv7-libcrypter.so

mv target/i686-linux-android/release/libcrypter.a target/i686-linux-android/release/i686-libcrypter.a
mv target/i686-linux-android/release/libcrypter.so target/i686-linux-android/release/i686-libcrypter.so