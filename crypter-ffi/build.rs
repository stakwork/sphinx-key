fn main() {
    uniffi_build::generate_scaffolding("./src/crypter.udl").unwrap();
}