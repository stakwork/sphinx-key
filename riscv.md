### buid with CC option

find the path to your `riscv32-esp-elf-gcc` binary withing the `.embuild` dir:

`export CC=/Users/evanfeenstra/code/sphinx-key/sphinx-key/signer/.embuild/espressif/tools/riscv32-esp-elf/esp-2021r2-patch3-8.4.0/riscv32-esp-elf/bin/riscv32-esp-elf-gcc`

### to get std features of vls:

point to local version of validating-lightning-signer:

in validating-lightning-signer/vls-protocol-signer/Cargo.toml `[features]`

add: `vls-std = ["vls-protocol/std"]`

`cargo build`

then in signer Cargo.toml

`vls-protocol-signer = { path = "../../vls-og/vls-protocol-signer", default-features = false, features = ["secp-lowmemory", "vls-std"] }`