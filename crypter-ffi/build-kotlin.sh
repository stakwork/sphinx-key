echo "=> creating kotlin bindings"
uniffi-bindgen generate src/crypter.udl --language kotlin

echo "=> creating C FFI scaffolding"
uniffi-bindgen scaffolding src/crypter.udl

echo "=> building i686-linux-android"
cross build --target i686-linux-android --release
echo "=> building aarch64-linux-android"
cross build --target aarch64-linux-android --release
echo "=> building arm-linux-androideabi"
cross build --target arm-linux-androideabi --release
echo "=> building armv7-linux-androideabi"
cross build --target armv7-linux-androideabi --release
echo "=> building x86_64-linux-android"
cross build --target x86_64-linux-android --release

echo "=> renaming files"

mkdir -p target/out

mv target/i686-linux-android/release/libcrypter.so target/out/i686-libcrypter.so

mv target/aarch64-linux-android/release/libcrypter.so target/out/aarch64-libcrypter.so

mv target/arm-linux-androideabi/release/libcrypter.so target/out/arm-libcrypter.so

mv target/armv7-linux-androideabi/release/libcrypter.so target/out/armv7-libcrypter.so

mv target/x86_64-linux-android/release/libcrypter.so target/out/x86_64-libcrypter.so

