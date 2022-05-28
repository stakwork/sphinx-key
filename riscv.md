
### point to local dep

```sh
git clone https://github.com/devrandom/rust-secp256k1.git secp256k1

cd secp256k1

# for static precomputation? See vls-signer-stm32 cargo.toml
git checkout 4e745ebe7e4c9cd0a7e9c8d5c42e989522e52f71

# DO THIS: for v0.22.0, this is what latest rust-bitcoin uses
git checkout 50b7c256377494d942826705a1275055e6f93925
# you'll need to change `mod key` to `pub mode key` in lib.rs

cd secp256k1-sys
```

rust-toolchain.toml:
```yaml
[toolchain]
channel = "nightly"
```

.cargo/config.toml
```yaml
[build]
target = "riscv32imc-esp-espidf"

[target.riscv32imc-esp-espidf]
linker = "ldproxy"

rustflags = ["-C", "default-linker-libraries"]

[unstable]

build-std = ["std", "panic_abort"]

[env]
ESP_IDF_VERSION = { value = "branch:release/v4.4" }
```

in build.rs:
```rs
// Actual build
let mut base_config = cc::Build::new();
// add this with your path to embuild gcc:
base_config.compiler(std::path::PathBuf::from(
   "/Users/evanfeenstra/code/sphinx-key/sphinx-key/signer/.embuild/espressif/tools/riscv32-esp-elf/esp-2021r2-patch3-8.4.0/riscv32-esp-elf/bin/riscv32-esp-elf-gcc"
));
```

and use path dep in Cargo.toml
```yaml
secp256k1-sys = { path = "../../secp256k1/secp256k1-sys" }
```

### overrides

validating-lightning-signer
bitcoin
lightning
lightning-invoice

point each of those to `path` dependencies of each other

remove `rand-std` feature from every crate dependency

### issue now:

no `XOnlyPublicKey` in the root

need to align the commits on each crate???

### path notes (dont do this)

CC=/usr/local/Cellar/riscv-gnu-toolchain/main/bin/riscv64-unknown-elf-gcc cargo build --target=riscv32imc-esp-espidf

CC=/Users/evanfeenstra/code/sphinx-key/sphinx-key/signer/.embuild/espressif/tools/riscv32-esp-elf/esp-2021r2-patch3-8.4.0/riscv32-esp-elf/bin/riscv32-esp-elf-gcc-8.4.0 cargo build --target=riscv32imc-esp-espidf
