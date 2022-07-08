echo "=> creating C FFI scaffolding"
uniffi-bindgen scaffolding src/crypter.udl

echo "=> creating swift bindings"
uniffi-bindgen generate src/crypter.udl --language swift

echo "=> creating swift bindings"
sed -i '' 's/module\ crypterFFI/framework\ module\ crypterFFI/' src/crypterFFI.modulemap

echo "=> building x86_64-apple-ios"
cross build --target=x86_64-apple-ios --release
echo "=> building aarch64-apple-ios"
cross build --target=aarch64-apple-ios --release

echo "=> combining into a universal lib"
lipo -create target/x86_64-apple-ios/release/libcrypter.a target/aarch64-apple-ios/release/libcrypter.a -output target/universal-crypter.a

echo "=> done!"
