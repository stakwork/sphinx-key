use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
// use vls_protocol_signer;

fn main() {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    let mut parity: secp256k1_sys::types::c_int = 0;

    println!("Hello, world!");
}
