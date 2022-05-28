
install the gcc risv toolchain on macos:

https://github.com/riscv-collab/riscv-gnu-toolchain

```
git clone --recursive https://github.com/riscv/riscv-gnu-toolchain

cd riscv-gnu-toolchain

./configure --prefix=/opt/riscv

sudo make

export PATH=$PATH:/opt/riscv/bin

(add that line above to your ~/.bash_profile to make it stick)
```

OR maybe this is better? 

https://github.com/riscv-software-src/homebrew-riscv

```
brew tap riscv-software-src/riscv

brew install riscv-tools
```

### path

CC=/usr/local/Cellar/riscv-gnu-toolchain/main/bin/riscv64-unknown-elf-gcc cargo build --target=riscv32imc-esp-espidf

CC=/Users/evanfeenstra/code/sphinx-key/sphinx-key/signer/.embuild/espressif/tools/riscv32-esp-elf/esp-2021r2-patch3-8.4.0/riscv32-esp-elf/bin/riscv32-esp-elf-gcc-8.4.0 cargo build --target=riscv32imc-esp-espidf

### point to local dep

```sh
git clone https://github.com/devrandom/rust-secp256k1.git secp256k1

cd secp256k1

checkout 4e745ebe7e4c9cd0a7e9c8d5c42e989522e52f71

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
