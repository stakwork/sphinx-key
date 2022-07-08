echo "=> creating C FFI scaffolding"
uniffi-bindgen scaffolding src/crypter.udl

echo "=> creating kotlin bindings"
uniffi-bindgen generate src/crypter.udl --language kotlin

echo "=> renaming uniffi_crypter to crypter"
sed -i '' 's/return "uniffi_crypter"/return "crypter"/' src/uniffi/crypter/crypter.kt

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
mkdir -p target/out/x86
mkdir -p target/out/arm64-v8a
mkdir -p target/out/armeabi
mkdir -p target/out/armeabi-v7a
mkdir -p target/out/x86_64

mv target/i686-linux-android/release/libcrypter.so target/out/x86/libcrypter.so
mv target/aarch64-linux-android/release/libcrypter.so target/out/arm64-v8a/libcrypter.so
mv target/arm-linux-androideabi/release/libcrypter.so target/out/armeabi/libcrypter.so
mv target/armv7-linux-androideabi/release/libcrypter.so target/out/armeabi-v7a/libcrypter.so
mv target/x86_64-linux-android/release/libcrypter.so target/out/x86_64/libcrypter.so

zip -r target/kotlin-libraries.zip target/out

echo "=> done!"
